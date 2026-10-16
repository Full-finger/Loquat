// Package pluginapi 定义核心与适配器插件之间的 go-plugin 约定：
// 握手配置、插件封装，以及生成的 gRPC 存根（adapter.pb.go / adapter_grpc.pb.go）。
package pluginapi

import (
	"context"

	"github.com/hashicorp/go-plugin"
	"google.golang.org/grpc"
)

// Handshake 是宿主与插件的握手约定，两侧必须完全一致才能握手成功。
// MagicCookieValue 用于防止核心误拉起无关程序。
var Handshake = plugin.HandshakeConfig{
	ProtocolVersion:  1,
	MagicCookieKey:   "LOQUAT_PLUGIN",
	MagicCookieValue: "loquat-adapter-v1",
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
