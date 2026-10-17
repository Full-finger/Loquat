package core

// 集成测试：验证插件运行时原子换装的并发安全（checkList/26101701.md 第 1 项）。
// 真实拉起 adapter-napcat 子进程（WS 指向空端口，连接失败不影响握手与换装逻辑），
// 主线程反复触发适配器替换，后台并发轰 Status / SendAction，
// 需配合竞态检测器运行：go test -race ./internal/core/
// 对旧实现（client/adapter 散装字段读写）本测试可复现竞争。

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/hashicorp/go-hclog"
)

var (
	adapterBin      string // 测试用适配器可执行文件路径，空表示构建失败
	adapterBuildErr error
)

// TestMain 构建测试用适配器。构建失败不致命：测试里 skip 并说明原因。
func TestMain(m *testing.M) {
	adapterBin, adapterBuildErr = buildTestAdapter()
	m.Run()
	if adapterBin != "" {
		os.RemoveAll(filepath.Dir(adapterBin))
	}
}

// buildTestAdapter 把 cmd/adapter-napcat 编到临时目录。cwd 是本包目录（internal/core），
// 模块根即 ../..。不依赖网络，依赖均在模块缓存。
func buildTestAdapter() (string, error) {
	if _, err := exec.LookPath("go"); err != nil {
		return "", fmt.Errorf("PATH 中找不到 go 工具链: %w", err)
	}
	dir, err := os.MkdirTemp("", "loquat-adapter-*")
	if err != nil {
		return "", err
	}
	bin := filepath.Join(dir, "adapter-test")
	cmd := exec.Command("go", "build", "-o", bin, "./cmd/adapter-napcat")
	cmd.Dir = "../.."
	if out, err := cmd.CombinedOutput(); err != nil {
		os.RemoveAll(dir)
		return "", fmt.Errorf("构建 adapter-napcat: %w\n%s", err, out)
	}
	return bin, nil
}

func TestRuntimeSwapRace(t *testing.T) {
	if adapterBin == "" {
		t.Skipf("适配器构建失败，跳过: %v", adapterBuildErr)
	}
	if testing.Short() {
		t.Skip("集成测试，-short 跳过")
	}

	// WS 指向本机端口 1（无人监听）：适配器 connectLoop 会重试，
	// 但 go-plugin 握手与核心的换装逻辑照常运转，正是要测的路径
	cfgDir, err := os.MkdirTemp("", "loquat-cfg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(cfgDir)
	cfg := filepath.Join(cfgDir, "loquat.yaml")
	content := fmt.Sprintf("napcat:\n  url: %s\nplugins:\n  napcat:\n    path: %s\n",
		"ws://127.0.0.1:1", adapterBin)
	if err := os.WriteFile(cfg, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}

	// 子进程与核心日志全部吞掉，测试输出只留断言
	logger := hclog.New(&hclog.LoggerOptions{Output: io.Discard, Level: hclog.Error})
	if err := Start(cfg, func([]byte) {}, logger); err != nil {
		t.Fatalf("核心启动失败: %v", err)
	}
	defer Stop()

	const hammer = 5 * time.Second
	deadline := time.Now().Add(hammer)

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		for time.Now().Before(deadline) {
			_ = Status()
		}
	}()
	go func() {
		defer wg.Done()
		for time.Now().Before(deadline) {
			_, _ = SendAction("send_private_msg", []byte(`{}`), time.Second)
		}
	}()
	// 主线程反复触发换装：kill 后 run 循环退避重启 spawn
	for time.Now().Before(deadline) {
		if err := RestartAdapters(); err != nil {
			t.Errorf("RestartAdapters: %v", err)
			break
		}
		time.Sleep(200 * time.Millisecond)
	}
	wg.Wait()
}
