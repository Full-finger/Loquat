package core

import (
	"encoding/json"
	"testing"
)

// 口径核对：字段齐全、宿主进程 RSS 有值（本进程必然存活）。
// 适配器未启动时 adapter_rss_mb 应为 null。
func TestSystemStatusJSON(t *testing.T) {
	out, err := SystemStatusJSON()
	if err != nil {
		t.Fatalf("SystemStatusJSON: %v", err)
	}
	var m map[string]any
	if err := json.Unmarshal([]byte(out), &m); err != nil {
		t.Fatalf("输出不是合法 JSON: %v\n%s", err, out)
	}

	for _, k := range []string{
		"os", "arch", "uptime_hours", "cpu_percent", "mem_total_gb",
		"mem_used_percent", "disk_used_percent",
		"host_rss_mb", "adapter_rss_mb", "goroutines", "go_heap_mb",
	} {
		if _, ok := m[k]; !ok {
			t.Errorf("缺少字段 %s", k)
		}
	}

	if hr, ok := m["host_rss_mb"].(float64); !ok || hr <= 0 {
		t.Errorf("host_rss_mb 应为正数（宿主进程必然存活），实际 %v", m["host_rss_mb"])
	}
	if arv, ok := m["adapter_rss_mb"]; ok && arv != nil {
		if ar, ok := arv.(float64); !ok || ar <= 0 {
			t.Errorf("adapter_rss_mb 非法: %v", arv)
		}
	}
}
