package main

// cmd/adapter-napcat 单测：dispatch 分流、authFailed 判定、head 截断、
// 写路径 deadline（清单 26102202#3 #5）。核心解析全是纯函数，表驱动直测；
// 写超时用本机 WS 服务端（accept 后不读）模拟对端半死。

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gorilla/websocket"
	"github.com/hashicorp/go-hclog"

	"github.com/full-finger/loquat/internal/pluginapi"
)

func quietLogger() hclog.Logger {
	return hclog.New(&hclog.LoggerOptions{Output: io.Discard, Level: hclog.Error})
}

func TestDispatchRoutesEventsAndResponses(t *testing.T) {
	n := newNapCat("ws://127.0.0.1:1", "", quietLogger())

	// 事件（带 post_type）→ events 通道
	n.dispatch([]byte(`{"post_type":"message","raw_message":"hi"}`))
	select {
	case ev := <-n.events:
		if string(ev.Payload) != `{"post_type":"message","raw_message":"hi"}` {
			t.Fatalf("事件载荷不符: %s", ev.Payload)
		}
	default:
		t.Fatal("事件未入队")
	}

	// 动作响应（带 echo）→ 路由到 pending 注册的通道
	ch := make(chan *pluginapi.SendActionResponse, 1)
	n.pmu.Lock()
	n.pending["e1"] = ch
	n.pmu.Unlock()
	n.dispatch([]byte(`{"status":"ok","retcode":0,"data":{"message_id":7},"echo":"e1"}`))
	select {
	case resp := <-ch:
		if resp.Status != "ok" || resp.Retcode != 0 {
			t.Fatalf("响应字段不符: %+v", resp)
		}
		if string(resp.Data) != `{"message_id":7}` {
			t.Fatalf("data 不符: %s", resp.Data)
		}
	default:
		t.Fatal("响应未路由到 pending 通道")
	}

	// 未知 echo：无注册通道，静默丢弃不 panic
	n.dispatch([]byte(`{"status":"ok","echo":"nobody"}`))

	// 非 JSON / 无 post_type 无 echo：忽略
	n.dispatch([]byte(`not json`))
	n.dispatch([]byte(`{"foo":1}`))
}

func TestAuthFailed(t *testing.T) {
	cases := []struct {
		name string
		data string
		want bool
	}{
		{"failed 且无 echo（retcode 1403 帧）", `{"status":"failed","retcode":1403}`, true},
		{"failed 但带 echo（动作失败，非鉴权）", `{"status":"failed","echo":"e1"}`, false},
		{"非 failed", `{"status":"ok"}`, false},
		{"事件帧", `{"post_type":"message"}`, false},
		{"非法 JSON", `xx`, false},
	}
	for _, tc := range cases {
		if got := authFailed([]byte(tc.data)); got != tc.want {
			t.Errorf("%s: got %v want %v", tc.name, got, tc.want)
		}
	}
}

func TestHeadTruncatesByRune(t *testing.T) {
	// 中文按字符截断：10 个中文字 + 省略号，不在码字中间断开
	got := head([]byte(strings.Repeat("中", 50)), 10)
	if got != strings.Repeat("中", 10)+"..." {
		t.Fatalf("中文截断不符: %q", got)
	}
	// 短内容原样返回
	if got := head([]byte("hi"), 10); got != "hi" {
		t.Fatalf("短内容不符: %q", got)
	}
	// 非法 UTF-8 不炸：rune 解码为替换符
	if head([]byte{0xff, 0xfe, 0xfd}, 10) == "" {
		t.Fatal("非法 UTF-8 应有输出")
	}
}

// TestWriteJSONTimesOutAgainstStalledPeer：对端 upgrade 后不读，
// TCP 缓冲填满后写阻塞，writeDeadline 到点必须返回错误而不是无限挂着。
func TestWriteJSONTimesOutAgainstStalledPeer(t *testing.T) {
	up := websocket.Upgrader{}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := up.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		_ = conn // 挂住：不读不写，模拟对端半死
		select {}
	}))
	defer srv.Close()
	t.Cleanup(srv.Close)
	wsURL := "ws" + strings.TrimPrefix(srv.URL, "http")

	dialer := websocket.DefaultDialer
	conn, _, err := dialer.Dial(wsURL, nil)
	if err != nil {
		t.Fatalf("拨号失败: %v", err)
	}
	defer conn.Close()

	n := newNapCat(wsURL, "", quietLogger())
	n.mu.Lock()
	n.conn = conn
	n.writeDeadline = 300 * time.Millisecond
	n.mu.Unlock()

	// 循环写大帧直到出错：首批进 TCP 缓冲会成功，缓冲填满后阻塞到 deadline
	big := strings.Repeat("x", 4<<20)
	start := time.Now()
	var werr error
	for i := 0; i < 100 && werr == nil; i++ {
		werr = n.writeJSON(map[string]any{"big": big})
	}
	elapsed := time.Since(start)

	if werr == nil {
		t.Fatal("预期写超时返回错误，实际全部成功（缓冲没填满？加大循环次数）")
	}
	if elapsed > 5*time.Second {
		t.Fatalf("写阻塞过久，deadline 未生效: %v", elapsed)
	}
}
