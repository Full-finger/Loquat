package core

import (
	"os"
	"path/filepath"
	"testing"
)

// TestDiscoverAdaptersIn 发现流扫描：adapter-<name>[.exe] 收录，
// 编辑器备份（~）、非适配器文件、目录排除；结果按名字排序。
func TestDiscoverAdaptersIn(t *testing.T) {
	dir := t.TempDir()
	for _, f := range []string{
		"adapter-napcat.exe", "adapter-sandbox", "adapter-sandbox.exe~",
		"adapter-.exe", "core.exe", "notes.txt",
	} {
		if err := os.WriteFile(filepath.Join(dir, f), []byte("x"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.Mkdir(filepath.Join(dir, "adapter-dir"), 0o755); err != nil {
		t.Fatal(err)
	}

	got := discoverAdaptersIn(dir)
	want := []discoveredAdapter{
		{Name: "napcat", Path: dir + "/adapter-napcat.exe"},
		{Name: "sandbox", Path: dir + "/adapter-sandbox"},
	}
	if len(got) != len(want) {
		t.Fatalf("发现数 = %d（%v），要 %d", len(got), got, len(want))
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("第 %d 项 = %+v，要 %+v", i, got[i], want[i])
		}
	}
}

// TestDiscoverAdaptersInEmpty 目录不存在（未编译产物）视为空，不报错。
func TestDiscoverAdaptersInEmpty(t *testing.T) {
	if got := discoverAdaptersIn(filepath.Join(t.TempDir(), "nope")); got != nil {
		t.Errorf("不存在的目录应返回 nil，得到 %v", got)
	}
}

// TestResolvedPath 显式 path 优先，留空走 bin/adapter-<name> 约定。
func TestResolvedPath(t *testing.T) {
	if got := resolvedPath(AdapterDef{Name: "x", Path: "custom/x.exe"}); got != "custom/x.exe" {
		t.Errorf("显式 path = %q", got)
	}
	if got := resolvedPath(AdapterDef{Name: "x"}); got != defaultAdapterPath("x") {
		t.Errorf("缺省 path = %q，要 %q", got, defaultAdapterPath("x"))
	}
}
