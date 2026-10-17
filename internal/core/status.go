// 系统状态采集：宿主机指标 + 核心自身运行状态。
// 作为核心能力经 FFI 暴露，最终由 Rust 注入 Python 供业务格式化。
package core

import (
	"encoding/json"
	"errors"
	"fmt"
	"runtime"
	"time"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/disk"
	"github.com/shirou/gopsutil/v4/host"
	"github.com/shirou/gopsutil/v4/mem"
)

// SystemStatusJSON 采集一次系统状态并返回 JSON。
// CPU 占用需要 200ms 采样窗口，调用方容忍该延迟。
func SystemStatusJSON() (string, error) {
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

	payload := map[string]any{
		"os":                hi.Platform,
		"arch":              runtime.GOARCH,
		"uptime_hours":      float64(hi.Uptime) / 3600,
		"cpu_percent":       cpuPercent[0],
		"mem_total_gb":      float64(vm.Total) / 1e9,
		"mem_used_percent":  vm.UsedPercent,
		"disk_used_percent": du.UsedPercent,
		"goroutines":        runtime.NumGoroutine(),
		"go_heap_mb":        float64(ms.HeapAlloc) / 1e6,
	}
	b, err := json.Marshal(payload)
	if err != nil {
		return "", err
	}
	return string(b), nil
}
