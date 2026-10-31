// 沙箱适配器：不连任何真实平台。网页聊天窗建在 WebUI 的沙箱选项卡上
// （消息显示走宿主的消息流 WS，输入经 InjectEvent 注入），
// 消息仍走完整的 沙箱 → gRPC → 核心 → 宿主 → 插件 链路，保真度最高。
// 约定：stdout 被 go-plugin 握手占用，所有日志必须走 stderr。
package main

import (
	"context"
	"encoding/json"
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
// segments 优先（富注入：图片、回复引用等），否则按 text 构造单个文本段；
// 群聊 + atBot 时在段首插 at 段，mentions_bot 由段扫描判定。
func (s *Sandbox) InjectEvent(ctx context.Context, req *pluginapi.InjectEventRequest) (*pluginapi.InjectEventResponse, error) {
	// 未知 mode 直接拒绝：按私聊处理会静默错路由
	mode := req.GetMode()
	if mode != "private" && mode != "group" {
		return nil, fmt.Errorf("未知 mode %q", mode)
	}
	// chat_id 会进会话文件路径（data/sessions/<mode>_<chat_id>.json），
	// 只放行安全字符，挡住 ../ 之类的目录穿越
	if chatID := req.GetChatId(); chatID != "" && !validChatID(chatID) {
		return nil, fmt.Errorf("非法 chat_id %q", chatID)
	}

	segments := append([]*pluginapi.Segment{}, req.GetSegments()...)
	if len(segments) == 0 {
		if req.GetText() == "" {
			return nil, fmt.Errorf("注入消息为空")
		}
		segments = append(segments, &pluginapi.Segment{
			Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: req.GetText()}},
		})
	}
	if mode == "group" && req.GetAtBot() {
		// at 段在前，贴近真实消息形态；target = bot 自身 ID
		segments = append([]*pluginapi.Segment{{
			Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: botID}},
		}}, segments...)
	}
	mentionsBot := false
	for _, seg := range segments {
		if at := seg.GetAt(); at != nil && at.GetTarget() == botID {
			mentionsBot = true
			break
		}
	}

	id := s.nextID()
	msg := &pluginapi.UnifiedMessage{
		Id:          id,
		Chat:        sandboxChat(mode, req.GetChatId()),
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
	return &pluginapi.InjectEventResponse{MessageId: id}, nil
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

// SendAction 沙箱专用动作（sandbox. 前缀）真实处理：
//
//	sandbox.echo    原样返回 params（链路连通性验证）
//	sandbox.status  返回沙盒状态（缓冲事件数等）
//
// 其余动作一律回成功空数据：插件的撤回/戳一戳等调用不报错（开发期少噪音），
// 但也测不到真平台的失败分支——那要靠 napcat。
func (s *Sandbox) SendAction(ctx context.Context, req *pluginapi.SendActionRequest) (*pluginapi.SendActionResponse, error) {
	switch req.GetAction() {
	case "sandbox.echo":
		return &pluginapi.SendActionResponse{Status: "ok", Retcode: 0, Data: req.GetParams()}, nil
	case "sandbox.status":
		s.mu.Lock()
		buffered := len(s.events)
		s.mu.Unlock()
		data, _ := json.Marshal(map[string]any{
			"platform": platform, "events_buffered": buffered,
		})
		return &pluginapi.SendActionResponse{Status: "ok", Retcode: 0, Data: data}, nil
	}
	return &pluginapi.SendActionResponse{Status: "ok", Retcode: 0, Data: []byte("{}")}, nil
}

// validChatID：1-64 位，只允许字母数字与 . _ -。
// 会话标识最终会拼进文件名，白名单比黑名单省心。
func validChatID(id string) bool {
	if id == "" || len(id) > 64 {
		return false
	}
	for _, r := range id {
		switch {
		case r >= 'a' && r <= 'z', r >= 'A' && r <= 'Z', r >= '0' && r <= '9':
		case r == '.', r == '_', r == '-':
		default:
			return false
		}
	}
	return true
}

// sandboxChat 构造会话引用：chatID 由调用方给（WebUI 每个会话一个），
// 留空回落到固定缺省会话，兼容老前端与手工 curl 注入。
func sandboxChat(mode, chatID string) *pluginapi.ChatRef {
	if chatID == "" {
		if mode == "group" {
			chatID = groupID
		} else {
			chatID = userID
		}
	}
	if mode == "group" {
		return &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: chatID}}
	}
	return &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_PrivateId{PrivateId: chatID}}
}

// nextID 发消息 ID：Mutex 保底。
func (s *Sandbox) nextID() string {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.seq++
	return strconv.FormatInt(s.seq, 10)
}
