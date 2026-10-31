package core

import (
	"os"
	"testing"
)

// 配置解析与迁移的单元测试：旧格式兼容、缺省沙箱、新格式往返。

// TestParseConfigLegacyMigrate 旧格式文件（平台分节 + 字符串 adapters）
// 必须无感迁移：napcat 的 path 与 url/token 参数不丢。
func TestParseConfigLegacyMigrate(t *testing.T) {
	data := []byte(`napcat:
  url: ws://127.0.0.1:3001
  token: abc
plugins:
  adapters:
    - napcat
    - sandbox
  napcat:
    path: bin/adapter-napcat.exe
  sandbox:
    path: bin/adapter-sandbox.exe
`)
	cfg, err := parseConfig(data)
	if err != nil {
		t.Fatalf("parseConfig: %v", err)
	}
	if len(cfg.Plugins.Adapters) != 2 {
		t.Fatalf("适配器数量 = %d，要 2", len(cfg.Plugins.Adapters))
	}
	napcat := cfg.Plugins.Adapters[0]
	if napcat.Name != "napcat" ||
		napcat.Path != "bin/adapter-napcat.exe" ||
		napcat.Env["LOQUAT_NAPCAT_URL"] != "ws://127.0.0.1:3001" ||
		napcat.Env["LOQUAT_NAPCAT_TOKEN"] != "abc" {
		t.Errorf("napcat 迁移不完整: %+v", napcat)
	}
	if !napcat.IsEnabled() {
		t.Error("缺省应启用")
	}
	sandbox := cfg.Plugins.Adapters[1]
	if sandbox.Path != "bin/adapter-sandbox.exe" {
		t.Errorf("sandbox path = %q", sandbox.Path)
	}
	if sandbox.Env != nil {
		t.Errorf("sandbox 不应有参数: %v", sandbox.Env)
	}
}

// TestParseConfigDefaultSandbox 无 adapters 键的新文件缺省沙箱，
// path 缺省按名字猜（bin/adapter-<name>.exe）。
func TestParseConfigDefaultSandbox(t *testing.T) {
	data := []byte("plugins: {}\n")
	cfg, err := parseConfig(data)
	if err != nil {
		t.Fatalf("parseConfig: %v", err)
	}
	defs := cfg.Plugins.Adapters
	if len(defs) != 1 || defs[0].Name != "sandbox" {
		t.Fatalf("缺省适配器 = %v，要 [sandbox]", names(defs))
	}
	if defs[0].Path == "" {
		t.Error("缺省 path 不应为空")
	}
}

// TestParseConfigLegacyMinimalNapcat 旧版极简写法：只配 napcat 分节不写
// adapters 列表（旧版缺省启用 napcat），迁移必须保持原语义。
func TestParseConfigLegacyMinimalNapcat(t *testing.T) {
	data := []byte("napcat:\n  url: ws://127.0.0.1:1\nplugins:\n  napcat:\n    path: bin/x.exe\n")
	cfg, err := parseConfig(data)
	if err != nil {
		t.Fatalf("parseConfig: %v", err)
	}
	defs := cfg.enabledAdapters()
	if len(defs) != 1 || defs[0].Name != "napcat" || defs[0].Path != "bin/x.exe" {
		t.Fatalf("极简 napcat 迁移错: %+v", defs)
	}
}

// TestParseConfigNewFormat 新格式完整能力：停用项、自定义平台、查重。
func TestParseConfigNewFormat(t *testing.T) {
	data := []byte(`plugins:
  adapters:
    - name: sandbox
      enabled: false
    - name: telegram
      path: bin/adapter-telegram
      env:
        TELEGRAM_TOKEN: t-123
`)
	cfg, err := parseConfig(data)
	if err != nil {
		t.Fatalf("parseConfig: %v", err)
	}
	defs := cfg.Plugins.Adapters
	if len(defs) != 2 {
		t.Fatalf("适配器数量 = %d，要 2（停用项保留）", len(defs))
	}
	if defs[0].IsEnabled() {
		t.Error("sandbox 应停用")
	}
	if defs[1].Name != "telegram" || defs[1].Env["TELEGRAM_TOKEN"] != "t-123" {
		t.Errorf("自定义平台解析错: %+v", defs[1])
	}
	// 自定义平台不因平台名被拒——核心零平台知识
	enabled := cfg.enabledAdapters()
	if len(enabled) != 1 || enabled[0].Name != "telegram" {
		t.Errorf("启用列表 = %v，要 [telegram]", names(enabled))
	}
}

func TestParseConfigDuplicateName(t *testing.T) {
	data := []byte(`plugins:
  adapters:
    - name: a
    - name: a
`)
	if _, err := parseConfig(data); err == nil {
		t.Fatal("重名应报错")
	}
}

// TestAdapterDefEqual Equal 覆盖 path/env/enabled 三个生效字段。
func TestAdapterDefEqual(t *testing.T) {
	a := AdapterDef{Name: "x", Path: "p", Env: map[string]string{"K": "v"}}
	b := AdapterDef{Name: "x", Path: "p", Env: map[string]string{"K": "v"}}
	if !a.Equal(b) {
		t.Fatal("相同定义应 Equal")
	}
	b.Env["K2"] = "v2"
	if a.Equal(b) {
		t.Fatal("env 多一个键不应 Equal")
	}
	b.Env = map[string]string{"K": "other"}
	if a.Equal(b) {
		t.Fatal("env 值不同不应 Equal")
	}
	b.Env = nil
	off := false
	b.Enabled = &off
	if a.Equal(b) {
		t.Fatal("启用状态不同不应 Equal")
	}
	b.Env = map[string]string{"K": "v"}
	b.Enabled = nil
	if !a.Equal(b) {
		t.Fatal("回到相同定义应 Equal")
	}
}

// TestWriteAdapterConfigRoundTrip 写盘→读回：定义不丢、旧键残留不影响。
func TestWriteAdapterConfigRoundTrip(t *testing.T) {
	c := &Core{configPath: t.TempDir() + "/loquat.yaml"}
	// 旧格式底子：残留键读时被新列表压制
	old := []byte(`napcat:
  url: ws://old:1
plugins:
  adapters:
    - napcat
  napcat:
    path: bin/adapter-napcat.exe
`)
	if err := os.WriteFile(c.configPath, old, 0o644); err != nil {
		t.Fatal(err)
	}
	next := &adapterConfig{Adapters: []AdapterDef{
		{Name: "sandbox", Path: "bin/adapter-sandbox.exe"},
		{Name: "napcat", Path: "bin/adapter-napcat.exe",
			Env: map[string]string{"LOQUAT_NAPCAT_URL": "ws://new:2"}},
	}}
	if err := c.writeAdapterConfig(next); err != nil {
		t.Fatalf("writeAdapterConfig: %v", err)
	}
	data, err := os.ReadFile(c.configPath)
	if err != nil {
		t.Fatal(err)
	}
	cfg, err := parseConfig(data)
	if err != nil {
		t.Fatalf("读回解析失败: %v", err)
	}
	if len(cfg.Plugins.Adapters) != 2 {
		t.Fatalf("写回后适配器数量 = %d", len(cfg.Plugins.Adapters))
	}
	if got := cfg.Plugins.Adapters[1].Env["LOQUAT_NAPCAT_URL"]; got != "ws://new:2" {
		t.Errorf("写回的 env 未生效，读到 %q（残留旧键 ws://old:1 被压制？）", got)
	}
}
