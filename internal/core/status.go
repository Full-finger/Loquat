// 系统状态采集：宿主机指标 + 核心自身运行状态。
// 作为核心能力经 FFI 暴露，最终由 Rust 注入 Python 供业务格式化。
//
// 口径说明（四层）：
//
//	机器级：os/uptime/cpu/mem/disk —— 整台 Windows，含所有进程
//	宿主进程：Rust 宿主 + Go 核心 dll + CPython 共享同一进程，RSS 为三者总和
//	适配器子进程：独立进程，单独计量
//	Go 核心：仅 dll 内 Go 运行时的 goroutine 与堆
package core

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"runtime"
	"time"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/disk"
	"github.com/shirou/gopsutil/v4/host"
	"github.com/shirou/gopsutil/v4/mem"
	"github.com/shirou/gopsutil/v4/process"
)

// SystemStatusJSON 汇集系统状态并返回 JSON。适配器指标取全部已启用平台。
func SystemStatusJSON() (string, error) {
	mu.Lock()
	c := active
	mu.Unlock()

	var adapterPIDs []int
	if c != nil {
		c.runtimesMu.Lock()
		for _, rt := range c.runtimes {
			adapterPIDs = append(adapterPIDs, rt.pid)
		}
		c.runtimesMu.Unlock()
	}
	return collectStatus(adapterPIDs...)
}

func collectStatus(adapterPIDs ...int) (string, error) {
	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)

	cpuPercent, err := cpu.Percent(200*time.Millisecond, false)
	if err != nil {
		return "", fmt.Errorf("cpu: %w", err)
	}
	// 某些虚拟化/容器环境可能返回空切片，直接下标会 panic（清单 #9）
	if len(cpuPercent) == 0 {
		return "", errors.New("cpu 采样结果为空")
	}
	vm, err := mem.VirtualMemory()
	if err != nil {
		return "", fmt.Errorf("mem: %w", err)
	}
	hi, err := host.Info()
	if err != nil {
		return "", fmt.Errorf("host: %w", err)
	}
	du, err := disk.Usage(".")
	if err != nil {
		return "", fmt.Errorf("disk: %w", err)
	}

	// 宿主进程 RSS：Rust 宿主 + Go 核心 dll + CPython 三者同进程，合计口径
	var hostRSS float64
	if self, err := process.NewProcess(int32(os.Getpid())); err == nil {
		if mi, err := self.MemoryInfo(); err == nil {
			hostRSS = float64(mi.RSS) / 1e6
		}
	}

	// 适配器进程：合计各平台子进程 RSS；不存在（未启动/已退出）时为 nil，
	// 业务侧显示"未运行"
	var adapterRSS any
	var totalRSS float64
	for _, pid := range adapterPIDs {
		if ap, err := process.NewProcess(int32(pid)); err == nil {
			if mi, err := ap.MemoryInfo(); err == nil {
				totalRSS += float64(mi.RSS) / 1e6
			}
		}
	}
	if totalRSS > 0 {
		adapterRSS = totalRSS
	}

	payload := map[string]any{
		"os":                hi.Platform,
		"arch":              runtime.GOARCH,
		"uptime_hours":      float64(hi.Uptime) / 3600,
		"cpu_percent":       cpuPercent[0],
		"mem_total_gb":      float64(vm.Total) / 1e9,
		"mem_used_percent":  vm.UsedPercent,
		"disk_used_percent": du.UsedPercent,
		"host_rss_mb":       hostRSS,
		"adapter_rss_mb":    adapterRSS,
		"goroutines":        runtime.NumGoroutine(),
		"go_heap_mb":        float64(ms.HeapAlloc) / 1e6,
	}
	b, err := json.Marshal(payload)
	if err != nil {
		return "", err
	}
	return string(b), nil
}
