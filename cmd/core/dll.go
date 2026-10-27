//go:build dll

// Go 核心的 c-shared 形态：导出 C ABI 给 Rust 宿主。
// 约定：Start 传配置路径和事件回调；事件以 JSON 字节经回调上行；
// 动作经 SendAction 同步下行。
// 内存归属（嵌入方必读）：LoquatSendAction / LoquatStatus /
// LoquatSystemStatus 三个导出都返回 malloc 分配的 C 字符串，
// 调用方用完必须逐个调 LoquatFree 归还；LoquatLogPipe 返回的读端 fd
// 由宿主持有（进程级一次），无需归还。
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
	"io"
	"os"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/hashicorp/go-hclog"
	"google.golang.org/protobuf/encoding/protojson"

	"github.com/full-finger/loquat/internal/core"
	"github.com/full-finger/loquat/internal/pluginapi"
)

var started atomic.Bool

// 日志管道：核心的 hclog 写入写端，读端句柄交给宿主泵出。
// 三边日志收敛到宿主的 stderr 锁下单一写者，避免并发绞行。
// sync.Once 保证并发调用只建一根管道（26102202#6）。
var (
	logPipeOnce  sync.Once
	logPipeRead  *os.File
	logPipeWrite *os.File
)

//export LoquatLogPipe
func LoquatLogPipe() C.uintptr_t {
	logPipeOnce.Do(func() {
		pr, pw, err := os.Pipe()
		if err != nil {
			return
		}
		logPipeRead, logPipeWrite = pr, pw
	})
	if logPipeRead == nil {
		return 0
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
func LoquatSendAction(platform *C.char, action *C.char, params *C.char, timeoutMs C.int) *C.char {
	resp, err := core.SendAction(
		C.GoString(platform),
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

//export LoquatSendMessage
func LoquatSendMessage(platform *C.char, chatKind *C.char, chatID *C.char, segments *C.char, timeoutMs C.int) *C.char {
	req := &pluginapi.SendMessageRequest{
		Platform: C.GoString(platform),
		Chat:     &pluginapi.ChatRef{},
	}
	switch C.GoString(chatKind) {
	case "private":
		req.Chat.Kind = &pluginapi.ChatRef_PrivateId{PrivateId: C.GoString(chatID)}
	case "group":
		req.Chat.Kind = &pluginapi.ChatRef_GroupId{GroupId: C.GoString(chatID)}
	default:
		return marshalFFIError(fmt.Sprintf("未知 chat kind: %s", C.GoString(chatKind)))
	}

	var rawSegs []json.RawMessage
	if err := json.Unmarshal([]byte(C.GoString(segments)), &rawSegs); err != nil {
		return marshalFFIError("segments 不是 JSON 数组: " + err.Error())
	}
	for i, raw := range rawSegs {
		var seg pluginapi.Segment
		// 段是 protojson 格式：{"text": {"text": "..."}}；错误带上序号好定位
		if err := (protojson.UnmarshalOptions{}).Unmarshal(raw, &seg); err != nil {
			return marshalFFIError(fmt.Sprintf("第 %d 个消息段解析失败: %v", i, err))
		}
		req.Segments = append(req.Segments, &seg)
	}

	resp, err := core.SendMessage(req, time.Duration(timeoutMs)*time.Millisecond)
	if err != nil {
		return marshalFFIError(err.Error())
	}
	// protojson 输出稳定字段名（messageId），宿主按同一约定解
	b, merr := protojson.Marshal(resp)
	if merr != nil {
		return marshalFFIError(merr.Error())
	}
	return C.CString(string(b))
}

func marshalFFIError(msg string) *C.char {
	b, _ := json.Marshal(map[string]string{"error": msg})
	return C.CString(string(b))
}

//export LoquatInjectEvent
func LoquatInjectEvent(platform *C.char, text *C.char, atBot C.int, mode *C.char) *C.char {
	if err := core.InjectEvent(C.GoString(platform), C.GoString(text), atBot != 0, C.GoString(mode)); err != nil {
		return marshalFFIError(err.Error())
	}
	return C.CString(`{"ok":true}`)
}

//export LoquatAdaptersConfig
func LoquatAdaptersConfig() *C.char {
	return C.CString(core.AdaptersConfigJSON())
}

//export LoquatApplyAdaptersConfig
func LoquatApplyAdaptersConfig(configJSON *C.char) *C.char {
	msg, err := core.ApplyAdaptersConfig(C.GoString(configJSON))
	if err != nil {
		return marshalFFIError(err.Error())
	}
	b, _ := json.Marshal(map[string]string{"msg": msg})
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
