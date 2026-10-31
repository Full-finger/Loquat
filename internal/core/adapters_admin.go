// 适配器管理：配置读取、热应用与写盘。供 WebUI 的适配器管理页调用。
// 设计：config/loquat.yaml 是唯一持久化来源；ApplyAdaptersConfig 走
// 读 YAML（yaml.v3 Node，尽量保留注释与排版）→ 改字段 → 写回 → 热应用 diff。
// 热应用语义：新启用平台 spawn+起流；停用/移除平台停流+杀子进程；
// 定义变更重建。载荷是通用 AdapterDef 列表，核心不感知平台差异。
package core

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"

	"gopkg.in/yaml.v3"
)

// adapterConfig 是适配器管理页的读写载荷（JSON）：通用定义列表。
// env 的键名含义由各适配器自己约定，核心与前端不解释内容。
type adapterConfig struct {
	Adapters []AdapterDef `json:"adapters"`
}

// discoveredAdapter bin/ 里发现的适配器二进制，可能尚未写入配置（发现流）。
type discoveredAdapter struct {
	Name string `json:"name"`
	Path string `json:"path"`
}

// adapterView 前端展示视图：定义 + 解析后的启动文件。只读字段，
// 前端发回时被 json.Unmarshal 忽略，不会污染配置。
type adapterView struct {
	AdapterDef
	ResolvedPath string `json:"resolved_path"`
}

// discoverAdapters 扫 bin/ 下 adapter-<name>[.exe]（发现流）：二进制放
// 进去即出现在 WebUI 列表，无需先写配置；配置条目仍可覆盖 path/env/enabled。
// 目录不存在（如尚未编译产物）视为空。
func discoverAdapters() []discoveredAdapter {
	return discoverAdaptersIn("bin")
}

func discoverAdaptersIn(dir string) []discoveredAdapter {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil
	}
	var out []discoveredAdapter
	for _, e := range entries {
		if e.IsDir() {
			continue
		}
		file := e.Name()
		if strings.HasSuffix(file, "~") { // 编辑器备份，如 adapter-x.exe~
			continue
		}
		rest, ok := strings.CutPrefix(strings.TrimSuffix(file, ".exe"), "adapter-")
		if !ok || rest == "" {
			continue
		}
		out = append(out, discoveredAdapter{Name: rest, Path: dir + "/" + file})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Name < out[j].Name })
	return out
}

// resolvedPath 启动文件的实际取值：显式 path 优先，否则按约定补齐。
func resolvedPath(d AdapterDef) string {
	if d.Path != "" {
		return d.Path
	}
	return defaultAdapterPath(d.Name)
}

// AdaptersConfigJSON 返回当前适配器定义（含解析后启动文件）、
// 各平台运行状态与 bin/ 下发现但未配置的适配器。
func AdaptersConfigJSON() string {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return "{}"
	}
	c.lockCfg()
	defs := c.cfg.Plugins.Adapters
	c.unlockCfg()
	views := make([]adapterView, len(defs))
	for i, d := range defs {
		views[i] = adapterView{AdapterDef: d, ResolvedPath: resolvedPath(d)}
	}

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

	b, err := json.Marshal(map[string]any{
		"config":     map[string]any{"adapters": views},
		"running":    states,
		"discovered": discoverAdapters(),
	})
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
	for i := range next.Adapters {
		if next.Adapters[i].Path == "" {
			next.Adapters[i].Path = defaultAdapterPath(next.Adapters[i].Name)
		}
	}

	if err := c.writeAdapterConfig(&next); err != nil {
		return "", fmt.Errorf("写回 loquat.yaml 失败（未应用）: %w", err)
	}

	applied := c.applyAdapters(&next)
	return applied, nil
}

// validateAdapterConfig 定义列表基本校验：name 必填且不重复。
// 允许全停用（空启用集）：核心照常运行，只是没有适配器。
func validateAdapterConfig(cfg *adapterConfig) error {
	return validateDefs(cfg.Adapters)
}

// writeAdapterConfig 用 yaml.v3 Node 读-改-写 loquat.yaml：
// Node 保住了注释与排版，用户手写的说明不至于被 GUI 一保存就抹掉。
// 只改写 plugins.adapters 序列为通用定义列表；旧版遗留的顶层 napcat:
// 与 plugins.<平台>: 节不清理（读时新格式优先，残留无害，用户手删即可）。
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

	// plugins.adapters 序列（通用定义列表，替代旧的平台分节）
	setAdaptersList(root, next.Adapters)

	out, err := yaml.Marshal(&doc)
	if err != nil {
		return err
	}
	return os.WriteFile(path, out, 0o644)
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

// setAdaptersList 把 plugins.adapters 序列改写为通用定义列表；无此键时新建。
// 每项写 name/path/env，enabled 只在停用时写（省略即启用）。
func setAdaptersList(root *yaml.Node, adapters []AdapterDef) {
	pluginsKey, pluginsVal := mappingEntry(root, "plugins")
	if pluginsKey == nil {
		return // plugins 段缺失：结构不完整，保守跳过
	}
	seq := &yaml.Node{Kind: yaml.SequenceNode, Tag: "!!seq"}
	for _, d := range adapters {
		item := &yaml.Node{Kind: yaml.MappingNode, Content: adapterDefPairs(d)}
		seq.Content = append(seq.Content, item)
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

// adapterDefNode 把一份定义写成键值对节点序列（挂在父 mapping 的 Content 里）。
func adapterDefPairs(d AdapterDef) []*yaml.Node {
	scalar := func(v string) *yaml.Node {
		return &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: v}
	}
	pair := func(key string, val *yaml.Node) []*yaml.Node {
		return []*yaml.Node{scalar(key), val}
	}
	pairs := pair("name", scalar(d.Name))
	if d.Path != "" {
		pairs = append(pairs, pair("path", scalar(d.Path))...)
	}
	if len(d.Env) > 0 {
		env := &yaml.Node{Kind: yaml.MappingNode}
		for _, k := range sortedKeys(d.Env) {
			env.Content = append(env.Content, pair(k, scalar(d.Env[k]))...)
		}
		pairs = append(pairs, pair("env", env)...)
	}
	if d.Enabled != nil && !*d.Enabled {
		pairs = append(pairs, pair("enabled",
			&yaml.Node{Kind: yaml.ScalarNode, Tag: "!!bool", Value: "false"})...)
	}
	return pairs
}

// sortedKeys map 键的排序切片：写盘顺序稳定。
func sortedKeys(m map[string]string) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// applyAdapters 热应用：改 cfg → diff 平台集合 → spawn/重建/移除。
// 返回给前端的结果说明。
func (c *Core) applyAdapters(next *adapterConfig) string {
	// 1) 改 cfg + 锁内读 diff 所需状态（不在锁内做 spawn/kill：
	//    那些函数自己也拿 runtimesMu，Mutex 不可重入）
	c.runtimesMu.Lock()
	c.cfgMu.Lock()
	c.cfg.Plugins.Adapters = next.Adapters
	wanted := map[string]bool{} // 启用集（enabled=false 不在其中）
	for _, d := range next.Adapters {
		if d.IsEnabled() {
			wanted[d.Name] = true
		}
	}
	had := make([]string, 0, len(c.runtimes))
	for name := range c.runtimes {
		had = append(had, name)
	}
	c.cfgMu.Unlock()

	type change struct {
		name    string
		missing bool // 未启用过：spawn + 起流
		rebuild bool // 已启用但参数变了：杀掉让 run 循环重建
	}
	var changes []change
	var removed []string
	var want []string
	for name := range wanted {
		want = append(want, name)
	}
	sort.Strings(want)
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
			def, ok := c.adapterDef(ch.name)
			if !ok {
				errs = append(errs, fmt.Sprintf("%s 启动失败: 配置中无该适配器", ch.name))
				continue
			}
			if err := c.spawn(def); err != nil {
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

// adapterParamsChanged 判断平台的启动定义是否变了（path/env/enabled）。
// 调用方持 runtimesMu；本方法内部拿 cfgMu。
func (c *Core) adapterParamsChanged(platform string, rt *pluginRuntime) bool {
	c.lockCfg()
	defer c.unlockCfg()
	def, ok := c.cfg.adapterDef(platform)
	return !ok || !def.Equal(rt.launched)
}
