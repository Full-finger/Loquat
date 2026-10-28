// 适配器运行时：一次 spawn 的产物与其在 Core 上的登记/查找/查杀。
// 从 core.go 按职责分出：run 循环的退避重启逻辑仍在 core.go。
package core

import (
	"errors"
	"fmt"

	"github.com/hashicorp/go-plugin"

	"github.com/full-finger/loquat/internal/pluginapi"
)

// pluginRuntime 打包一次 spawn 的产物：插件客户端与 Dispense 出的适配器桩。
// 经 atomic.Pointer 整体换装——写端 Store 新实例，读端 Load 一次快照用到底，
// 避免 client/adapter 两字段在 spawn 重启与并发调用间的散装数据竞争。
// 读到旧快照意味着旧适配器正被替换，调用随其 Kill 返回错误，属预期行为。
type pluginRuntime struct {
	client  *plugin.Client
	adapter pluginapi.AdapterClient
	pid     int // 适配器子进程 PID，供系统状态采集
	// spawn 时的启动参数快照：适配器管理 diff「参数是否变更」用
	launchedPath  string
	launchedURL   string
	launchedToken string
}

// getRuntime 取平台对应的 runtime。未启用/未就绪返回 false。
func (c *Core) getRuntime(platform string) (*pluginRuntime, bool) {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	rt, ok := c.runtimes[platform]
	return rt, ok
}

// runtimeFor 下行调用的路由入口：platform 必填，未启用报错。
// 消息该发给谁由调用方决定——核心不猜（猜错会打到真实用户）。
func (c *Core) runtimeFor(platform string) (*pluginRuntime, error) {
	if platform == "" {
		return nil, errors.New("platform 必填（下行按平台路由，不猜默认）")
	}
	rt, ok := c.getRuntime(platform)
	if !ok {
		return nil, fmt.Errorf("适配器 %s 未启用或未就绪", platform)
	}
	return rt, nil
}

// killRuntime 杀掉平台插件客户端。可重复调用。与 spawn 的换装持同一把锁：
// 杀的时候不会有新实例插入到身后。
// 只 Kill 不从 map 删除：Status/采集方仍能读到该平台的 PID（alive 如实
// 变 false），run 循环退避后 spawn 会覆盖同 key。
func (c *Core) killRuntime(platform string) {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	if rt, ok := c.runtimes[platform]; ok {
		rt.client.Kill()
	}
}

// killAllRuntimes 停止/重启路径用：杀掉全部平台的插件客户端。
// 只 Kill 不删 map（同 killRuntime 的观测语义）；spawn 会覆盖同 key。
func (c *Core) killAllRuntimes() {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	for _, rt := range c.runtimes {
		rt.client.Kill()
	}
}
