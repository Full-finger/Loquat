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
	"fmt"
	"os"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/hashicorp/go-hclog"

	"github.com/full-finger/loquat/internal/core"
)

var started atomic.Bool

// c-shared 要求 package main 且有 main，但不会被调用。
func main() {}

//export LoquatStart
func LoquatStart(configPath *C.char, cb C.loquat_event_cb) C.int {
	if !started.CompareAndSwap(false, true) {
		return 1 // 已启动
	}
	logger := hclog.New(&hclog.LoggerOptions{
		Output: os.Stderr,
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
	var out string
	if err != nil {
		out = fmt.Sprintf(`{"error":%q}`, err.Error())
	} else {
		b, merr := json.Marshal(resp)
		if merr != nil {
			out = fmt.Sprintf(`{"error":%q}`, merr.Error())
		} else {
			out = string(b)
		}
	}
	return C.CString(out)
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
		out = fmt.Sprintf(`{"error":%q}`, err.Error())
	}
	return C.CString(out)
}
