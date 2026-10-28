// Package core 是 loquat 的 Go 核心：
// 读配置、以 go-plugin 管理适配器插件、事件入有界队列、回调宿主、代发动作。
// exe 形态自测（cmd/core 默认构建）与 c-shared 形态嵌入 Rust（dll 标签构建）共用。
// 按职责分文件：生命周期与监督在本文件，配置在 config.go，
// 下行动作（SendAction/SendMessage/Status 等）在 actions.go。
package core

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"runtime/debug"
	"sync"
	"sync/atomic"
	"time"

	"github.com/hashicorp/go-hclog"
	"github.com/hashicorp/go-plugin"
	"google.golang.org/protobuf/encoding/protojson"

	"github.com/full-finger/loquat/internal/pluginapi"
)

const (
	eventsBuffer  = 256 // 事件队列容量，满了丢弃并记日志
	maxBackoff    = 30 * time.Second
	firstBackoff  = time.Second
	actionTimeout = 15 * time.Second
)

var ErrNotStarted = errors.New("core 未启动")

type Core struct {
	cfg        *Config
	configPath string // 配置文件路径：适配器管理写回用
	logger     hclog.Logger
	events     chan *pluginapi.EventEnvelope
	onEvent    func(payload []byte)
	// cfg 的字段可被 ApplyAdaptersConfig 热更（适配器管理写回），
	// 读取点（adapterPath/adapterEnv/enabledAdapters/napcat 参数）持 cfgMu
	cfgMu sync.Mutex
	// c.ctx 是进程级总闸（Stop 取消）；每平台 run 循环另持派生 ctx，
	// 移除单个平台时只取消它自己的
	ctx    context.Context
	cancel context.CancelFunc
	// 每平台一个 runtime（独立子进程与重启循环）。读写都拿 runtimesMu：
	// 调用频率低（每消息一两次），锁无压力；换装与 kill 互斥由它保证
	runtimesMu sync.Mutex
	runtimes   map[string]*pluginRuntime
	runCancels map[string]context.CancelFunc
	// run/consume 收口用：Stop 等它们退出后再返回
	wg sync.WaitGroup
	// 队列满丢弃的事件数：Status 的 events_dropped 口径（26102202#9）
	eventsDropped atomic.Int64
}

// lockCfg/unlockCfg：cfg 字段读取点的短锁包裹。
func (c *Core) lockCfg() {
	c.cfgMu.Lock()
}

func (c *Core) unlockCfg() {
	c.cfgMu.Unlock()
}

var (
	mu     sync.Mutex
	active *Core
)

// Start 读配置并异步拉起核心。事件经 onEvent 回调投递（在核心的
// 消费协程里调用，回调方应当只做入队这类轻活，重活自己开协程）。
// 已启动时返回错误。
func Start(configPath string, onEvent func([]byte), logger hclog.Logger) error {
	mu.Lock()
	if active != nil {
		mu.Unlock()
		return errors.New("core 已启动")
	}
	mu.Unlock()

	cfg, err := loadConfig(configPath)
	if err != nil {
		return err
	}
	logger.Info("配置已加载", "napcat", cfg.Napcat.URL, "adapters", cfg.enabledAdapters())

	ctx, cancel := context.WithCancel(context.Background())
	c := &Core{
		cfg:        cfg,
		configPath: configPath,
		logger:     logger,
		events:     make(chan *pluginapi.EventEnvelope, eventsBuffer),
		onEvent:    onEvent,
		ctx:        ctx,
		cancel:     cancel,
		runtimes:   map[string]*pluginRuntime{},
		runCancels: map[string]context.CancelFunc{},
	}
	// 逐平台 spawn（含子进程握手，可能耗时；不持全局锁，避免查询被拖死）。
	// 任一失败：回收已启动的平台后整体失败——半启动状态最难受。
	c.lockCfg()
	adapters := cfg.enabledAdapters()
	c.unlockCfg()
	for _, name := range adapters {
		if err := c.spawn(name); err != nil {
			cancel()
			c.killAllRuntimes()
			return err
		}
	}
	mu.Lock()
	if active != nil {
		mu.Unlock()
		cancel()
		c.killAllRuntimes()
		return errors.New("core 已启动（并发）")
	}
	active = c
	mu.Unlock()

	// 每平台一条事件流循环：断流重启互不影响，沙箱死不连累 napcat
	c.wg.Add(1)
	go func() { defer c.wg.Done(); c.consume() }()
	for _, name := range adapters {
		c.startRunLoop(name)
	}
	c.logger.Info("核心已启动", "napcat", cfg.Napcat.URL, "adapters", adapters)
	return nil
}

// startRunLoop 为平台起事件流循环（派生 ctx，移除平台时单独取消）。
func (c *Core) startRunLoop(platform string) {
	ctx, cancel := context.WithCancel(c.ctx)
	c.runtimesMu.Lock()
	c.runCancels[platform] = cancel
	c.runtimesMu.Unlock()
	c.wg.Add(1)
	go func() {
		defer c.wg.Done()
		c.run(platform, ctx)
	}()
}

// stopRunLoop 取消平台事件流循环（不杀子进程，由调用方决定）。
// 不等退出：run 循环在下一次泵检查时自行返回，wg 统一收口。
func (c *Core) stopRunLoop(platform string) {
	c.runtimesMu.Lock()
	cancel, ok := c.runCancels[platform]
	delete(c.runCancels, platform)
	c.runtimesMu.Unlock()
	if ok {
		cancel()
	}
}

// spawn 创建平台适配器子进程并完成握手，成功后换装该平台的 runtime。
// 断流重启时会被再次调用。失败路径回收已创建的客户端，避免泄漏子进程。
func (c *Core) spawn(platform string) error {
	c.lockCfg()
	cmd := exec.Command(c.cfg.adapterPath(platform))
	cmd.Env = append(os.Environ(), c.cfg.adapterEnv(platform)...)
	launchedPath := c.cfg.adapterPath(platform)
	launchedURL, launchedToken := c.cfg.Napcat.URL, c.cfg.Napcat.Token
	c.unlockCfg()
	client := plugin.NewClient(&plugin.ClientConfig{
		Cmd:              cmd,
		HandshakeConfig:  pluginapi.Handshake,
		Plugins:          map[string]plugin.Plugin{"adapter": &pluginapi.AdapterPlugin{}},
		AllowedProtocols: []plugin.Protocol{plugin.ProtocolGRPC},
		Logger:           c.logger.Named("plugin"),
	})
	// go-plugin 握手只确认子进程就绪，与 NapCat 的连接是两回事，措辞别混淆。
	c.logger.Info("插件客户端已创建，开始 go-plugin 握手")
	rpcClient, err := client.Client()
	if err != nil {
		client.Kill()
		return fmt.Errorf("go-plugin 握手: %w", err)
	}
	c.logger.Info("go-plugin 握手完成，适配器子进程就绪")
	raw, err := rpcClient.Dispense("adapter")
	if err != nil {
		client.Kill()
		return fmt.Errorf("获取插件实例: %w", err)
	}
	adapter, ok := raw.(pluginapi.AdapterClient)
	if !ok {
		client.Kill()
		return fmt.Errorf("插件实例类型异常: %T", raw)
	}
	pid := 0
	if cmd.Process != nil {
		pid = cmd.Process.Pid
	}
	// 入册前在锁内复查 ctx：Stop 可能已经跑完 killRuntime，此刻换装
	// 出的新实例会成孤儿（无人认领、带着 WS 重连循环一直活）。
	// 已取消则自己收拾自己。
	c.runtimesMu.Lock()
	if err := c.ctx.Err(); err != nil {
		c.runtimesMu.Unlock()
		client.Kill()
		return fmt.Errorf("核心已停止，放弃入册新插件实例: %w", err)
	}
	c.runtimes[platform] = &pluginRuntime{
		client: client, adapter: adapter, pid: pid,
		launchedPath: launchedPath, launchedURL: launchedURL, launchedToken: launchedToken,
	}
	c.runtimesMu.Unlock()
	return nil
}

// guard 捕获单次 panic 并记日志，让循环继续。onEvent 是宿主注入的
// 不可信回调，panic 跨 C 边界 = 宿主进程 abort（Rust 侧 worker 已有
// catch_unwind，Go 侧不能裸奔，26102202#4）。
func guard(logger hclog.Logger, what string) {
	if r := recover(); r != nil {
		logger.Error("panic 已捕获，继续执行", "where", what, "panic", r,
			"stack", string(debug.Stack()))
	}
}

// run 维护平台事件流：断流就退避重启该平台插件子进程，直到 ctx 取消。
// 这是崩溃隔离的另一半：插件死活不影响调用方，最多丢它自己那条流；
// 平台之间互不影响。
func (c *Core) run(platform string, ctx context.Context) {
	backoff := firstBackoff
	for ctx.Err() == nil {
		err := func() (err error) {
			defer guard(c.logger, "pump")
			return c.pump(platform, ctx)
		}()
		if ctx.Err() != nil {
			return
		}
		c.logger.Error("事件流断开", "adapter", platform, "err", err, "backoff", backoff.String())
		c.killRuntime(platform)
		select {
		case <-time.After(backoff):
		case <-ctx.Done():
			return
		}
		if backoff < maxBackoff {
			backoff *= 2
		}
		if err := c.spawn(platform); err != nil {
			c.logger.Error("插件重启失败", "adapter", platform, "err", err)
			continue
		}
		if ctx.Err() != nil {
			// 平台在 spawn 飞行中被移除：Stop 的全局复查拦不住单平台取消，
			// 刚入册的新实例没人认领会成孤儿，自查自收（26102701#6）
			c.killRuntime(platform)
			return
		}
		backoff = firstBackoff
		c.logger.Info("适配器插件已重启", "adapter", platform)
	}
}

// pump 建立平台事件流并逐条入队；ctx 取消返回 nil，其余返回错误。
func (c *Core) pump(platform string, ctx context.Context) error {
	rt, ok := c.getRuntime(platform)
	if !ok {
		return errors.New("插件运行时未就绪")
	}
	c.logger.Info("订阅适配器事件流", "adapter", platform)
	stream, err := rt.adapter.StreamEvents(ctx, &pluginapi.StreamEventsRequest{})
	if err != nil {
		return err
	}
	for {
		ev, err := func() (ev *pluginapi.EventEnvelope, err error) {
			defer guard(c.logger, "stream.Recv")
			return stream.Recv()
		}()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		select {
		case c.events <- ev:
		default:
			n := c.eventsDropped.Add(1)
			c.logger.Warn("队列已满，丢弃一条事件", "dropped_total", n)
		}
	}
}

// consume 消费队列，把事件投递给宿主回调。
// 信封用 protojson marshal 过 C ABI（camelCase 字段、enum 名、int64 字符串，
// 宿主按同一约定解）；marshal 失败记日志丢单条不拖垮循环。
func (c *Core) consume() {
	for {
		select {
		case <-c.ctx.Done():
			return
		case ev := <-c.events:
			func() {
				defer guard(c.logger, "onEvent")
				b, err := protojson.Marshal(ev)
				if err != nil {
					c.logger.Error("事件信封序列化失败，已丢弃", "err", err)
					return
				}
				c.onEvent(b)
			}()
		}
	}
}

// stopWaitTimeout：Stop 等待 run/consume 收口的上限，防个别阻塞点卡死退出路径。
const stopWaitTimeout = 5 * time.Second

// Stop 停止核心并杀掉全部插件子进程。重复调用安全。
// 顺序：cancel → 杀所有平台插件实例 → 等 run/consume 收口。飞行中的 spawn
// 会在锁内看到 ctx 已取消并自己回收，不会留下孤儿子进程（26102202#2）。
func Stop() {
	mu.Lock()
	c := active
	active = nil
	mu.Unlock()
	if c == nil {
		return
	}
	c.cancel()
	c.killAllRuntimes()
	done := make(chan struct{})
	go func() {
		c.wg.Wait()
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(stopWaitTimeout):
		c.logger.Warn("停止等待超时，后台协程未在限期内收口", "timeout", stopWaitTimeout.String())
	}
	c.logger.Info("核心已停止")
}

// RestartAdapters 杀掉全部插件客户端；各平台的 run 循环检测到断流后
// 会退避重启，互不影响。
func RestartAdapters() error {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return ErrNotStarted
	}
	c.killAllRuntimes()
	return nil
}
