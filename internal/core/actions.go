// 核心下行动作：宿主（Rust FFI）经此调用适配器——发消息、透传平台动作、
// 注入沙箱事件、查询状态。按 platform 路由，核心不猜默认平台。
// 从 core.go 按职责分出：生命周期在 core.go，配置在 config.go。
package core

import (
	"context"
	"encoding/json"
	"sort"
	"time"

	"github.com/full-finger/loquat/internal/pluginapi"
)

// SendAction 供宿主（Rust FFI）透传平台动作，按 platform 路由。并发安全。
// platform 来自调用方解析（消息上下文跟随来源平台），核心不猜。
func SendAction(platform, action string, params []byte, timeout time.Duration) (*pluginapi.SendActionResponse, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return nil, ErrNotStarted
	}
	rt, err := c.runtimeFor(platform)
	if err != nil {
		return nil, err
	}
	if timeout <= 0 {
		timeout = actionTimeout
	}
	ctx, cancel := context.WithTimeout(c.ctx, timeout)
	defer cancel()
	return rt.adapter.SendAction(ctx, &pluginapi.SendActionRequest{
		Action:   action,
		Params:   params,
		Platform: platform,
	})
}

// InjectEvent 注入一条本地事件（沙箱调试用），按 platform 路由。并发安全。
func InjectEvent(platform, text string, atBot bool, mode string) error {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return ErrNotStarted
	}
	rt, err := c.runtimeFor(platform)
	if err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(c.ctx, actionTimeout)
	defer cancel()
	_, err = rt.adapter.InjectEvent(ctx, &pluginapi.InjectEventRequest{
		Text: text, AtBot: atBot, Mode: mode,
	})
	return err
}

// SendMessage 统一发送：段列表交适配器翻译成平台消息。并发安全。
func SendMessage(req *pluginapi.SendMessageRequest, timeout time.Duration) (*pluginapi.SendMessageResponse, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return nil, ErrNotStarted
	}
	rt, err := c.runtimeFor(req.GetPlatform())
	if err != nil {
		return nil, err
	}
	if timeout <= 0 {
		timeout = actionTimeout
	}
	ctx, cancel := context.WithTimeout(c.ctx, timeout)
	defer cancel()
	return rt.adapter.SendMessage(ctx, req)
}

// Status 返回核心状态快照 JSON，供宿主的 status 命令展示。
func Status() string {
	mu.Lock()
	c := active
	mu.Unlock()
	started := c != nil
	dropped := int64(0)
	adapters := []map[string]any{}
	if started {
		dropped = c.eventsDropped.Load()
		c.runtimesMu.Lock()
		for name, rt := range c.runtimes {
			adapters = append(adapters, map[string]any{
				"platform": name,
				"alive":    !rt.client.Exited(),
				"pid":      rt.pid,
			})
		}
		c.runtimesMu.Unlock()
		sort.Slice(adapters, func(i, j int) bool {
			return adapters[i]["platform"].(string) < adapters[j]["platform"].(string)
		})
	}
	b, err := json.Marshal(map[string]any{
		"started": started, "adapters": adapters,
		"events_dropped": dropped,
	})
	if err != nil {
		return "{}"
	}
	return string(b)
}
