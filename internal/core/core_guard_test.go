package core

// guard（panic 恢复）的单测（26102202#4）：宿主回调 panic 不许带走
// consume 循环，后续事件照常投递。

import (
	"context"
	"io"
	"strings"
	"testing"

	"github.com/hashicorp/go-hclog"

	"github.com/full-finger/loquat/internal/pluginapi"
)

func TestConsumeSurvivesOnEventPanic(t *testing.T) {
	// 投递结果走 channel：consume goroutine 写、主 goroutine 读，无共享内存。
	// consume 现在把信封 marshal 成 JSON 再投递，用 message.id 区分两条。
	delivered := make(chan string, 2)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	c := &Core{
		logger: hclog.New(&hclog.LoggerOptions{Output: io.Discard, Level: hclog.Error}),
		events: make(chan *pluginapi.EventEnvelope, 4),
		onEvent: func(payload []byte) {
			delivered <- string(payload)
			if strings.Contains(string(payload), "first") {
				panic("boom: 宿主回调第一枪") // 模拟宿主侧 panic
			}
		},
		ctx: ctx,
	}
	go c.consume()

	msg := func(id string) *pluginapi.EventEnvelope {
		return &pluginapi.EventEnvelope{
			Kind:    pluginapi.EventKind_EVENT_KIND_MESSAGE,
			Message: &pluginapi.UnifiedMessage{Id: id},
		}
	}
	c.events <- msg("first")
	c.events <- msg("second")

	for got := range delivered {
		if strings.Contains(got, "second") {
			return // panic 后第二条照常投递，guard 生效
		}
		if !strings.Contains(got, "first") {
			t.Fatalf("投递内容不符: %s", got)
		}
	}
	t.Fatal("超时：panic 后第二条事件未投递")
}
