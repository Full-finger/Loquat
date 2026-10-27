package core

// 集成测试：验证插件运行时原子换装的并发安全（checkList/26101701.md 第 1 项）。
// 真实拉起 adapter-napcat 子进程（WS 指向空端口，连接失败不影响握手与换装逻辑），
// 主线程反复触发适配器替换，后台并发轰 Status / SendAction，
// 需配合竞态检测器运行：go test -race ./internal/core/
// 对旧实现（client/adapter 散装字段读写）本测试可复现竞争。

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
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
	if runtime.GOOS == "windows" {
		bin += ".exe" // go build -o 在 Windows 上不会自动补扩展名
	}
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
			_, _ = SendAction("napcat", "send_private_msg", []byte(`{}`), time.Second)
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

// TestStopDuringRespawnNoOrphan（26102202#2）：Stop 撞上退避重启路径时，
// 飞行中的 spawn 不许留下孤儿子进程。收集存活期出现过的适配器 PID，
// Stop 返回（run/consume 已收口）后逐一确认进程退出。
func TestStopDuringRespawnNoOrphan(t *testing.T) {
	if adapterBin == "" {
		t.Skipf("适配器构建失败，跳过: %v", adapterBuildErr)
	}
	if testing.Short() {
		t.Skip("集成测试，-short 跳过")
	}

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

	logger := hclog.New(&hclog.LoggerOptions{Output: io.Discard, Level: hclog.Error})
	if err := Start(cfg, func([]byte) {}, logger); err != nil {
		t.Fatalf("核心启动失败: %v", err)
	}

	// 后台定期抠 Status 里的 adapter_pid，记录出现过的所有适配器进程
	var pidMu sync.Mutex
	pids := map[int]bool{}
	stopCollect := make(chan struct{})
	go func() {
		for {
			select {
			case <-stopCollect:
				return
			default:
			}
			var snap struct {
				Adapters []struct {
					PID int `json:"pid"`
				} `json:"adapters"`
			}
			if err := json.Unmarshal([]byte(Status()), &snap); err == nil {
				for _, a := range snap.Adapters {
					if a.PID > 0 {
						pidMu.Lock()
						pids[a.PID] = true
						pidMu.Unlock()
					}
				}
			}
			time.Sleep(50 * time.Millisecond)
		}
	}()

	// 触发若干次重启，让 spawn 反复飞行；不等于 Stop 打点同步，
	// 让 Stop 自然落在 spawn 飞行中的概率窗口里
	for i := 0; i < 3; i++ {
		if err := RestartAdapters(); err != nil {
			t.Fatalf("RestartAdapters: %v", err)
		}
		time.Sleep(300 * time.Millisecond)
	}

	Stop() // 内部等 run/consume 收口，返回即表明 wg 已清
	close(stopCollect)

	pidMu.Lock()
	defer pidMu.Unlock()
	if len(pids) == 0 {
		t.Fatal("整个生命周期没观测到适配器 PID，采集逻辑失效")
	}
	// 子进程退出是异步的，给一个宽限窗口轮询
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		allGone := true
		for pid := range pids {
			if processAlive(pid) {
				allGone = false
				break
			}
		}
		if allGone {
			t.Logf("全部 %d 个适配器实例均已退出", len(pids))
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	for pid := range pids {
		if processAlive(pid) {
			t.Errorf("孤儿进程: 适配器 PID %d 在 Stop 后仍存活", pid)
		}
	}
}

// processAlive 探测进程是否存活。Windows 走 tasklist，Unix 走 pgrep。
func processAlive(pid int) bool {
	if runtime.GOOS == "windows" {
		out, err := exec.Command("tasklist", "/FI",
			fmt.Sprintf("PID eq %d", pid), "/NH").Output()
		return err == nil && strings.Contains(string(out), strconv.Itoa(pid))
	}
	out, err := exec.Command("pgrep", "-p", strconv.Itoa(pid)).Output()
	return err == nil && len(out) > 0
}
