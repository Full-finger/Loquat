// 核心配置：loquat.yaml 的结构与校验、适配器定义、旧格式迁移。
// 从 core.go 按职责分出：生命周期在 core.go，下行动作在 actions.go。
package core

import (
	"errors"
	"fmt"
	"os"
	"runtime"
	"sort"

	"gopkg.in/yaml.v3"
)

// AdapterDef 单个适配器的启动定义：名字、可执行文件、环境变量。
// 核心对平台零知识——平台差异全装在 path 与 env 里，由适配器自己解释；
// 新增平台不改核心代码，配置里加一项即可。
type AdapterDef struct {
	Name    string            `yaml:"name" json:"name"`
	Path    string            `yaml:"path,omitempty" json:"path,omitempty"`
	Env     map[string]string `yaml:"env,omitempty" json:"env,omitempty"`
	Enabled *bool             `yaml:"enabled,omitempty" json:"enabled,omitempty"`

	// legacy 标记旧格式的字符串项（`- napcat`）：迁移时允许从旧配置位置
	// 补 path 与参数。非序列化字段，只在解析与迁移之间存活。
	legacy bool `yaml:"-" json:"-"`
}

// UnmarshalYAML 兼容两种写法：字符串（旧格式，`- napcat`）与映射（新格式）。
func (d *AdapterDef) UnmarshalYAML(value *yaml.Node) error {
	if value.Kind == yaml.ScalarNode {
		d.Name = value.Value
		d.legacy = true
		return nil
	}
	if value.Kind != yaml.MappingNode {
		return fmt.Errorf("适配器定义须是名字或映射（第 %d 行）", value.Line)
	}
	type plain AdapterDef // 规避 UnmarshalYAML 递归
	if err := value.Decode((*plain)(d)); err != nil {
		return err
	}
	return nil
}

// IsEnabled 停用开关。字段缺省即启用，只有显式 false 才停用。
func (d AdapterDef) IsEnabled() bool {
	return d.Enabled == nil || *d.Enabled
}

// EnvSlice 环境变量的确定性切片（KEY=value），spawn 用。
// map 迭代无序，排序让同配置产出同 env，日志与测试可复现。
func (d AdapterDef) EnvSlice() []string {
	keys := make([]string, 0, len(d.Env))
	for k := range d.Env {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := make([]string, 0, len(keys))
	for _, k := range keys {
		out = append(out, k+"="+d.Env[k])
	}
	return out
}

// Equal 两份定义的生效字段是否完全一致：热应用 diff 判断「参数变更重建」用。
func (d AdapterDef) Equal(o AdapterDef) bool {
	if d.Name != o.Name || d.Path != o.Path || d.IsEnabled() != o.IsEnabled() {
		return false
	}
	if len(d.Env) != len(o.Env) {
		return false
	}
	for k, v := range d.Env {
		if ov, ok := o.Env[k]; !ok || ov != v {
			return false
		}
	}
	return true
}

// normalize 补默认值：path 逐级兜底（映射项显式值 → 旧配置分节 → 按名猜测）；
// 旧格式字符串项的 napcat 参数从顶层 napcat: 节换成 env 键。
func (d *AdapterDef) normalize(lg legacyConfig) {
	if d.Path == "" {
		d.Path = legacyPath(lg, d.Name)
	}
	if d.Path == "" {
		d.Path = defaultAdapterPath(d.Name)
	}
	if d.legacy && d.Name == "napcat" && len(d.Env) == 0 &&
		(lg.Napcat.URL != "" || lg.Napcat.Token != "") {
		d.Env = map[string]string{
			"LOQUAT_NAPCAT_URL":   lg.Napcat.URL,
			"LOQUAT_NAPCAT_TOKEN": lg.Napcat.Token,
		}
	}
}

// legacyConfig 抓旧版配置的字段（配置去平台化之前）：顶层 napcat 参数、
// 按平台分节的 path、字符串形式的 adapters 列表。只服务旧文件迁移，
// 新配置不再产生这些节；删迁移逻辑时这里的平台名一并消失。
type legacyConfig struct {
	Napcat struct {
		URL   string `yaml:"url"`
		Token string `yaml:"token"`
	} `yaml:"napcat"`
	Plugins struct {
		Adapters []string `yaml:"adapters"`
		Napcat   struct {
			Path string `yaml:"path"`
		} `yaml:"napcat"`
		Sandbox struct {
			Path string `yaml:"path"`
		} `yaml:"sandbox"`
	} `yaml:"plugins"`
}

// legacyPath 旧配置按平台分节的 path。
func legacyPath(lg legacyConfig, name string) string {
	switch name {
	case "napcat":
		return lg.Plugins.Napcat.Path
	case "sandbox":
		return lg.Plugins.Sandbox.Path
	}
	return ""
}

// defaultAdapterPath path 缺省时按名字猜：约定优于配置，
// 适配器产物统一放 bin/adapter-<name>。
func defaultAdapterPath(name string) string {
	ext := ""
	if runtime.GOOS == "windows" {
		ext = ".exe"
	}
	return "bin/adapter-" + name + ext
}

type Config struct {
	Plugins struct {
		// Adapters 适配器定义列表。核心为每个适配器维护独立子进程与
		// 断流重启循环，下行（SendMessage/SendAction）按请求里的
		// platform 路由到对应子进程。
		Adapters []AdapterDef `yaml:"adapters"`
	} `yaml:"plugins"`
}

// parseConfig 解析并规范化配置：迁移旧格式、补默认值、查重。
func parseConfig(data []byte) (*Config, error) {
	var lg legacyConfig
	_ = yaml.Unmarshal(data, &lg) // 尽力而为：旧节缺失不致命

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("解析配置: %w", err)
	}
	if len(cfg.Plugins.Adapters) == 0 {
		switch {
		case len(lg.Plugins.Adapters) > 0:
			// 纯旧文件：整表迁移字符串列表
			for _, name := range lg.Plugins.Adapters {
				cfg.Plugins.Adapters = append(cfg.Plugins.Adapters,
					AdapterDef{Name: name, legacy: true})
			}
		case lg.Plugins.Napcat.Path != "" || lg.Napcat.URL != "":
			// 旧版极简写法：只配 napcat 分节不写列表（旧版缺省启用 napcat），
			// 迁移保持原语义
			cfg.Plugins.Adapters = []AdapterDef{{Name: "napcat", legacy: true}}
		default:
			// 全新文件：缺省沙箱（沙盒优先：开箱即用，不要求任何平台参数）
			cfg.Plugins.Adapters = []AdapterDef{{Name: "sandbox"}}
		}
	}
	for i := range cfg.Plugins.Adapters {
		cfg.Plugins.Adapters[i].normalize(lg)
	}
	if err := validateDefs(cfg.Plugins.Adapters); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// validateDefs 定义列表的基本校验：name 必填且不重复。
// path 与 env 的含义由适配器解释，核心不校验内容。
func validateDefs(defs []AdapterDef) error {
	seen := map[string]bool{}
	for _, d := range defs {
		if d.Name == "" {
			return errors.New("适配器定义缺 name")
		}
		if seen[d.Name] {
			return fmt.Errorf("适配器 %q 重复", d.Name)
		}
		seen[d.Name] = true
	}
	return nil
}

// enabledAdapters 启用列表：剔除 enabled=false，去重保序（后者胜）。
// 调用方持 cfgMu（热更与读取互斥）。
func (c *Config) enabledAdapters() []AdapterDef {
	seen := map[string]bool{}
	var out []AdapterDef
	for _, d := range c.Plugins.Adapters {
		if !d.IsEnabled() || seen[d.Name] {
			continue
		}
		seen[d.Name] = true
		out = append(out, d)
	}
	return out
}

// adapterDef 按名字取定义（含停用项）。调用方持 cfgMu。
func (c *Config) adapterDef(name string) (AdapterDef, bool) {
	for _, d := range c.Plugins.Adapters {
		if d.Name == name {
			return d, true
		}
	}
	return AdapterDef{}, false
}

// names 适配器名列表，日志用。
func names(defs []AdapterDef) []string {
	out := make([]string, len(defs))
	for i, d := range defs {
		out[i] = d.Name
	}
	return out
}

// loadConfig 读取并校验配置；文件不存在时从模板生成一份并直接使用
// （模板缺省沙箱，无需任何必填项即可启动）。第二返回值标记首运生成。
func loadConfig(path string) (*Config, bool, error) {
	data, err := os.ReadFile(path)
	firstRun := false
	if errors.Is(err, os.ErrNotExist) {
		// 首次运行：从模板生成一份，用户有现成文件可改
		if genErr := generateFromExample(path); genErr != nil {
			return nil, false, fmt.Errorf("读取配置 %s: %w（且 %v）", path, err, genErr)
		}
		firstRun = true
		data, err = os.ReadFile(path)
	}
	if err != nil {
		return nil, false, err
	}
	cfg, err := parseConfig(data)
	if err != nil {
		return nil, false, err
	}
	return cfg, firstRun, nil
}

// generateFromExample 用 <配置路径>.example 模板生成配置文件。
func generateFromExample(path string) error {
	data, err := os.ReadFile(path + ".example")
	if err != nil {
		return fmt.Errorf("找不到配置模板: %w", err)
	}
	return os.WriteFile(path, data, 0o644)
}
