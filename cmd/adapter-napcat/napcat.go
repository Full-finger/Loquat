// NapCat 适配器：go-plugin 插件子进程。
// 职责：维持与 NapCat 的 WebSocket 连接，OneBot 11 编解码，
// 把事件上报给核心，替核心调用 NapCat 的 action。
// 注意：stdout 被 go-plugin 握手占用，所有日志必须走 stderr。
package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"os"
	"sync"
	"sync/atomic"
	"time"

	"github.com/gorilla/websocket"
	"github.com/hashicorp/go-hclog"
	"google.golang.org/grpc"

	"github.com/full-finger/loquat/internal/pluginapi"
)

const (
	eventsBuffer   = 256              // 事件缓冲上限，满了丢弃
	defaultTimeout = 10 * time.Second // SendAction 默认超时
	reconnectDelay = 3 * time.Second  // MVP 固定重连间隔，第 4 步升级指数退避
	confirmTimeout = 5 * time.Second  // 等首条消息确认鉴权的上限
	writeTimeout   = 10 * time.Second // WS 写上限：对端半死时不能无限阻塞（26102202#3）
)

type NapCat struct {
	pluginapi.UnimplementedAdapterServer

	url    string // 形如 ws://127.0.0.1:3001，不带查询参数
	token  string
	logger hclog.Logger

	mu            sync.Mutex      // 保护 conn 的写与置空
	conn          *websocket.Conn // 当前连接，断开后置 nil
	writeDeadline time.Duration   // 单次写上限，测试可缩短
	pmu           sync.Mutex      // 保护 pending
	pending       map[string]chan *pluginapi.SendActionResponse
	seq           atomic.Int64
	dropped       atomic.Int64 // 事件队列满丢弃计数（本进程口径，日志观测用）

	events chan *pluginapi.Event
}

func newNapCat(url, token string, logger hclog.Logger) *NapCat {
	return &NapCat{
		url:           url,
		token:         token,
		logger:        logger,
		pending:       make(map[string]chan *pluginapi.SendActionResponse),
		events:        make(chan *pluginapi.Event, eventsBuffer),
		writeDeadline: writeTimeout,
	}
}

// connectLoop 维持与 NapCat 的长连接：断开就重连。该函数永不返回。
func (n *NapCat) connectLoop() {
	// 鉴权走 Authorization 头而非 URL query，避免 token 落进日志（清单 #8）
	header := http.Header{}
	if n.token != "" {
		header.Set("Authorization", "Bearer "+n.token)
	}
	for {
		conn, _, err := websocket.DefaultDialer.Dial(n.url, header)
		if err != nil {
			n.logger.Error("连接 NapCat 失败，3s 后重试", "err", err)
			time.Sleep(reconnectDelay)
			continue
		}
		n.mu.Lock()
		n.conn = conn
		n.mu.Unlock()

		// 这版 NapCat 先完成 WS 升级再验 token：token 错时它回一条
		// retcode=1403 失败帧再断开，Dial 成功不代表可用，先等首条消息确认。
		if !n.confirm(conn) {
			n.mu.Lock()
			if n.conn == conn {
				n.conn = nil
			}
			n.mu.Unlock()
			_ = conn.Close()
			n.failPending(errors.New("NapCat 连接未通过确认，动作未完成"))
			n.logger.Error("NapCat 连接不可用，3s 后重试")
			time.Sleep(reconnectDelay)
			continue
		}
		n.logger.Info("已连接 NapCat", "url", n.url)

		n.readLoop(conn) // 阻塞直到连接断开

		n.failPending(errors.New("ws 断开，动作未完成"))
		n.logger.Info("与 NapCat 断开，3s 后重连")
		time.Sleep(reconnectDelay)
	}
}

// confirm 等首条消息确认鉴权：收到鉴权失败帧或连接提前断开返回 false；
// 超时无消息的连接按已连接处理（可能只是不发事件），返回 true 并告警。
// 读到的合法首条帧交给 dispatch，不丢。
func (n *NapCat) confirm(conn *websocket.Conn) bool {
	_ = conn.SetReadDeadline(time.Now().Add(confirmTimeout))
	_, data, err := conn.ReadMessage()
	_ = conn.SetReadDeadline(time.Time{}) // 后续读取交给 readLoop，不再限时
	switch {
	case err == nil:
		if authFailed(data) {
			n.logger.Error("NapCat 鉴权失败，请检查 token", "reply", head(data, 120))
			return false
		}
		n.dispatch(data)
		return true
	case errors.Is(err, os.ErrDeadlineExceeded):
		n.logger.Warn("NapCat 连接后超时无消息，无法确认鉴权，按已连接处理")
		return true
	default:
		n.logger.Error("NapCat 连接未确认即断开", "err", err)
		return false
	}
}

// authFailed 识别 NapCat 的鉴权失败帧：status=failed 且无 echo。
// 动作响应都带 echo，没有 echo 的失败帧只能是连接层的拒绝（如 token 错）。
func authFailed(data []byte) bool {
	var probe struct {
		Status string `json:"status"`
		Echo   string `json:"echo"`
	}
	return json.Unmarshal(data, &probe) == nil && probe.Status == "failed" && probe.Echo == ""
}

// readLoop 只在持有连接的 goroutine 里读；写走 writeJSON 加锁。
func (n *NapCat) readLoop(conn *websocket.Conn) {
	for {
		_, data, err := conn.ReadMessage()
		if err != nil {
			n.logger.Warn("WS 读取中断", "err", err)
			n.mu.Lock()
			if n.conn == conn {
				n.conn = nil
			}
			n.mu.Unlock()
			return
		}
		n.dispatch(data)
	}
}

// dispatch 按 OneBot 11 约定分流：带 post_type 的是事件，带 echo 的是动作响应。
func (n *NapCat) dispatch(data []byte) {
	var probe struct {
		PostType string `json:"post_type"`
		Echo     string `json:"echo"`
	}
	if err := json.Unmarshal(data, &probe); err != nil {
		n.logger.Error("消息不是合法 JSON", "err", err)
		return
	}

	switch {
	case probe.PostType != "":
		select {
		case n.events <- &pluginapi.Event{Payload: data}:
		default:
			nd := n.dropped.Add(1)
			n.logger.Warn("事件队列已满，丢弃一条事件", "dropped_total", nd)
		}
	case probe.Echo != "":
		var resp struct {
			Status  string          `json:"status"`
			Retcode int             `json:"retcode"`
			Data    json.RawMessage `json:"data"`
			Echo    string          `json:"echo"`
		}
		if err := json.Unmarshal(data, &resp); err != nil {
			n.logger.Error("动作响应解析失败", "err", err)
			return
		}
		n.pmu.Lock()
		ch := n.pending[resp.Echo]
		delete(n.pending, resp.Echo)
		n.pmu.Unlock()
		if ch != nil {
			ch <- &pluginapi.SendActionResponse{
				Status:  resp.Status,
				Retcode: int32(resp.Retcode),
				Data:    resp.Data,
			}
		}
	default:
		n.logger.Warn("无法识别的消息，已忽略", "head", head(data, 80))
	}
}

// StreamEvents 把事件流式推给核心。核心断开或退出时返回。
func (n *NapCat) StreamEvents(req *pluginapi.StreamEventsRequest, stream grpc.ServerStreamingServer[pluginapi.Event]) error {
	n.logger.Info("核心已订阅事件流")
	for {
		select {
		case <-stream.Context().Done():
			return stream.Context().Err()
		case ev := <-n.events:
			if err := stream.Send(ev); err != nil {
				return err
			}
		}
	}
}

// SendAction 让核心调用 NapCat 的 action：写请求、按 echo 等响应。
func (n *NapCat) SendAction(ctx context.Context, req *pluginapi.SendActionRequest) (*pluginapi.SendActionResponse, error) {
	timeout := defaultTimeout
	if req.TimeoutMs > 0 {
		timeout = time.Duration(req.TimeoutMs) * time.Millisecond
	}

	echo := fmt.Sprintf("loquat-%d", n.seq.Add(1))
	ch := make(chan *pluginapi.SendActionResponse, 1)
	n.pmu.Lock()
	n.pending[echo] = ch
	n.pmu.Unlock()
	defer func() {
		n.pmu.Lock()
		delete(n.pending, echo)
		n.pmu.Unlock()
	}()

	body := map[string]any{
		"action": req.Action,
		"params": json.RawMessage(req.Params),
		"echo":   echo,
	}
	if err := n.writeJSON(body); err != nil {
		return nil, fmt.Errorf("写动作请求失败: %w", err)
	}

	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	case <-time.After(timeout):
		return nil, fmt.Errorf("调用 %s 超时", req.Action)
	case resp := <-ch:
		return resp, nil
	}
}

func (n *NapCat) writeJSON(v any) error {
	n.mu.Lock()
	defer n.mu.Unlock()
	if n.conn == nil {
		return errors.New("ws 未连接")
	}
	// 写前设 deadline：对端半死（挂起、网络异常）时写会无限阻塞，
	// 且这把锁重连路径也要拿，不设限整个适配器僵死（26102202#3）
	_ = n.conn.SetWriteDeadline(time.Now().Add(n.writeDeadline))
	if err := n.conn.WriteJSON(v); err != nil {
		// 主动断开，让读循环走既有的重连与 failPending 语义
		_ = n.conn.Close()
		return fmt.Errorf("ws 写失败，连接已关闭待重连: %w", err)
	}
	return nil
}

// failPending 在连接断开时唤醒所有等响应的调用方，避免它们干等超时。
func (n *NapCat) failPending(reason error) {
	n.pmu.Lock()
	defer n.pmu.Unlock()
	for echo, ch := range n.pending {
		select {
		case ch <- &pluginapi.SendActionResponse{Status: "failed", Retcode: -1}:
		default:
		}
		delete(n.pending, echo)
	}
}

// head 按字符截断并兜底非法 UTF-8：按字节切中文会从码字中间断开（清单 #10）。
func head(data []byte, max int) string {
	runes := []rune(string(data))
	if len(runes) > max {
		return string(runes[:max]) + "..."
	}
	return string(runes)
}
