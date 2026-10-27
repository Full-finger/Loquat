// 沙箱适配器插件入口。以 go-plugin 子进程形态被核心拉起，
// 无环境变量依赖（不连任何外部服务）。
package main

import (
	"os"

	"github.com/hashicorp/go-hclog"
	"github.com/hashicorp/go-plugin"

	"github.com/full-finger/loquat/internal/pluginapi"
)

func main() {
	// JSON 格式：go-plugin 客户端按 @level 还原真实级别（同 adapter-napcat）。
	logger := hclog.New(&hclog.LoggerOptions{
		Output:     os.Stderr,
		Level:      hclog.Info,
		Name:       "adapter-sandbox",
		JSONFormat: true,
	})

	plugin.Serve(&plugin.ServeConfig{
		HandshakeConfig: pluginapi.Handshake,
		Plugins: map[string]plugin.Plugin{
			"adapter": &pluginapi.AdapterPlugin{Impl: newSandbox(logger)},
		},
		GRPCServer: plugin.DefaultGRPCServer,
	})
}
