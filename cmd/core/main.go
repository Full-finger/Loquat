//go:build !dll

// loquat 核心 exe 形态：第 1 步自测入口（固定回复）。
// 正式宿主是 Rust：加 dll 标签构建成 c-shared（见 dll.go）。
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"os"
	"os/signal"
	"strings"
	"time"

	"github.com/hashicorp/go-hclog"

	"github.com/full-finger/loquat/internal/core"
)

// ob11Message 只取自测关心的 OneBot 11 字段。
type ob11Message struct {
	PostType    string `json:"post_type"`
	MessageType string `json:"message_type"`
	UserID      int64  `json:"user_id"`
	GroupID     int64  `json:"group_id"`
	RawMessage  string `json:"raw_message"`
}

func main() {
	configPath := flag.String("config", "config/loquat.yaml", "配置文件路径")
	flag.Parse()

	logger := hclog.New(&hclog.LoggerOptions{
		Output: os.Stderr,
		Level:  hclog.Info,
		Name:   "core",
	})

	err := core.Start(*configPath, func(payload []byte) {
		logger.Info("收到事件", "json", string(payload))
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
func selfTestReply(payload []byte, logger hclog.Logger) {
	var m ob11Message
	if json.Unmarshal(payload, &m) != nil {
		return
	}
	if m.PostType != "message" {
		return
	}

	var action string
	params := map[string]any{"message": "loquat 已连通（Go 侧自测，业务还没接入）"}
	switch m.MessageType {
	case "private":
		action = "send_private_msg"
		params["user_id"] = m.UserID
	case "group":
		if !strings.HasPrefix(m.RawMessage, "/") {
			return
		}
		action = "send_group_msg"
		params["group_id"] = m.GroupID
	default:
		return
	}

	pjson, err := json.Marshal(params)
	if err != nil {
		return
	}
	resp, err := core.SendAction(action, pjson, 15*time.Second)
	if err != nil {
		logger.Error("发送动作失败", "action", action, "err", err)
		return
	}
	logger.Info("已回复", "action", action, "retcode", resp.GetRetcode())
}
