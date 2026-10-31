package core

// InjectEvent 的集成测试：segments JSON 解析与按平台路由。
// 真实拉起适配器子进程（拿 adapter-napcat 二进制当替身，go-plugin 不校验
// 平台名），验证 WebUI 透传链路的核心侧：JSON → proto 段 → 适配器应答。

import (
	"io"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/hashicorp/go-hclog"
)

func TestInjectEventWithSegments(t *testing.T) {
	if sandboxBin == "" {
		t.Skipf("沙箱适配器构建失败，跳过: %v", adapterBuildErr)
	}
	if testing.Short() {
		t.Skip("集成测试，-short 跳过")
	}

	cfgDir, err := os.MkdirTemp("", "loquat-cfg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(cfgDir)
	// 通用定义列表（新格式）：直接用真 adapter-sandbox
	cfg := filepath.Join(cfgDir, "loquat.yaml")
	content := "plugins:\n  adapters:\n    - name: sandbox\n      path: " +
		filepath.ToSlash(sandboxBin) + "\n"
	if err := os.WriteFile(cfg, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}

	logger := hclog.New(&hclog.LoggerOptions{Output: io.Discard, Level: hclog.Error})
	if err := Start(cfg, func([]byte) {}, logger); err != nil {
		t.Fatalf("核心启动失败: %v", err)
	}
	defer Stop()
	waitRuntimeReady(t, "sandbox")

	// 正常段注入：带回复与文本段，应答回消息 ID
	resp, err := InjectEvent("sandbox",
		`[{"reply":{"messageId":"9"}},{"text":{"text":"hi"}}]`, false, "private", "10000.t1")
	if err != nil {
		t.Fatalf("注入失败: %v", err)
	}
	if resp.GetMessageId() == "" {
		t.Fatal("应答应带消息 ID")
	}

	// 坏 JSON 报错，不静默降级
	if _, err := InjectEvent("sandbox", `[{bad`, false, "private", ""); err == nil {
		t.Fatal("坏 JSON 应报错")
	}
	// 未知平台路由报错
	if _, err := InjectEvent("nope", `[{"text":{"text":"x"}}]`, false, "private", ""); err == nil {
		t.Fatal("未启用平台应报错")
	}
}

// waitRuntimeReady 等平台的插件 runtime 就绪（spawn 异步握手）。
func waitRuntimeReady(t *testing.T, platform string) {
	t.Helper()
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		if _, ok := active.getRuntime(platform); ok {
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("平台 %s 的 runtime 未就绪", platform)
}
