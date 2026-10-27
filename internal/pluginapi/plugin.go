// Package pluginapi 定义核心与适配器插件之间的 go-plugin 约定：
// 握手配置、插件封装，以及生成的 gRPC 存根（adapter.pb.go / adapter_grpc.pb.go）。
//
// v2：统一消息格式（IR）。协议变化（Event → EventEnvelope、新增 SendMessage），
// 用握手版本号强制适配器与核心同代——混代进程握手直接失败，
// 比运行时字段缺失好排查。
package pluginapi

import (
	"context"

	"github.com/hashicorp/go-plugin"
	"google.golang.org/grpc"
)

// Handshake 是宿主与插件的握手约定，两侧必须完全一致才能握手成功。
// MagicCookieValue 用于防止核心误拉起无关程序。
var Handshake = plugin.HandshakeConfig{
	ProtocolVersion:  2,
	MagicCookieKey:   "LOQUAT_PLUGIN",
	MagicCookieValue: "loquat-adapter-v2",
}

// AdapterPlugin 把适配器实现挂到 go-plugin 上，宿主与插件两侧共用：
// 插件侧带 Impl 起服务，核心侧不带 Impl、Dispense 后拿 AdapterClient 桩。
type AdapterPlugin struct {
	plugin.NetRPCUnsupportedPlugin
	Impl AdapterServer
}

func (p *AdapterPlugin) GRPCServer(broker *plugin.GRPCBroker, s *grpc.Server) error {
	RegisterAdapterServer(s, p.Impl)
	return nil
}

func (p *AdapterPlugin) GRPCClient(ctx context.Context, broker *plugin.GRPCBroker, c *grpc.ClientConn) (interface{}, error) {
	return NewAdapterClient(c), nil
}
