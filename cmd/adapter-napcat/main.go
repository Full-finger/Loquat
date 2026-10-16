// NapCat 适配器插件入口。以 go-plugin 子进程形态被核心拉起，
// 配置经环境变量传入（配置中心的正式下发通道后续接入）。
package main

import (
	"os"

	"github.com/hashicorp/go-hclog"
	"github.com/hashicorp/go-plugin"

	"github.com/full-finger/loquat/internal/pluginapi"
)

func main() {
	logger := hclog.New(&hclog.LoggerOptions{
		Output: os.Stderr,
		Level:  hclog.Info,
		Name:   "adapter-napcat",
	})

	url := os.Getenv("LOQUAT_NAPCAT_URL")
	token := os.Getenv("LOQUAT_NAPCAT_TOKEN")
	if url == "" {
		logger.Error("缺少 LOQUAT_NAPCAT_URL 环境变量")
		os.Exit(1)
	}

	n := newNapCat(url, token, logger)
	go n.connectLoop()

	plugin.Serve(&plugin.ServeConfig{
		HandshakeConfig: pluginapi.Handshake,
		Plugins: map[string]plugin.Plugin{
			"adapter": &pluginapi.AdapterPlugin{Impl: n},
		},
		GRPCServer: plugin.DefaultGRPCServer,
	})
}
