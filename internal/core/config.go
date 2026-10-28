// 核心配置：loquat.yaml 的结构与校验、适配器路径/环境变量解析。
// 从 core.go 按职责分出：生命周期在 core.go，下行动作在 actions.go。
package core

import (
	"errors"
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	Napcat struct {
		URL   string `yaml:"url"`
		Token string `yaml:"token"`
	} `yaml:"napcat"`
	Plugins struct {
		// Adapters 拉起的适配器列表，如 [napcat, sandbox]。
		// 核心为每个适配器维护独立子进程与断流重启循环，
		// 下行（SendMessage/SendAction）按请求里的 platform 路由。
		Adapters []string `yaml:"adapters"`
		Napcat   struct {
			Path string `yaml:"path"`
		} `yaml:"napcat"`
		Sandbox struct {
			Path string `yaml:"path"`
			Addr string `yaml:"addr"` // 聊天页监听地址（已并入 webui，此字段预留）
		} `yaml:"sandbox"`
	} `yaml:"plugins"`
}

// enabledAdapters 归一化适配器列表：缺省 [napcat]（兼容旧配置），去重保序。
// 调用方持 cfgMu（热更与读取互斥）。
func (c *Config) enabledAdapters() []string {
	seen := map[string]bool{}
	var out []string
	for _, name := range c.Plugins.Adapters {
		if name != "napcat" && name != "sandbox" {
			continue // loadConfig 已报错，这里兜底跳过
		}
		if !seen[name] {
			seen[name] = true
			out = append(out, name)
		}
	}
	if len(out) == 0 {
		out = []string{"napcat"}
	}
	return out
}

// adapterPath 平台适配器的可执行文件路径。调用方持 cfgMu。
func (cfg *Config) adapterPath(platform string) string {
	if platform == "sandbox" {
		return cfg.Plugins.Sandbox.Path
	}
	return cfg.Plugins.Napcat.Path
}

// adapterEnv 平台适配器的环境变量通道。调用方持 cfgMu。
func (cfg *Config) adapterEnv(platform string) []string {
	if platform == "sandbox" {
		addr := cfg.Plugins.Sandbox.Addr
		if addr == "" {
			addr = "127.0.0.1:7902"
		}
		return []string{"LOQUAT_SANDBOX_ADDR=" + addr}
	}
	return []string{
		"LOQUAT_NAPCAT_URL=" + cfg.Napcat.URL,
		"LOQUAT_NAPCAT_TOKEN=" + cfg.Napcat.Token,
	}
}

// ErrFirstRun 标记首运引导：模板已生成，等用户填 token 重启。
// 非故障，调用方（exe 自测入口）按 Info 级提示而非 Error（26102202#10）。
var ErrFirstRun = errors.New("first-run bootstrap")

// loadConfig 读取并校验配置；文件不存在时从模板生成一份并返回
// ErrFirstRun 标记的引导错误。
func loadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		// 首次运行：从模板生成一份，让用户有现成文件可填
		if genErr := generateFromExample(path); genErr != nil {
			return nil, fmt.Errorf("读取配置 %s: %w（且 %v）", path, err, genErr)
		}
		return nil, fmt.Errorf("已从模板生成 %s，请填入真实 token 后重启: %w", path, ErrFirstRun)
	}
	if err != nil {
		return nil, err
	}
	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("解析 %s: %w", path, err)
	}
	// 按启用的适配器校验必填项：不用到的平台不要求配置
	if len(cfg.Plugins.Adapters) == 0 {
		cfg.Plugins.Adapters = []string{"napcat"} // 缺省 napcat，兼容旧配置
	}
	for _, name := range cfg.Plugins.Adapters {
		switch name {
		case "sandbox":
			if cfg.Plugins.Sandbox.Path == "" {
				return nil, fmt.Errorf("配置缺项：plugins.sandbox.path 必填（adapters 含 sandbox）")
			}
		case "napcat":
			if cfg.Napcat.URL == "" || cfg.Plugins.Napcat.Path == "" {
				return nil, fmt.Errorf("配置缺项：napcat.url 与 plugins.napcat.path 必填")
			}
		default:
			return nil, fmt.Errorf("未知适配器 %q（支持：napcat、sandbox）", name)
		}
	}
	return &cfg, nil
}

// generateFromExample 用 <配置路径>.example 模板生成配置文件。
func generateFromExample(path string) error {
	data, err := os.ReadFile(path + ".example")
	if err != nil {
		return fmt.Errorf("找不到配置模板: %w", err)
	}
	return os.WriteFile(path, data, 0o644)
}
