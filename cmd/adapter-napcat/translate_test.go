package main

// translate.go 单测：OneBot ↔ IR 双向翻译。
// 事件侧用例移植自原宿主 event.rs 的测试（CQ 码边界：转义、qq=all、
// 1001≠100）；发送侧覆盖 chat 路由、段类型与 extension 保真。

import (
	"encoding/base64"
	"encoding/json"
	"strings"
	"testing"
	"time"

	"google.golang.org/protobuf/types/known/structpb"

	"github.com/full-finger/loquat/internal/pluginapi"
)

func mustTranslate(t *testing.T, raw string) *pluginapi.EventEnvelope {
	t.Helper()
	env, err := translateEvent([]byte(raw), time.UnixMilli(1700000000000))
	if err != nil {
		t.Fatalf("翻译失败: %v", err)
	}
	return env
}

// ── 事件：私聊 / 群聊 ──

func TestTranslatePrivateMessage(t *testing.T) {
	env := mustTranslate(t, `{"post_type":"message","message_type":"private","self_id":7,
		"message_id":100,"time":1699999999,"user_id":42,
		"message":"早","raw_message":"早","sender":{"user_id":42,"nickname":"nm"}}`)
	msg := env.GetMessage()
	if msg.GetChat().GetPrivateId() != "42" {
		t.Fatalf("chat 不符: %+v", msg.GetChat())
	}
	if msg.GetId() != "100" || msg.GetSender().GetId() != "42" || msg.GetSender().GetNickname() != "nm" {
		t.Fatalf("id/sender 不符: %+v", msg)
	}
	if env.GetTimeMs() != 1699999999000 {
		t.Fatalf("time_ms 不符: %d", env.GetTimeMs())
	}
	if msg.GetMentionsBot() {
		t.Fatal("无 at 段不应算 mentions_bot")
	}
	if textOf(msg.GetSegments()) != "早" {
		t.Fatalf("text 不符: %+v", msg.GetSegments())
	}
}

func TestTranslateGroupArrayAtBot(t *testing.T) {
	// 数组格式 at：qq 数字与字符串两种都要认（不同实现不一致，清单 26102201 #4）
	for _, qq := range []string{`7`, `"7"`} {
		raw := `{"post_type":"message","message_type":"group","self_id":7,"message_id":1,
			"group_id":99,"message":[{"type":"text","data":{"text":"早"}},{"type":"at","data":{"qq":` + qq + `}}],
			"raw_message":"[CQ:at,qq=7] 早","sender":{"user_id":42}}`
		env := mustTranslate(t, raw)
		if !env.GetMessage().GetMentionsBot() {
			t.Fatalf("qq=%s 应命中 mentions_bot", qq)
		}
		if env.GetMessage().GetChat().GetGroupId() != "99" {
			t.Fatalf("group chat 不符: %+v", env.GetMessage().GetChat())
		}
	}
}

func TestTranslateCQAtExactMatch(t *testing.T) {
	cases := []struct {
		msg  string
		want bool
	}{
		{"[CQ:at,qq=7] 早", true},
		{"看这个[CQ:at,qq=7,id=2]哦", true},
		{"[CQ:at,qq=8] 早", false},   // at 别人不算
		{"[CQ:at,qq=all] 早", false}, // at 全体不算
		{"[CQ:at,qq=70] 早", false},  // 前缀相似不误匹配
		{"无码纯文本", false},
	}
	for _, tc := range cases {
		raw := `{"post_type":"message","message_type":"private","self_id":7,"message_id":1,
			"user_id":42,"message":` + jsonString(tc.msg) + `,"raw_message":` + jsonString(tc.msg) + `,
			"sender":{"user_id":42}}`
		if got := mustTranslate(t, raw).GetMessage().GetMentionsBot(); got != tc.want {
			t.Errorf("%q: mentions_bot=%v want %v", tc.msg, got, tc.want)
		}
	}
}

// jsonString 造 JSON 字符串字面量（含 CQ 码的消息文本）。
func jsonString(s string) string {
	b, _ := json.Marshal(s)
	return string(b)
}

func TestTranslateImageSources(t *testing.T) {
	// 数组格式：data.url 优先，data.file 的 http 链接兑底（部分实现只给 file）
	raw := `{"post_type":"message","message_type":"private","self_id":7,"message_id":1,
		"user_id":42,
		"message":[
			{"type":"image","data":{"url":"http://a/1.png","file":"abc.image"}},
			{"type":"image","data":{"file":"http://b/2.png"}},
			{"type":"image","data":{"file":"local_id_3"}}
		],
		"raw_message":"","sender":{"user_id":42}}`
	segs := mustTranslate(t, raw).GetMessage().GetSegments()
	var urls []string
	for _, seg := range segs {
		if img := seg.GetImage(); img != nil {
			urls = append(urls, img.GetUrl()+"/"+img.GetFileId())
		}
	}
	want := []string{"http://a/1.png/", "http://b/2.png/", "/local_id_3"}
	if len(urls) != 3 || urls[0] != want[0] || urls[1] != want[1] || urls[2] != want[2] {
		t.Fatalf("图片来源不符: %v", urls)
	}
}

func TestTranslateCQImageEscape(t *testing.T) {
	// QQ 图床 URL 带逗号签名参数，CQ 码里转义成 &#44;，须反转义
	raw := `{"post_type":"message","message_type":"private","self_id":7,"message_id":1,
		"user_id":42,
		"message":"看[CQ:image,file=E3C0.http://x/a.jpg?sign&#44;abc,url=http://x/a.jpg?sign&#44;abc]",
		"raw_message":"同上","sender":{"user_id":42}}`
	env := mustTranslate(t, raw)
	var got string
	for _, seg := range env.GetMessage().GetSegments() {
		if img := seg.GetImage(); img != nil && img.GetUrl() != "" {
			got = img.GetUrl()
		}
	}
	if got != "http://x/a.jpg?sign,abc" {
		t.Fatalf("URL 反转义不符: %q", got)
	}
}

// ── 事件：丢弃口径 ──

func TestTranslateSelfMessageDropped(t *testing.T) {
	_, err := translateEvent([]byte(`{"post_type":"message","message_type":"private","self_id":42,
		"message_id":1,"user_id":42,"message":"hi","raw_message":"hi","sender":{"user_id":42}}`),
		time.Now())
	if err == nil || !strings.Contains(err.Error(), "机器人自己") {
		t.Fatalf("机器人自己的消息应被丢弃: %v", err)
	}
}

func TestTranslateMissingFieldsDropped(t *testing.T) {
	cases := []struct {
		name string
		raw  string
	}{
		{"缺 message_type", `{"post_type":"message","self_id":7,"message_id":1,"user_id":42,"message":"","raw_message":"","sender":{"user_id":42}}`},
		{"未知 message_type", `{"post_type":"message","message_type":"channel","self_id":7,"message_id":1,"user_id":42,"message":"","raw_message":"","sender":{"user_id":42}}`},
		{"私聊缺 user_id", `{"post_type":"message","message_type":"private","self_id":7,"message_id":1,"message":"","raw_message":"","sender":{"user_id":42}}`},
		{"群缺 group_id", `{"post_type":"message","message_type":"group","self_id":7,"message_id":1,"message":"","raw_message":"","sender":{"user_id":42}}`},
		{"群缺 sender.user_id", `{"post_type":"message","message_type":"group","self_id":7,"message_id":1,"group_id":99,"message":"","raw_message":"","sender":{}}`},
		{"未知 post_type", `{"post_type":"vibe","self_id":7}`},
	}
	for _, tc := range cases {
		if _, err := translateEvent([]byte(tc.raw), time.Now()); err == nil {
			t.Errorf("%s: 应返回错误", tc.name)
		}
	}
}

// ── CQ 码字符串拆段 ──

func TestParseCQStringMixed(t *testing.T) {
	// 文本与 CQ 码交替拆段；文本反转义 &#91;/&#93;
	segs := parseCQString("看&#91;图&#93;[CQ:image,url=http://x/a.png]好看")
	if len(segs) != 3 {
		t.Fatalf("段数不符: %d (%+v)", len(segs), segs)
	}
	if textOf(segs[0:1]) != "看[图]" {
		t.Fatalf("文本反转义不符: %q", textOf(segs[0:1]))
	}
	if segs[1].GetImage().GetUrl() != "http://x/a.png" {
		t.Fatalf("图片段不符: %+v", segs[1])
	}
	if textOf(segs[2:3]) != "好看" {
		t.Fatalf("尾段文本不符: %q", textOf(segs[2:3]))
	}
}

func TestParseCQStringMalformedKeptAsText(t *testing.T) {
	// 畸形码（参数无 =）整段按文本保留，不丢字
	segs := parseCQString("前[CQ:at,qq]后")
	if len(segs) != 1 {
		t.Fatalf("畸形码应并入文本: %+v", segs)
	}
	if got := textOf(segs); got != "前[CQ:at,qq]后" {
		t.Fatalf("原文回填不符: %q", got)
	}
}

func TestParseCQStringUnknownBecomesExtension(t *testing.T) {
	segs := parseCQString("[CQ:tts,text=hi]")
	ext := segs[0].GetExtension()
	if ext == nil || ext.GetNamespace() != "napcat" || ext.GetType() != "tts" {
		t.Fatalf("未知码应降级 extension: %+v", segs[0])
	}
	if ext.GetData().AsMap()["text"] != "hi" {
		t.Fatalf("extension data 不符: %+v", ext.GetData())
	}
}

// ── 发送：IR → OneBot ──

func TestBuildSendPrivateAndGroup(t *testing.T) {
	priv := &pluginapi.SendMessageRequest{
		Chat:     &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_PrivateId{PrivateId: "42"}},
		Segments: textSegs("hi"),
	}
	action, params, err := buildSendMessage(priv)
	if err != nil || action != "send_private_msg" || params["user_id"] != int64(42) {
		t.Fatalf("私聊不符: %s %v %v", action, params, err)
	}

	group := &pluginapi.SendMessageRequest{
		Chat:     &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: "99"}},
		Segments: textSegs("hi"),
	}
	action, params, err = buildSendMessage(group)
	if err != nil || action != "send_group_msg" || params["group_id"] != int64(99) {
		t.Fatalf("群聊不符: %s %v %v", action, params, err)
	}

	// 非数字 ID 原样传字符串（不同实现宽严不一）
	group.Chat = &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: "abc"}}
	_, params, _ = buildSendMessage(group)
	if params["group_id"] != "abc" {
		t.Fatalf("非数字 ID 应原样保留: %v", params["group_id"])
	}
}

func TestBuildSendSegments(t *testing.T) {
	st, _ := structpb.NewStruct(map[string]any{"text": "hi"})
	req := &pluginapi.SendMessageRequest{
		Chat: &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: "99"}},
		Segments: []*pluginapi.Segment{
			{Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: "7"}}},
			{Body: &pluginapi.Segment_Reply{Reply: &pluginapi.ReplySeg{MessageId: "100"}}},
			{Body: &pluginapi.Segment_Face{Face: &pluginapi.FaceSeg{FaceId: "5"}}},
			mediaSegOfType("image", "http://x/a.png", "", ""),
			{Body: &pluginapi.Segment_Extension{Extension: &pluginapi.ExtensionSeg{
				Namespace: "napcat", Type: "tts", Data: st,
			}}},
		},
	}
	_, params, err := buildSendMessage(req)
	if err != nil {
		t.Fatalf("构造失败: %v", err)
	}
	msg := params["message"].([]map[string]any)
	if msg[0]["type"] != "at" || msg[1]["type"] != "reply" || msg[2]["type"] != "face" ||
		msg[3]["type"] != "image" || msg[4]["type"] != "tts" {
		t.Fatalf("段类型不符: %+v", msg)
	}
}

func TestBuildSendInlineAndForeignExtension(t *testing.T) {
	// inline 二进制 → base64://
	req := &pluginapi.SendMessageRequest{
		Chat: &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_PrivateId{PrivateId: "42"}},
		Segments: []*pluginapi.Segment{{
			Body: &pluginapi.Segment_Image{Image: &pluginapi.MediaSeg{
				Source: &pluginapi.MediaSeg_Inline{Inline: []byte{0x01, 0x02}},
			}},
		}},
	}
	_, params, err := buildSendMessage(req)
	if err != nil {
		t.Fatalf("构造失败: %v", err)
	}
	want := "base64://" + base64.StdEncoding.EncodeToString([]byte{0x01, 0x02})
	if params["message"].([]map[string]any)[0]["data"].(map[string]any)["file"] != want {
		t.Fatalf("inline 编码不符: %v", params)
	}

	// 非 napcat 的 extension 无法表达，应报错
	req.Segments = []*pluginapi.Segment{{
		Body: &pluginapi.Segment_Extension{Extension: &pluginapi.ExtensionSeg{Namespace: "telegram"}},
	}}
	if _, _, err := buildSendMessage(req); err == nil {
		t.Fatal("异平台 extension 应报错")
	}
}

func TestBuildSendTextWithCQCode(t *testing.T) {
	// 旧插件约定：text 里嵌 CQ 码（如 status 发图）。适配器拆开翻译，
	// OneBot 平台知识收敛在此，其他平台适配器可自行决定当纯文本。
	req := &pluginapi.SendMessageRequest{
		Chat:     &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: "99"}},
		Segments: textSegs("看图[CQ:image,file=file:///tmp/a.png]好了"),
	}
	_, params, err := buildSendMessage(req)
	if err != nil {
		t.Fatalf("构造失败: %v", err)
	}
	msg := params["message"].([]map[string]any)
	if len(msg) != 3 || msg[0]["type"] != "text" || msg[1]["type"] != "image" || msg[2]["type"] != "text" {
		t.Fatalf("CQ 拆段不符: %+v", msg)
	}
	if msg[1]["data"].(map[string]any)["file"] != "file:///tmp/a.png" {
		t.Fatalf("图片参数不符: %+v", msg[1])
	}

	// 畸形 CQ 码按纯文本保留，不丢字
	req.Segments = textSegs("前[CQ:at,qq]后")
	_, params, _ = buildSendMessage(req)
	msg = params["message"].([]map[string]any)
	if len(msg) != 1 || msg[0]["data"].(map[string]any)["text"] != "前[CQ:at,qq]后" {
		t.Fatalf("畸形码应按纯文本: %+v", msg)
	}
}

// ── 辅助 ──

func textSegs(text string) []*pluginapi.Segment {
	return []*pluginapi.Segment{{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: text}}}}
}

// textOf 拼接段列表里所有 text 段。
func textOf(segs []*pluginapi.Segment) string {
	var b strings.Builder
	for _, seg := range segs {
		b.WriteString(seg.GetText().GetText())
	}
	return b.String()
}
