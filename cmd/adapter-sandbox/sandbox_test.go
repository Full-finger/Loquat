package main

// 沙箱适配器单测：InjectEvent 注入→信封、发送确认、动作 stub。
// 全链路靠构建后冒烟（WebUI 沙箱选项卡 → REST → 注入 → 回复帧）。

import (
	"context"
	"strings"
	"testing"

	"github.com/full-finger/loquat/internal/pluginapi"
)

type fakeLogger struct{}

func (f *fakeLogger) Info(msg string, kv ...any)  {}
func (f *fakeLogger) Error(msg string, kv ...any) {}

func TestInjectPrivate(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	_, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "你好", Mode: "private",
	})
	if err != nil {
		t.Fatalf("注入失败: %v", err)
	}

	select {
	case env := <-s.events:
		if env.GetPlatform() != platform || env.GetKind() != pluginapi.EventKind_EVENT_KIND_MESSAGE {
			t.Fatalf("信封头不符: %+v", env)
		}
		if env.GetSelfId() != botID {
			t.Fatalf("self_id 不符: %s", env.GetSelfId())
		}
		msg := env.GetMessage()
		if msg.GetChat().GetPrivateId() != userID {
			t.Fatalf("chat 不符: %+v", msg.GetChat())
		}
		if msg.GetSender().GetId() != userID || msg.GetSender().GetNickname() != userNick {
			t.Fatalf("sender 不符: %+v", msg.GetSender())
		}
		if msg.GetMentionsBot() {
			t.Fatal("私聊不应 mentions_bot")
		}
		if textOf(msg.GetSegments()) != "你好" {
			t.Fatalf("text 段不符: %+v", msg.GetSegments())
		}
	default:
		t.Fatal("事件未入队")
	}
}

func TestInjectGroupWithAt(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	_, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "在吗", Mode: "group", AtBot: true,
	})
	if err != nil {
		t.Fatalf("注入失败: %v", err)
	}

	env := <-s.events
	msg := env.GetMessage()
	if msg.GetChat().GetGroupId() != groupID {
		t.Fatalf("群 chat 不符: %+v", msg.GetChat())
	}
	if !msg.GetMentionsBot() {
		t.Fatal("atBot 应置 mentions_bot")
	}
	// at 段在前、text 在后，贴近真实消息形态
	if len(msg.GetSegments()) != 2 {
		t.Fatalf("段数不符: %+v", msg.GetSegments())
	}
	if msg.GetSegments()[0].GetAt().GetTarget() != botID {
		t.Fatalf("at 段不符: %+v", msg.GetSegments()[0])
	}
	if msg.GetSegments()[1].GetText().GetText() != "在吗" {
		t.Fatalf("text 段不符: %+v", msg.GetSegments()[1])
	}

	// 群聊不 at：不置 mentions_bot
	if _, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "闲聊", Mode: "group",
	}); err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if env = <-s.events; env.GetMessage().GetMentionsBot() {
		t.Fatal("未 at 不应 mentions_bot")
	}
}

func TestInjectRejectsBadInput(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	cases := []struct {
		name string
		req  pluginapi.InjectEventRequest
	}{
		{"空文本", pluginapi.InjectEventRequest{Text: "", Mode: "private"}},
		{"未知 mode", pluginapi.InjectEventRequest{Text: "hi", Mode: "channel"}},
		// chat_id 会拼进会话文件名，路径穿越必须挡住
		{"chat_id 带斜杠", pluginapi.InjectEventRequest{Text: "hi", Mode: "private", ChatId: "../x"}},
		{"chat_id 带空格", pluginapi.InjectEventRequest{Text: "hi", Mode: "private", ChatId: "a b"}},
	}
	// 索引访问取 req：proto 消息内嵌 sync.Mutex，range 拷贝会被 go vet 拦
	for i := range cases {
		if _, err := s.InjectEvent(context.Background(), &cases[i].req); err == nil {
			t.Errorf("%s: 应报错", cases[i].name)
		}
	}
	select {
	case env := <-s.events:
		t.Fatalf("坏请求不应入队: %+v", env)
	default:
	}
}

// TestInjectChatID 会话隔离：chat_id 原样成为 ChatRef 的会话标识，
// 缺省（空）时回落到固定会话，老前端与手工 curl 不受影响。
func TestInjectChatID(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	if _, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "hi", Mode: "private", ChatId: "10000.a1b2c3",
	}); err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if got := (<-s.events).GetMessage().GetChat().GetPrivateId(); got != "10000.a1b2c3" {
		t.Fatalf("私聊 chat 不符: %s", got)
	}

	if _, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "hi", Mode: "group", ChatId: "dev-group.d4e5f6",
	}); err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if got := (<-s.events).GetMessage().GetChat().GetGroupId(); got != "dev-group.d4e5f6" {
		t.Fatalf("群聊 chat 不符: %s", got)
	}

	// 不传 chat_id：沿用缺省会话
	if _, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Text: "hi", Mode: "private",
	}); err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if got := (<-s.events).GetMessage().GetChat().GetPrivateId(); got != userID {
		t.Fatalf("缺省 chat 不符: %s", got)
	}
}

// TestInjectSegments 富注入：segments 优先于 text，段类型原样透传，
// mentions_bot 由 at 段扫描（不再只看 atBot 开关），应答回消息 ID。
func TestInjectSegments(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	resp, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Mode: "private",
		Segments: []*pluginapi.Segment{
			{Body: &pluginapi.Segment_Reply{Reply: &pluginapi.ReplySeg{MessageId: "7"}}},
			{Body: &pluginapi.Segment_Image{Image: &pluginapi.MediaSeg{Source: &pluginapi.MediaSeg_Url{Url: "https://x/y.png"}}}},
			{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: "看这张"}}},
		},
	})
	if err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if resp.GetMessageId() == "" {
		fatal := "应答应带消息 ID"
		t.Fatal(fatal)
	}

	env := <-s.events
	msg := env.GetMessage()
	if msg.GetMentionsBot() {
		t.Fatal("无 at 段不应 mentions_bot")
	}
	segs := msg.GetSegments()
	if len(segs) != 3 {
		t.Fatalf("段数不符: %+v", segs)
	}
	if segs[0].GetReply().GetMessageId() != "7" {
		t.Fatalf("reply 段不符: %+v", segs[0])
	}
	if segs[1].GetImage().GetUrl() != "https://x/y.png" {
		t.Fatalf("image 段不符: %+v", segs[1])
	}
	if segs[2].GetText().GetText() != "看这张" {
		t.Fatalf("text 段不符: %+v", segs[2])
	}

	// 段里自带 at（不经 atBot 开关）同样置 mentions_bot
	if _, err := s.InjectEvent(context.Background(), &pluginapi.InjectEventRequest{
		Mode: "group",
		Segments: []*pluginapi.Segment{
			{Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: botID}}},
		},
	}); err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if env = <-s.events; !env.GetMessage().GetMentionsBot() {
		t.Fatal("段扫描应发现 at 机器人")
	}
}

func TestSendMessageAcknowledges(t *testing.T) {
	// 回复展示在 WebUI 消息流（宿主 publish_reply），适配器只确认
	s := newSandbox(&fakeLogger{})
	resp, err := s.SendMessage(context.Background(), &pluginapi.SendMessageRequest{
		Chat:     sandboxChat("group", ""),
		Segments: textSegs("回复"),
	})
	if err != nil {
		t.Fatalf("发送失败: %v", err)
	}
	if resp.GetMessageId() == "" {
		t.Fatal("应返回消息 ID")
	}
	select {
	case env := <-s.events:
		t.Fatalf("回复不应进事件流（防插件自问自答）: %+v", env)
	default:
	}
}

func TestSendAction(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	// 平台动作 stub：一律成功（开发期少噪音，失败分支靠 napcat 测）
	resp, err := s.SendAction(context.Background(), &pluginapi.SendActionRequest{Action: "delete_msg"})
	if err != nil || resp.GetStatus() != "ok" {
		t.Fatalf("动作 stub 应回成功: %v %v", err, resp.GetStatus())
	}
	// sandbox.echo：原样返回 params，验证链路连通
	resp, err = s.SendAction(context.Background(), &pluginapi.SendActionRequest{
		Action: "sandbox.echo", Params: []byte(`{"k":1}`),
	})
	if err != nil || string(resp.GetData()) != `{"k":1}` {
		t.Fatalf("echo 应原样返回 params: %v %+v", err, resp)
	}
	// sandbox.status：返回沙盒状态 JSON
	resp, err = s.SendAction(context.Background(), &pluginapi.SendActionRequest{Action: "sandbox.status"})
	if err != nil || resp.GetStatus() != "ok" || !strings.Contains(string(resp.GetData()), "events_buffered") {
		t.Fatalf("status 应返回状态 JSON: %v %+v", err, resp)
	}
}

// textSegs/textOf 与 translate_test.go 同款辅助（不同包，各自持有）。
func textSegs(text string) []*pluginapi.Segment {
	return []*pluginapi.Segment{{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: text}}}}
}

func textOf(segs []*pluginapi.Segment) string {
	var b strings.Builder
	for _, seg := range segs {
		b.WriteString(seg.GetText().GetText())
	}
	return b.String()
}
