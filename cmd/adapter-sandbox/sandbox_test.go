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

func TestSendMessageAcknowledges(t *testing.T) {
	// 回复展示在 WebUI 消息流（宿主 publish_reply），适配器只确认
	s := newSandbox(&fakeLogger{})
	resp, err := s.SendMessage(context.Background(), &pluginapi.SendMessageRequest{
		Chat:     sandboxChat("group"),
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

func TestSendActionAlwaysOk(t *testing.T) {
	s := newSandbox(&fakeLogger{})
	resp, err := s.SendAction(context.Background(), &pluginapi.SendActionRequest{Action: "delete_msg"})
	if err != nil || resp.GetStatus() != "ok" {
		t.Fatalf("动作 stub 应回成功: %v %v", err, resp.GetStatus())
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
