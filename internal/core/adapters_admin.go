// 适配器管理：配置读取、热应用与写盘。供 WebUI 的适配器管理页调用。
// 设计：config/loquat.yaml 是唯一持久化来源；ApplyAdaptersConfig 走
// 读 YAML（yaml.v3 Node，尽量保留注释与排版）→ 改字段 → 写回 → 热应用 diff。
// 热应用语义：新平台 spawn+起流；移除平台停流+杀子进程；参数变更重建。
package core

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sort"

	"gopkg.in/yaml.v3"
)

// adapterConfig 是适配器管理页的读写载荷（JSON）。
type adapterConfig struct {
	Adapters []string `json:"adapters"`
	Napcat   struct {
		URL   string `json:"url"`
		Token string `json:"token"`
		Path  string `json:"path"`
	} `json:"napcat"`
	Sandbox struct {
		Path string `json:"path"`
	} `json:"sandbox"`
}

// AdaptersConfigJSON 返回当前适配器配置与各平台运行状态。
func AdaptersConfigJSON() string {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return "{}"
	}
	c.lockCfg()
	out := adapterConfig{}
	out.Adapters = c.cfg.enabledAdapters()
	out.Napcat.URL = c.cfg.Napcat.URL
	out.Napcat.Token = c.cfg.Napcat.Token
	out.Napcat.Path = c.cfg.Plugins.Napcat.Path
	out.Sandbox.Path = c.cfg.Plugins.Sandbox.Path
	c.unlockCfg()

	// 运行状态叠加（不进写回载荷，仅展示）
	type status struct {
		Platform string `json:"platform"`
		Alive    bool   `json:"alive"`
		PID      int    `json:"pid"`
	}
	var states []status
	c.runtimesMu.Lock()
	for name, rt := range c.runtimes {
		states = append(states, status{Platform: name, Alive: !rt.client.Exited(), PID: rt.pid})
	}
	c.runtimesMu.Unlock()
	sort.Slice(states, func(i, j int) bool { return states[i].Platform < states[j].Platform })

	b, err := json.Marshal(map[string]any{"config": out, "running": states})
	if err != nil {
		return "{}"
	}
	return string(b)
}

// ApplyAdaptersConfig 校验并应用新配置：写盘 + 热应用。返回给前端的说明文本。
// 请求失败时不动现有进程；写盘成功后热应用失败会继续返回错误（配置已保存，
// 重启后生效）。
func ApplyAdaptersConfig(jsonStr string) (string, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return "", ErrNotStarted
	}

	var next adapterConfig
	if err := json.Unmarshal([]byte(jsonStr), &next); err != nil {
		return "", fmt.Errorf("配置 JSON 解析失败: %w", err)
	}
	if err := validateAdapterConfig(&next); err != nil {
		return "", err
	}

	if err := c.writeAdapterConfig(&next); err != nil {
		return "", fmt.Errorf("写回 loquat.yaml 失败（未应用）: %w", err)
	}

	applied := c.applyAdapters(&next)
	return applied, nil
}

func validateAdapterConfig(cfg *adapterConfig) error {
	if len(cfg.Adapters) == 0 {
		return errors.New("至少启用一个适配器")
	}
	known := map[string]bool{}
	for _, name := range cfg.Adapters {
		switch name {
		case "napcat":
			if cfg.Napcat.URL == "" || cfg.Napcat.Path == "" {
				return errors.New("napcat：url 与 path 必填")
			}
		case "sandbox":
			if cfg.Sandbox.Path == "" {
				return errors.New("sandbox：path 必填")
			}
		default:
			return fmt.Errorf("未知适配器 %q（支持：napcat、sandbox）", name)
		}
		if known[name] {
			return fmt.Errorf("适配器 %q 重复", name)
		}
		known[name] = true
	}
	return nil
}

// writeAdapterConfig 用 yaml.v3 Node 读-改-写 loquat.yaml：
// Node 保住了注释与排版，用户手写的说明不至于被 GUI 一保存就抹掉。
// 文件不存在（首运引导未完成）时跳过写盘——此时没有可保真的结构。
func (c *Core) writeAdapterConfig(next *adapterConfig) error {
	path := c.configPath
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var doc yaml.Node
	if err := yaml.Unmarshal(data, &doc); err != nil {
		return fmt.Errorf("现有配置解析失败: %w", err)
	}
	if len(doc.Content) == 0 {
		return errors.New("现有配置为空结构，拒绝覆盖（请手改后重试）")
	}
	root := doc.Content[0] // document -> mapping
	if root.Kind != yaml.MappingNode {
		return errors.New("现有配置顶层不是映射，拒绝覆盖")
	}

	// napcat.url / napcat.token
	setMappingScalar(root, []string{"napcat", "url"}, next.Napcat.URL)
	setMappingScalar(root, []string{"napcat", "token"}, next.Napcat.Token)
	// plugins.napcat.path / plugins.sandbox.path
	setMappingScalar(root, []string{"plugins", "napcat", "path"}, next.Napcat.Path)
	setMappingScalar(root, []string{"plugins", "sandbox", "path"}, next.Sandbox.Path)
	// plugins.adapters 序列
	setAdaptersList(root, next.Adapters)

	out, err := yaml.Marshal(&doc)
	if err != nil {
		return err
	}
	return os.WriteFile(path, out, 0o644)
}

// setMappingScalar 按路径定位嵌套 mapping 的标量字段，存在则改值，
// 中间层缺失则静默跳过（保守：不为缺省结构造新键）。
func setMappingScalar(root *yaml.Node, path []string, value string) {
	cur := root
	for i, key := range path {
		keyNode, valNode := mappingEntry(cur, key)
		if i == len(path)-1 {
			if keyNode != nil && valNode != nil && valNode.Kind == yaml.ScalarNode {
				valNode.Value = value
				valNode.Tag = "!!str"
				valNode.Style = 0 // 双引号等样式交给 encoder 决定
			}
			return
		}
		if valNode == nil || valNode.Kind != yaml.MappingNode {
			return
		}
		cur = valNode
	}
}

// mappingEntry 取 mapping 里 key 的 (键节点, 值节点)。
func mappingEntry(m *yaml.Node, key string) (*yaml.Node, *yaml.Node) {
	if m.Kind != yaml.MappingNode {
		return nil, nil
	}
	for i := 0; i+1 < len(m.Content); i += 2 {
		if m.Content[i].Value == key {
			return m.Content[i], m.Content[i+1]
		}
	}
	return nil, nil
}

// setAdaptersList 改写 plugins.adapters 序列；无此键时新建。
func setAdaptersList(root *yaml.Node, adapters []string) {
	pluginsKey, pluginsVal := mappingEntry(root, "plugins")
	if pluginsKey == nil {
		return // plugins 段缺失：结构不完整，保守跳过
	}
	seq := &yaml.Node{Kind: yaml.SequenceNode, Tag: "!!seq"}
	for _, name := range adapters {
		seq.Content = append(seq.Content, &yaml.Node{
			Kind:  yaml.ScalarNode,
			Tag:   "!!str",
			Value: name,
		})
	}
	if _, val := mappingEntry(pluginsVal, "adapters"); val != nil {
		*val = *seq // 原地替换保留键节点（含注释）
		return
	}
	pluginsVal.Content = append(pluginsVal.Content,
		&yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: "adapters"},
		seq,
	)
}

// applyAdapters 热应用：改 cfg → diff 平台集合 → spawn/重建/移除。
// 返回给前端的结果说明。
func (c *Core) applyAdapters(next *adapterConfig) string {
	// 1) 改 cfg + 锁内读 diff 所需状态（不在锁内做 spawn/kill：
	//    那些函数自己也拿 runtimesMu，Mutex 不可重入）
	c.runtimesMu.Lock()
	c.cfgMu.Lock()
	c.cfg.Napcat.URL = next.Napcat.URL
	c.cfg.Napcat.Token = next.Napcat.Token
	c.cfg.Plugins.Napcat.Path = next.Napcat.Path
	c.cfg.Plugins.Sandbox.Path = next.Sandbox.Path
	want := next.Adapters
	had := make([]string, 0, len(c.runtimes))
	for name := range c.runtimes {
		had = append(had, name)
	}
	c.cfgMu.Unlock()

	wanted := map[string]bool{}
	for _, name := range want {
		wanted[name] = true
	}
	type change struct {
		name    string
		missing bool // 未启用过：spawn + 起流
		rebuild bool // 已启用但参数变了：杀掉让 run 循环重建
	}
	var changes []change
	var removed []string
	for _, name := range had {
		if wanted[name] {
			rt := c.runtimes[name]
			if c.adapterParamsChanged(name, rt) {
				changes = append(changes, change{name: name, rebuild: true})
			}
			continue
		}
		// 移除：停流 + 杀子进程 + 出册。必须 delete：run 循环已取消，
		// 不会再有 spawn 覆盖同 key，stale 条目会让「重加」误判为已存在
		// 而跳过 spawn（该平台永久 dead），还会在后续每次应用时反复误报
		// 「已移除」（26102701#6）。killRuntime/killAllRuntimes 的
		// 「只 Kill 不删」观测语义不受影响——那两条路径有 spawn 覆盖兜底。
		if cancel, ok := c.runCancels[name]; ok {
			cancel()
			delete(c.runCancels, name)
		}
		c.runtimes[name].client.Kill()
		delete(c.runtimes, name)
		removed = append(removed, name)
	}
	for _, name := range want {
		if _, exists := c.runtimes[name]; !exists {
			changes = append(changes, change{name: name, missing: true})
		}
	}
	c.runtimesMu.Unlock()

	// 2) 锁外执行：新增 / 重建
	var added, rebuilt []string
	var errs []string
	for _, ch := range changes {
		switch {
		case ch.missing:
			if err := c.spawn(ch.name); err != nil {
				errs = append(errs, fmt.Sprintf("%s 启动失败: %v", ch.name, err))
				continue
			}
			c.startRunLoop(ch.name)
			added = append(added, ch.name)
		case ch.rebuild:
			// 参数变更：杀旧子进程，run 循环检测到流断后按新参数 spawn。
			// 退避延迟最多 maxBackoff，期间该平台消息不可用
			c.killRuntime(ch.name)
			rebuilt = append(rebuilt, ch.name)
		}
	}

	sort.Strings(removed)
	sort.Strings(added)
	sort.Strings(rebuilt)
	msg := fmt.Sprintf("已应用：启用 %v", want)
	if len(added) > 0 {
		msg += fmt.Sprintf("；新增 %v", added)
	}
	if len(rebuilt) > 0 {
		msg += fmt.Sprintf("；参数变更重建 %v（按退避节奏重启）", rebuilt)
	}
	if len(removed) > 0 {
		msg += fmt.Sprintf("；移除 %v", removed)
	}
	if len(errs) > 0 {
		msg += "；" + fmt.Sprint(errs)
	}
	c.logger.Info("适配器配置已热应用", "detail", msg)
	return msg
}

// adapterParamsChanged 判断平台的启动参数（路径/url/token）是否变了。
// 调用方持 runtimesMu；本方法内部拿 cfgMu。
func (c *Core) adapterParamsChanged(platform string, rt *pluginRuntime) bool {
	c.lockCfg()
	defer c.unlockCfg()
	switch platform {
	case "sandbox":
		return c.cfg.Plugins.Sandbox.Path != rt.launchedPath
	default: // napcat
		return c.cfg.Plugins.Napcat.Path != rt.launchedPath ||
			c.cfg.Napcat.URL != rt.launchedURL ||
			c.cfg.Napcat.Token != rt.launchedToken
	}
}
