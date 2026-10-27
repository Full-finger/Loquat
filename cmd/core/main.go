//go:build !dll

// loquat 核心 exe 形态：第 1 步自测入口（固定回复）。
// 正式宿主是 Rust：加 dll 标签构建成 c-shared（见 dll.go）。
package main

import (
	"errors"
	"flag"
	"os"
	"os/signal"
	"strings"
	"time"

	"github.com/hashicorp/go-hclog"
	"google.golang.org/protobuf/encoding/protojson"

	"github.com/full-finger/loquat/internal/core"
	"github.com/full-finger/loquat/internal/pluginapi"
)

func main() {
	configPath := flag.String("config", "config/loquat.yaml", "配置文件路径")
	flag.Parse()

	logger := hclog.New(&hclog.LoggerOptions{
		Output: os.Stderr,
		Level:  hclog.Info,
		Name:   "core",
	})

	err := core.Start(*configPath, func(payload []byte) {
		logger.Info("收到事件信封", "json", string(payload))
		selfTestReply(payload, logger)
	}, logger)
	if err != nil {
		if errors.Is(err, core.ErrFirstRun) {
			// 首运引导非故障：模板已生成，等用户填 token，不定 Error 级（26102202#10）
			logger.Info(err.Error())
			os.Exit(1)
		}
		logger.Error("核心启动失败", "err", err)
		os.Exit(1)
	}
	logger.Info("exe 自测模式运行中（Ctrl+C 退出）")

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt)
	<-stop
	core.Stop()
}

// selfTestReply 私聊全回、群聊回 / 开头，固定文案。业务接入后删除。
// 事件已是统一信封（适配器翻译），这里直接走 SendMessage 段列表发送。
func selfTestReply(payload []byte, logger hclog.Logger) {
	var env pluginapi.EventEnvelope
	// consume 的投递是 protojson（camelCase/enum 名/int64 字符串），
	// encoding/json 解不了 oneof 与 enum，必须用 protojson
	if protojson.Unmarshal(payload, &env) != nil {
		logger.Error("事件信封解析失败")
		return
	}
	if env.GetKind() != pluginapi.EventKind_EVENT_KIND_MESSAGE {
		return
	}
	chat := env.GetMessage().GetChat()
	if chat == nil {
		return
	}

	text := "loquat 已连通（Go 侧自测，业务还没接入）"
	if chat.GetGroupId() != "" {
		first := ""
		for _, seg := range env.GetMessage().GetSegments() {
			if s := seg.GetText().GetText(); s != "" {
				first = s
				break
			}
		}
		if !strings.HasPrefix(first, "/") {
			return // 群聊只回 / 开头（旧自测约定）
		}
	}

	resp, err := core.SendMessage(&pluginapi.SendMessageRequest{
		Chat: chat,
		Segments: []*pluginapi.Segment{{
			Body: &pluginapi.Segment_Text{Text: &pluginapi.TextSeg{Text: text}},
		}},
	}, 15*time.Second)
	if err != nil {
		logger.Error("发送消息失败", "err", err)
		return
	}
	logger.Info("已回复", "message_id", resp.GetMessageId())
}
