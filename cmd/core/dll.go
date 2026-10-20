//go:build dll

// Go 核心的 c-shared 形态：导出 C ABI 给 Rust 宿主。
// 约定：Start 传配置路径和事件回调；事件以 JSON 字节经回调上行；
// 动作经 SendAction 同步下行，返回 malloc 的 JSON 字符串，调用方须 LoquatFree。
package main

/*
#include <stdlib.h>

typedef void (*loquat_event_cb)(const char* data, int len);

static void loquat_call_cb(loquat_event_cb cb, const char* data, int len) {
	cb(data, len);
}
*/
import "C"

import (
	"encoding/json"
	"io"
	"os"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/hashicorp/go-hclog"

	"github.com/full-finger/loquat/internal/core"
)

var started atomic.Bool

// 日志管道：核心的 hclog 写入写端，读端句柄交给宿主泵出。
// 三边日志收敛到宿主的 stderr 锁下单一写者，避免并发绞行。
var (
	logPipeRead  *os.File
	logPipeWrite *os.File
)

//export LoquatLogPipe
func LoquatLogPipe() C.uintptr_t {
	if logPipeRead == nil {
		pr, pw, err := os.Pipe()
		if err != nil {
			return 0
		}
		logPipeRead, logPipeWrite = pr, pw
	}
	return C.uintptr_t(logPipeRead.Fd())
}

// c-shared 要求 package main 且有 main，但不会被调用。
func main() {}

//export LoquatStart
func LoquatStart(configPath *C.char, cb C.loquat_event_cb) C.int {
	if !started.CompareAndSwap(false, true) {
		return 1 // 已启动
	}
	var out io.Writer = os.Stderr
	if logPipeWrite != nil {
		out = logPipeWrite // 宿主已接管日志泵，走管道统一打印
	}
	logger := hclog.New(&hclog.LoggerOptions{
		Output: out,
		Level:  hclog.Info,
		Name:   "core",
	})
	err := core.Start(C.GoString(configPath), func(payload []byte) {
		cdata := C.CString(string(payload))
		defer C.free(unsafe.Pointer(cdata))
		C.loquat_call_cb(cb, cdata, C.int(len(payload)))
	}, logger)
	if err != nil {
		started.Store(false)
		logger.Error("核心启动失败", "err", err)
		return 2
	}
	return 0
}

//export LoquatSendAction
func LoquatSendAction(action *C.char, params *C.char, timeoutMs C.int) *C.char {
	resp, err := core.SendAction(
		C.GoString(action),
		[]byte(C.GoString(params)),
		time.Duration(timeoutMs)*time.Millisecond,
	)
	// 错误与应答统一走 json.Marshal：%q 可能产出 \xNN 等非法 JSON 转义，
	// Rust 侧 serde_json 解析会失败（清单 #6）。
	var payload any = resp
	if err != nil {
		payload = map[string]string{"error": err.Error()}
	}
	// Data 字段是 []byte，会序列化为 base64 字符串；当前仅用于展示，
	// 需要原始字节时改用 protojson（清单 #7）。
	b, merr := json.Marshal(payload)
	if merr != nil {
		b, _ = json.Marshal(map[string]string{"error": merr.Error()})
	}
	return C.CString(string(b))
}

//export LoquatStop
func LoquatStop() {
	core.Stop()
	started.Store(false)
}

//export LoquatFree
func LoquatFree(p *C.char) {
	C.free(unsafe.Pointer(p))
}

//export LoquatReloadPlugin
func LoquatReloadPlugin(name *C.char) C.int {
	// 当前只有一个适配器，name 预留给多插件场景
	if err := core.RestartAdapters(); err != nil {
		return 1
	}
	return 0
}

//export LoquatStatus
func LoquatStatus() *C.char {
	return C.CString(core.Status())
}

//export LoquatSystemStatus
func LoquatSystemStatus() *C.char {
	out, err := core.SystemStatusJSON()
	if err != nil {
		b, _ := json.Marshal(map[string]string{"error": err.Error()})
		return C.CString(string(b))
	}
	return C.CString(out)
}
