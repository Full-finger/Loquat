// 沙箱适配器：不连任何真实平台。网页聊天窗建在 WebUI 的沙箱选项卡上
// （消息显示走宿主的消息流 WS，输入经 InjectEvent 注入），
// 消息仍走完整的 沙箱 → gRPC → 核心 → 宿主 → 插件 链路，保真度最高。
// 约定：stdout 被 go-plugin 握手占用，所有日志必须走 stderr。
package main

import (
	"context"
	"fmt"
	"strconv"
	"sync"
	"time"

	"google.golang.org/grpc"

	"github.com/full-finger/loquat/internal/pluginapi"
)

const (
	eventsBuffer = 256           // 事件缓冲上限，满了丢弃（与 napcat 同语义）
	platform     = "sandbox"     // 信封 platform 标识
	botID        = "sandbox-bot" // 机器人自身 ID（at 判定锚点）
	userID       = "10000"       // 模拟用户的固定 ID
	userNick     = "开发者"
	groupID      = "dev-group" // 群聊模式的固定群号
)

// Sandbox 沙箱适配器：Adapter 服务（事件由 InjectEvent 注入）。
type Sandbox struct {
	pluginapi.UnimplementedAdapterServer

	logger hclogSlim

	mu     sync.Mutex
	events chan *pluginapi.EventEnvelope
	seq    int64 // 消息 ID 发号器（单页面场景，互斥保底即可）
}

// logger 收窄依赖成小接口：单测不必起真 go-plugin。
type hclogSlim interface {
	Info(msg string, kv ...any)
	Error(msg string, kv ...any)
}

func newSandbox(logger hclogSlim) *Sandbox {
	return &Sandbox{
		logger: logger,
		events: make(chan *pluginapi.EventEnvelope, eventsBuffer),
	}
}

// InjectEvent 接收 WebUI 沙箱选项卡注入的消息（经核心转发），
// 包装成统一信封入队，走与真实平台完全相同的事件链路。
func (s *Sandbox) InjectEvent(ctx context.Context, req *pluginapi.InjectEventRequest) (*pluginapi.InjectEventResponse, error) {
	if req.GetText() == "" {
		return nil, fmt.Errorf("注入消息文本为空")
	}
	// 未知 mode 直接拒绝：按私聊处理会静默错路由
	mode := req.GetMode()
	if mode != "private" && mode != "group" {
		return nil, fmt.Errorf("未知 mode %q", mode)
	}

	var segments []*pluginapi.Segment
	mentionsBot := false
	if mode == "group" && req.GetAtBot() {
		// at 段在前，贴近真实消息形态；target = bot 自身 ID
		segments = append(segments, &pluginapi.Segment{
			Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: botID}},
		})
		mentionsBot = true
	}
	segments = append(segments, &pluginapi.Segment{
		Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: req.GetText()}},
	})

	msg := &pluginapi.UnifiedMessage{
		Id:          s.nextID(),
		Chat:        sandboxChat(mode),
		Sender:      &pluginapi.Sender{Id: userID, Nickname: userNick},
		Segments:    segments,
		MentionsBot: mentionsBot,
	}
	env := &pluginapi.EventEnvelope{
		Platform: platform,
		Kind:     pluginapi.EventKind_EVENT_KIND_MESSAGE,
		TimeMs:   time.Now().UnixMilli(),
		SelfId:   botID,
		Message:  msg,
	}
	select {
	case s.events <- env:
	default:
		s.logger.Error("事件队列已满，丢弃一条消息")
	}
	return &pluginapi.InjectEventResponse{}, nil
}

// StreamEvents 把信封流式推给核心。核心断开或退出时返回。
func (s *Sandbox) StreamEvents(req *pluginapi.StreamEventsRequest, stream grpc.ServerStreamingServer[pluginapi.EventEnvelope]) error {
	for {
		select {
		case <-stream.Context().Done():
			return stream.Context().Err()
		case ev := <-s.events:
			if err := stream.Send(ev); err != nil {
				return err
			}
		}
	}
}

// SendMessage 确认接收即返回：回复的展示不在适配器（沙箱没有本地页面），
// 宿主发送成功后经 publish_reply 推 WebUI 消息流，沙箱选项卡从那里取。
func (s *Sandbox) SendMessage(ctx context.Context, req *pluginapi.SendMessageRequest) (*pluginapi.SendMessageResponse, error) {
	return &pluginapi.SendMessageResponse{MessageId: s.nextID()}, nil
}

// SendAction 沙箱不支持长尾动作：一律回成功空数据，
// 插件的撤回/戳一戳等调用不报错（开发期少噪音）。
func (s *Sandbox) SendAction(ctx context.Context, req *pluginapi.SendActionRequest) (*pluginapi.SendActionResponse, error) {
	return &pluginapi.SendActionResponse{Status: "ok", Retcode: 0, Data: []byte("{}")}, nil
}

func sandboxChat(mode string) *pluginapi.ChatRef {
	if mode == "group" {
		return &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: groupID}}
	}
	return &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_PrivateId{PrivateId: userID}}
}

// nextID 发消息 ID：Mutex 保底。
func (s *Sandbox) nextID() string {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.seq++
	return strconv.FormatInt(s.seq, 10)
}
