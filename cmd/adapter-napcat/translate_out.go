// OneBot 11 发送方向：IR 段列表 → OneBot action 与数组格式消息段。
// 接收方向见 translate.go；协议知识全部收敛在本包，核心与宿主只认 IR。
package main

import (
	"encoding/base64"
	"fmt"
	"strconv"
	"strings"

	"github.com/full-finger/loquat/internal/pluginapi"
)

// buildSendMessage 把 SendMessageRequest 翻成 OneBot action 与 params JSON。
// 消息用数组格式（结构化段无损），ID 能转数字就转数字（OneBot 11 惯例）。
func buildSendMessage(req *pluginapi.SendMessageRequest) (string, map[string]any, error) {
	if req.GetChat() == nil {
		return "", nil, fmt.Errorf("SendMessage 缺 chat")
	}
	var action string
	params := map[string]any{}
	switch kind := req.GetChat().Kind.(type) {
	case *pluginapi.ChatRef_PrivateId:
		action = "send_private_msg"
		params["user_id"] = looseID(kind.PrivateId)
	case *pluginapi.ChatRef_GroupId:
		action = "send_group_msg"
		params["group_id"] = looseID(kind.GroupId)
	case *pluginapi.ChatRef_ChannelId:
		return "", nil, fmt.Errorf("napcat 适配器不支持频道 chat")
	default:
		return "", nil, fmt.Errorf("SendMessage 的 chat 缺 kind")
	}

	msg := make([]map[string]any, 0, len(req.GetSegments()))
	for i, seg := range req.GetSegments() {
		obs, err := segToOneBotSegs(seg)
		if err != nil {
			return "", nil, fmt.Errorf("第 %d 个消息段: %w", i, err)
		}
		msg = append(msg, obs...)
	}
	params["message"] = msg
	return action, params, nil
}

// segToOneBotSegs 把单个 IR 段翻成一段或多段 OneBot 数组格式段。
// text 段内嵌 CQ 码（如旧插件返回 "[CQ:image,file=...]" 发图）时拆开翻译；
// 这是 OneBot 平台知识，收敛在适配器，其他平台适配器可自行决定当纯文本。
func segToOneBotSegs(seg *pluginapi.Segment) ([]map[string]any, error) {
	if ts := seg.GetText(); ts != nil && strings.Contains(ts.GetText(), "[CQ:") {
		return textWithCQToOneBot(ts.GetText())
	}
	ob, err := segToOneBot(seg)
	if err != nil {
		return nil, err
	}
	return []map[string]any{ob}, nil
}

// textWithCQToOneBot 拆 CQ 码字符串为 OneBot 段列表。
// 不递归 segToOneBotSegs：畸形码回填的文本里仍含 "[CQ:"，会无限递归，
// 这里对回填文本直接按纯文本输出。
func textWithCQToOneBot(text string) ([]map[string]any, error) {
	segs := parseCQString(text)
	out := make([]map[string]any, 0, len(segs))
	for _, seg := range segs {
		if _, isText := seg.GetBody().(*pluginapi.Segment_Text); isText {
			out = append(out, map[string]any{
				"type": "text", "data": map[string]any{"text": seg.GetText().GetText()},
			})
			continue
		}
		ob, err := segToOneBot(seg)
		if err != nil {
			return nil, err
		}
		out = append(out, ob)
	}
	return out, nil
}

// segToOneBot 把单个 IR 段翻回 OneBot 数组格式段。
func segToOneBot(seg *pluginapi.Segment) (map[string]any, error) {
	switch body := seg.GetBody().(type) {
	case *pluginapi.Segment_Text:
		return map[string]any{"type": "text", "data": map[string]any{"text": body.Text.GetText()}}, nil
	case *pluginapi.Segment_At:
		return map[string]any{"type": "at", "data": map[string]any{"qq": looseID(body.At.GetTarget())}}, nil
	case *pluginapi.Segment_Reply:
		return map[string]any{"type": "reply", "data": map[string]any{"id": looseID(body.Reply.GetMessageId())}}, nil
	case *pluginapi.Segment_Face:
		return map[string]any{"type": "face", "data": map[string]any{"id": body.Face.GetFaceId()}}, nil
	case *pluginapi.Segment_Image:
		return mediaToOneBot("image", body.Image)
	case *pluginapi.Segment_Voice:
		return mediaToOneBot("record", body.Voice)
	case *pluginapi.Segment_Video:
		return mediaToOneBot("video", body.Video)
	case *pluginapi.Segment_File:
		return mediaToOneBot("file", body.File)
	case *pluginapi.Segment_Extension:
		if body.Extension.GetNamespace() != "napcat" {
			return nil, fmt.Errorf("extension 段 namespace=%q 无法表达", body.Extension.GetNamespace())
		}
		data := map[string]any{}
		for k, v := range body.Extension.GetData().AsMap() {
			data[k] = v
		}
		return map[string]any{"type": body.Extension.GetType(), "data": data}, nil
	default:
		return nil, fmt.Errorf("未知段类型 %T", seg.GetBody())
	}
}

// mediaToOneBot 把 IR 媒体段翻回 OneBot 数组格式段：
// OneBot 的 image/record/video/file 都用 file 参数承载来源。
func mediaToOneBot(kind string, m *pluginapi.MediaSeg) (map[string]any, error) {
	var file string
	switch src := m.GetSource().(type) {
	case *pluginapi.MediaSeg_Url:
		file = src.Url
	case *pluginapi.MediaSeg_FileId:
		file = src.FileId
	case *pluginapi.MediaSeg_Inline:
		file = "base64://" + base64.StdEncoding.EncodeToString(src.Inline)
	default:
		return nil, fmt.Errorf("媒体段缺来源")
	}
	data := map[string]any{"file": file}
	if name := m.GetFilename(); name != "" && kind == "file" {
		data["name"] = name
	}
	return map[string]any{"type": kind, "data": data}, nil
}

// looseID 能转数字转数字，转不动原样返回字符串（不同实现宽严不一）。
func looseID(s string) any {
	if n, err := strconv.ParseInt(s, 10, 64); err == nil {
		return n
	}
	return s
}
