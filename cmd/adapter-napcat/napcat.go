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
	confirmTimeout = 5 * time.Second  // 等首条消息确认鉴权的上限
	writeTimeout   = 10 * time.Second // WS 写上限：对端半死时不能无限阻塞（26102202#3）

	// keepalive（26102701#4）：没有它，TCP 半开（对端断电/NAT 超时）时
	// ReadMessage 永久阻塞，本地 gRPC 依然健康，重启循环永不触发——
	// 适配器活着、连接死了，机器人静默失联。
	pingInterval = 30 * time.Second // 主动 Ping 周期
	pongWait     = 60 * time.Second // 读空闲上限：收到 Pong 或任何消息都续期

	firstReconnect = time.Second // 重连退避起步
	maxReconnect   = 30 * time.Second
)

// nextBackoff 指数退避的下一次等待：×2 封顶 maxReconnect。
func nextBackoff(cur time.Duration) time.Duration {
	return min(cur*2, maxReconnect)
}

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

	events chan *pluginapi.EventEnvelope
}

func newNapCat(url, token string, logger hclog.Logger) *NapCat {
	return &NapCat{
		url:           url,
		token:         token,
		logger:        logger,
		pending:       make(map[string]chan *pluginapi.SendActionResponse),
		events:        make(chan *pluginapi.EventEnvelope, eventsBuffer),
		writeDeadline: writeTimeout,
	}
}

// connectLoop 维持与 NapCat 的长连接：断开就指数退避重连（1s 起、×2、
// 上限 30s，连接确认成功后复位）。该函数永不返回。
func (n *NapCat) connectLoop() {
	// 鉴权走 Authorization 头而非 URL query，避免 token 落进日志（清单 #8）
	header := http.Header{}
	if n.token != "" {
		header.Set("Authorization", "Bearer "+n.token)
	}
	backoff := firstReconnect
	for {
		conn, _, err := websocket.DefaultDialer.Dial(n.url, header)
		if err != nil {
			n.logger.Error("连接 NapCat 失败", "err", err, "retry", backoff.String())
			time.Sleep(backoff)
			backoff = nextBackoff(backoff)
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
			n.failPending()
			n.logger.Error("NapCat 连接不可用", "retry", backoff.String())
			time.Sleep(backoff)
			backoff = nextBackoff(backoff)
			continue
		}
		n.logger.Info("已连接 NapCat", "url", n.url)
		backoff = firstReconnect

		stopPing := make(chan struct{})
		go n.pingLoop(conn, stopPing)
		n.readLoop(conn) // 阻塞直到连接断开
		close(stopPing)

		n.failPending()
		n.logger.Info("与 NapCat 断开", "retry", backoff.String())
		time.Sleep(backoff)
		backoff = nextBackoff(backoff)
	}
}

// pingLoop 周期发 WS Ping 探活：对端 gorilla 层自动回 Pong，读侧的
// PongHandler 借此续读超时。写失败（对端半死）主动断连，readLoop 随之
// 退出走既有重连语义。
func (n *NapCat) pingLoop(conn *websocket.Conn, stop <-chan struct{}) {
	ticker := time.NewTicker(pingInterval)
	defer ticker.Stop()
	for {
		select {
		case <-stop:
			return
		case <-ticker.C:
			n.mu.Lock()
			var err error
			if n.conn == conn {
				_ = conn.SetWriteDeadline(time.Now().Add(n.writeDeadline))
				err = conn.WriteControl(websocket.PingMessage, nil,
					time.Now().Add(n.writeDeadline))
			}
			n.mu.Unlock()
			if err != nil {
				_ = conn.Close() // 唤醒 readLoop 走重连
				return
			}
		}
	}
}

// armKeepalive 挂 PongHandler 并设读空闲上限。读空闲判定是双保险：
// 有流量（任何消息）续期，纯空闲靠 Pong 续期；两者都没有 = 连接已死。
func armKeepalive(conn *websocket.Conn) {
	conn.SetPongHandler(func(string) error {
		return conn.SetReadDeadline(time.Now().Add(pongWait))
	})
	_ = conn.SetReadDeadline(time.Now().Add(pongWait))
}

// confirm 等首条消息确认鉴权：收到鉴权失败帧或连接提前断开返回 false；
// 超时无消息的连接按已连接处理（可能只是不发事件），返回 true 并告警。
// 读到的合法首条帧交给 dispatch，不丢。确认通过即布防 keepalive。
func (n *NapCat) confirm(conn *websocket.Conn) bool {
	_ = conn.SetReadDeadline(time.Now().Add(confirmTimeout))
	_, data, err := conn.ReadMessage()
	switch {
	case err == nil:
		if authFailed(data) {
			n.logger.Error("NapCat 鉴权失败，请检查 token", "reply", head(data, 120))
			return false
		}
		armKeepalive(conn)
		n.dispatch(data)
		return true
	case errors.Is(err, os.ErrDeadlineExceeded):
		n.logger.Warn("NapCat 连接后超时无消息，无法确认鉴权，按已连接处理")
		armKeepalive(conn)
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
// 每条消息都续读空闲期：有流量就不需要额外探活。
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
		_ = conn.SetReadDeadline(time.Now().Add(pongWait))
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
		ev, err := translateEvent(data, time.Now())
		if err != nil {
			if errors.Is(err, errSelfMessage) {
				return // 机器人自己的消息，静默丢弃防自问自答
			}
			n.logger.Error("事件翻译失败，已丢弃", "err", err, "head", head(data, 120))
			return
		}
		select {
		case n.events <- ev:
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

// StreamEvents 把统一信封流式推给核心。核心断开或退出时返回。
func (n *NapCat) StreamEvents(req *pluginapi.StreamEventsRequest, stream grpc.ServerStreamingServer[pluginapi.EventEnvelope]) error {
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

// SendMessage 把统一发送请求翻成 OneBot action 调用，返回新消息 ID。
func (n *NapCat) SendMessage(ctx context.Context, req *pluginapi.SendMessageRequest) (*pluginapi.SendMessageResponse, error) {
	action, params, err := buildSendMessage(req)
	if err != nil {
		return nil, err
	}
	b, err := json.Marshal(params)
	if err != nil {
		return nil, fmt.Errorf("params 序列化失败: %w", err)
	}
	resp, err := n.SendAction(ctx, &pluginapi.SendActionRequest{Action: action, Params: b})
	if err != nil {
		return nil, err
	}
	// 平台拒绝不再当成功（26102701#5）：retcode/status 透传失败原因，
	// 宿主的统计与重试才有依据。async = OneBot 异步受理，已排队不等
	// 执行结果，message_id 拿不到，留空不算失败。
	if resp.GetStatus() == "failed" || (resp.GetRetcode() != 0 && resp.GetStatus() != "async") {
		return nil, fmt.Errorf("发送被 NapCat 拒绝: status=%s retcode=%d data=%s",
			resp.GetStatus(), resp.GetRetcode(), head(resp.GetData(), 120))
	}
	var data struct {
		MessageID json.Number `json:"message_id"`
	}
	_ = json.Unmarshal(resp.GetData(), &data) // 解不出 ID 就留空
	return &pluginapi.SendMessageResponse{MessageId: data.MessageID.String()}, nil
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
func (n *NapCat) failPending() {
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
