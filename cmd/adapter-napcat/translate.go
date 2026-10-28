// OneBot 11 ↔ 统一消息格式（IR）的双向翻译。
// 协议知识全部收敛在此：核心与宿主只认 EventEnvelope / Segment。
// 事件侧：post_type 判 kind、message 段解析（数组与 CQ 码字符串两种格式）、
// at 判定与图片提取。发送方向（IR → OneBot）见 translate_out.go。
package main

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"google.golang.org/protobuf/types/known/structpb"

	"github.com/full-finger/loquat/internal/pluginapi"
)

// translateEvent 把一条 OneBot 11 事件 JSON 翻译成统一信封。
// message 事件缺关键字段时返回错误（调用方记日志丢弃，沿用原宿主口径：
// 出新类型或字段变化时消息不再无声消失）；非 message 事件只装信封不翻消息。
func translateEvent(raw []byte, recvAt time.Time) (*pluginapi.EventEnvelope, error) {
	var ev struct {
		PostType    string          `json:"post_type"`
		Time        int64           `json:"time"`
		SelfID      int64           `json:"self_id"`
		MessageType string          `json:"message_type"`
		MessageID   json.Number     `json:"message_id"`
		UserID      json.Number     `json:"user_id"`
		GroupID     json.Number     `json:"group_id"`
		Message     json.RawMessage `json:"message"`
		Sender      struct {
			UserID   json.Number `json:"user_id"`
			Nickname string      `json:"nickname"`
		} `json:"sender"`
	}
	if err := json.Unmarshal(raw, &ev); err != nil {
		return nil, fmt.Errorf("事件 JSON 解析失败: %w", err)
	}

	kind := kindFromPostType(ev.PostType)
	if kind == pluginapi.EventKind_EVENT_KIND_UNSPECIFIED {
		return nil, fmt.Errorf("未知的 post_type=%q", ev.PostType)
	}

	timeMs := recvAt.UnixMilli()
	if ev.Time > 0 {
		timeMs = ev.Time * 1000
	}
	env := &pluginapi.EventEnvelope{
		Platform: "napcat",
		Kind:     kind,
		TimeMs:   timeMs,
		Raw:      raw,
		SelfId:   strconv.FormatInt(ev.SelfID, 10),
	}
	if kind != pluginapi.EventKind_EVENT_KIND_MESSAGE {
		return env, nil
	}

	// chat：私聊 = 对方 user_id，群聊 = group_id
	msg := &pluginapi.UnifiedMessage{
		Id: ev.MessageID.String(),
		Sender: &pluginapi.Sender{
			Id:       ev.Sender.UserID.String(),
			Nickname: ev.Sender.Nickname,
		},
	}
	switch ev.MessageType {
	case "private":
		if ev.UserID.String() == "" || ev.UserID.String() == "0" {
			return nil, fmt.Errorf("私聊事件缺 user_id")
		}
		msg.Chat = &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_PrivateId{PrivateId: ev.UserID.String()}}
	case "group":
		if ev.Sender.UserID.String() == "" {
			return nil, fmt.Errorf("群事件缺 sender.user_id")
		}
		if ev.GroupID.String() == "" || ev.GroupID.String() == "0" {
			return nil, fmt.Errorf("群事件缺 group_id")
		}
		msg.Chat = &pluginapi.ChatRef{Kind: &pluginapi.ChatRef_GroupId{GroupId: ev.GroupID.String()}}
	default:
		return nil, fmt.Errorf("未知的 message_type=%q", ev.MessageType)
	}

	segs, err := parseMessageField(ev.Message)
	if err != nil {
		return nil, err
	}
	// 机器人自己的消息不上报，防自问自答（原宿主口径，挪到适配器）
	if ev.SelfID != 0 && ev.Sender.UserID.String() == strconv.FormatInt(ev.SelfID, 10) {
		return nil, errSelfMessage
	}
	msg.Segments = segs
	msg.MentionsBot = segsMention(segs, ev.SelfID)
	env.Message = msg
	return env, nil
}

// errSelfMessage 标记机器人自己的消息：不是故障，调用方静默跳过。
var errSelfMessage = fmt.Errorf("机器人自己的消息，丢弃")

func kindFromPostType(postType string) pluginapi.EventKind {
	switch postType {
	case "message":
		return pluginapi.EventKind_EVENT_KIND_MESSAGE
	case "notice":
		return pluginapi.EventKind_EVENT_KIND_NOTICE
	case "request":
		return pluginapi.EventKind_EVENT_KIND_REQUEST
	case "meta_event":
		return pluginapi.EventKind_EVENT_KIND_META
	}
	return pluginapi.EventKind_EVENT_KIND_UNSPECIFIED
}

// parseMessageField 解析 OneBot message 字段：数组格式（消息段 list）或
// CQ 码字符串格式。缺失或空视为纯空文本。
func parseMessageField(raw json.RawMessage) ([]*pluginapi.Segment, error) {
	trimmed := strings.TrimSpace(string(raw))
	if trimmed == "" || trimmed == "null" {
		return []*pluginapi.Segment{{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: ""}}}}, nil
	}
	if strings.HasPrefix(trimmed, "[") {
		var arr []map[string]any
		if err := json.Unmarshal(raw, &arr); err != nil {
			return nil, fmt.Errorf("message 数组解析失败: %w", err)
		}
		segs := make([]*pluginapi.Segment, 0, len(arr))
		for _, item := range arr {
			seg, err := segFromMap(item)
			if err != nil {
				return nil, err
			}
			segs = append(segs, seg)
		}
		return segs, nil
	}
	// 字符串格式：CQ 码字符串
	var s string
	if err := json.Unmarshal(raw, &s); err != nil {
		return nil, fmt.Errorf("message 字段既非数组也非字符串: %w", err)
	}
	return parseCQString(s), nil
}

// segFromMap 把 OneBot 数组格式的单个消息段翻译成 IR 段。
// 认识的 type 走 IR；不认识的装进 napcat extension 段保真。
func segFromMap(item map[string]any) (*pluginapi.Segment, error) {
	segType, _ := item["type"].(string)
	data, _ := item["data"].(map[string]any)
	get := func(key string) string {
		if data == nil {
			return ""
		}
		switch v := data[key].(type) {
		case string:
			return v
		case float64:
			return strconv.FormatInt(int64(v), 10)
		}
		return ""
	}

	switch segType {
	case "text":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: get("text")}}}, nil
	case "at":
		return &pluginapi.Segment{Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: get("qq")}}}, nil
	case "reply":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Reply{Reply: &pluginapi.ReplySeg{MessageId: get("id")}}}, nil
	case "face":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Face{Face: &pluginapi.FaceSeg{FaceId: get("id")}}}, nil
	case "image", "record", "video", "file":
		return mediaSegOfType(segType, get("url"), get("file"), get("name")), nil
	default:
		st, err := structpb.NewStruct(data)
		if err != nil {
			return nil, fmt.Errorf("未知消息段 %q 且 data 无法保真: %w", segType, err)
		}
		return &pluginapi.Segment{Body: &pluginapi.Segment_Extension{Extension: &pluginapi.ExtensionSeg{
			Namespace: "napcat",
			Type:      segType,
			Data:      st,
		}}}, nil
	}
}

// mediaSeg 组媒体段：url 优先；file 为 http 链接时兜底（部分实现只给 file）；
// 其余 file 值当平台文件 ID。
func mediaSegOfType(kind, url, file, name string) *pluginapi.Segment {
	m := &pluginapi.MediaSeg{Filename: name}
	switch {
	case url != "":
		m.Source = &pluginapi.MediaSeg_Url{Url: url}
	case strings.HasPrefix(file, "http"):
		m.Source = &pluginapi.MediaSeg_Url{Url: file}
	case file != "":
		m.Source = &pluginapi.MediaSeg_FileId{FileId: file}
	}
	switch kind {
	case "record":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Voice{Voice: m}}
	case "video":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Video{Video: m}}
	case "file":
		return &pluginapi.Segment{Body: &pluginapi.Segment_File{File: m}}
	default: // image
		return &pluginapi.Segment{Body: &pluginapi.Segment_Image{Image: m}}
	}
}

// ── CQ 码字符串解析 ──

// parseCQString 把 CQ 码字符串拆成 IR 段：[CQ:type,k=v,...] 与普通文本交替。
// 文本做 &#91;/&#93;/&amp; 反转义；CQ 码参数值额外反转义 &#44;（QQ 图床
// URL 常带逗号签名参数）。无法识别的码降级为 napcat extension 段。
func parseCQString(s string) []*pluginapi.Segment {
	var segs []*pluginapi.Segment
	var text strings.Builder
	flushText := func() {
		if text.Len() > 0 {
			segs = append(segs, &pluginapi.Segment{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{
				Text: cqUnescapeText(text.String()),
			}}})
			text.Reset()
		}
	}
	rest := s
	for {
		pos := strings.Index(rest, "[CQ:")
		if pos < 0 {
			break
		}
		end := strings.Index(rest[pos:], "]")
		if end < 0 {
			break // 畸形码：剩余部分按文本收尾
		}
		end += pos
		text.WriteString(rest[:pos])
		code := rest[pos+len("[CQ:") : end]
		raw := rest[pos : end+1] // 含方括号的原文，畸形码回填用
		rest = rest[end+1:]

		if seg, ok := cqCodeToSeg(code); ok {
			flushText()
			segs = append(segs, seg)
		} else {
			// 畸形参数或未知码：原样回填文本，不丢字
			text.WriteString(raw)
		}
	}
	text.WriteString(rest)
	flushText()
	if len(segs) == 0 {
		segs = append(segs, &pluginapi.Segment{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: ""}}})
	}
	return segs
}

// cqCodeToSeg 解析单个 CQ 码体（type[,k=v...]）。返回 false 表示整码按文本处理。
func cqCodeToSeg(code string) (*pluginapi.Segment, bool) {
	parts := strings.Split(code, ",")
	segType := strings.TrimSpace(parts[0])
	params := map[string]string{}
	for _, p := range parts[1:] {
		eq := strings.Index(p, "=")
		if eq < 0 {
			return nil, false // 无 = 的参数块：畸形，整码按文本
		}
		params[p[:eq]] = cqUnescapeValue(p[eq+1:])
	}

	switch segType {
	case "text":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: params["text"]}}}, true
	case "at":
		return &pluginapi.Segment{Body: &pluginapi.Segment_At{At: &pluginapi.AtSeg{Target: params["qq"]}}}, true
	case "reply":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Reply{Reply: &pluginapi.ReplySeg{MessageId: params["id"]}}}, true
	case "face":
		return &pluginapi.Segment{Body: &pluginapi.Segment_Face{Face: &pluginapi.FaceSeg{FaceId: params["id"]}}}, true
	case "image", "record", "video", "file":
		return mediaSegOfType(segType, params["url"], params["file"], params["name"]), true
	default:
		// CQ 码参数已扁平，extension.data 用 k=v 原样装
		data := map[string]any{}
		for k, v := range params {
			data[k] = v
		}
		if st, err := structpb.NewStruct(data); err == nil {
			return &pluginapi.Segment{Body: &pluginapi.Segment_Extension{Extension: &pluginapi.ExtensionSeg{
				Namespace: "napcat",
				Type:      segType,
				Data:      st,
			}}}, true
		}
		return nil, false
	}
}

// CQ 码值反转义（OneBot 11 约定）。&amp; 放最后（转义顺序的逆）。
func cqUnescapeValue(s string) string {
	return strings.NewReplacer("&#44;", ",", "&#91;", "[", "&#93;", "]", "&amp;", "&").Replace(s)
}

// CQ 码外文本反转义：没有 &#44;（逗号在文本里不转义）。
func cqUnescapeText(s string) string {
	return strings.NewReplacer("&#91;", "[", "&#93;", "]", "&amp;", "&").Replace(s)
}

// segsMention 判断段列表是否 at 了指定 bot：target 精确等于 bot ID；
// at 全体（"all"）不算。
func segsMention(segs []*pluginapi.Segment, botID int64) bool {
	if botID == 0 {
		return false
	}
	want := strconv.FormatInt(botID, 10)
	for _, seg := range segs {
		if at := seg.GetAt(); at != nil && at.Target == want {
			return true
		}
	}
	return false
}
