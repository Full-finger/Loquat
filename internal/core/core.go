// Package core 是 loquat 的 Go 核心：
// 读配置、以 go-plugin 管理适配器插件、事件入有界队列、回调宿主、代发动作。
// exe 形态自测（cmd/core 默认构建）与 c-shared 形态嵌入 Rust（dll 标签构建）共用。
package core

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"runtime/debug"
	"sort"
	"sync"
	"sync/atomic"
	"time"

	"github.com/hashicorp/go-hclog"
	"github.com/hashicorp/go-plugin"
	"google.golang.org/protobuf/encoding/protojson"
	"gopkg.in/yaml.v3"

	"github.com/full-finger/loquat/internal/pluginapi"
)

const (
	eventsBuffer  = 256 // 事件队列容量，满了丢弃并记日志
	maxBackoff    = 30 * time.Second
	firstBackoff  = time.Second
	actionTimeout = 15 * time.Second
)

type Config struct {
	Napcat struct {
		URL   string `yaml:"url"`
		Token string `yaml:"token"`
	} `yaml:"napcat"`
	Plugins struct {
		// Adapters 拉起的适配器列表，如 [napcat, sandbox]。
		// 核心为每个适配器维护独立子进程与断流重启循环，
		// 下行（SendMessage/SendAction）按请求里的 platform 路由。
		Adapters []string `yaml:"adapters"`
		Napcat   struct {
			Path string `yaml:"path"`
		} `yaml:"napcat"`
		Sandbox struct {
			Path string `yaml:"path"`
			Addr string `yaml:"addr"` // 聊天页监听地址（已并入 webui，此字段预留）
		} `yaml:"sandbox"`
	} `yaml:"plugins"`
}

// enabledAdapters 归一化适配器列表：缺省 [napcat]（兼容旧配置），去重保序。
// 调用方持 cfgMu（热更与读取互斥）。
func (c *Config) enabledAdapters() []string {
	seen := map[string]bool{}
	var out []string
	for _, name := range c.Plugins.Adapters {
		if name != "napcat" && name != "sandbox" {
			continue // loadConfig 已报错，这里兜底跳过
		}
		if !seen[name] {
			seen[name] = true
			out = append(out, name)
		}
	}
	if len(out) == 0 {
		out = []string{"napcat"}
	}
	return out
}

var ErrNotStarted = errors.New("core 未启动")

// pluginRuntime 打包一次 spawn 的产物：插件客户端与 Dispense 出的适配器桩。
// 经 atomic.Pointer 整体换装——写端 Store 新实例，读端 Load 一次快照用到底，
// 避免 client/adapter 两字段在 spawn 重启与并发调用间的散装数据竞争。
// 读到旧快照意味着旧适配器正被替换，调用随其 Kill 返回错误，属预期行为。
type pluginRuntime struct {
	client  *plugin.Client
	adapter pluginapi.AdapterClient
	pid     int // 适配器子进程 PID，供系统状态采集
	// spawn 时的启动参数快照：适配器管理 diff「参数是否变更」用
	launchedPath  string
	launchedURL   string
	launchedToken string
}

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

// getRuntime 取平台对应的 runtime。未启用/未就绪返回 false。
func (c *Core) getRuntime(platform string) (*pluginRuntime, bool) {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	rt, ok := c.runtimes[platform]
	return rt, ok
}

// runtimeFor 下行调用的路由入口：platform 必填，未启用报错。
// 消息该发给谁由调用方决定——核心不猜（猜错会打到真实用户）。
func (c *Core) runtimeFor(platform string) (*pluginRuntime, error) {
	if platform == "" {
		return nil, errors.New("platform 必填（下行按平台路由，不猜默认）")
	}
	rt, ok := c.getRuntime(platform)
	if !ok {
		return nil, fmt.Errorf("适配器 %s 未启用或未就绪", platform)
	}
	return rt, nil
}

// killRuntime 杀掉平台插件客户端。可重复调用。与 spawn 的换装持同一把锁：
// 杀的时候不会有新实例插入到身后。
// 只 Kill 不从 map 删除：Status/采集方仍能读到该平台的 PID（alive 如实
// 变 false），run 循环退避后 spawn 会覆盖同 key。
func (c *Core) killRuntime(platform string) {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	if rt, ok := c.runtimes[platform]; ok {
		rt.client.Kill()
	}
}

// killAllRuntimes 停止/重启路径用：杀掉全部平台的插件客户端。
// 只 Kill 不删 map（同 killRuntime 的观测语义）；spawn 会覆盖同 key。
func (c *Core) killAllRuntimes() {
	c.runtimesMu.Lock()
	defer c.runtimesMu.Unlock()
	for _, rt := range c.runtimes {
		rt.client.Kill()
	}
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

// adapterPath 平台适配器的可执行文件路径。调用方持 cfgMu。
func (cfg *Config) adapterPath(platform string) string {
	if platform == "sandbox" {
		return cfg.Plugins.Sandbox.Path
	}
	return cfg.Plugins.Napcat.Path
}

// adapterEnv 平台适配器的环境变量通道。调用方持 cfgMu。
func (cfg *Config) adapterEnv(platform string) []string {
	if platform == "sandbox" {
		addr := cfg.Plugins.Sandbox.Addr
		if addr == "" {
			addr = "127.0.0.1:7902"
		}
		return []string{"LOQUAT_SANDBOX_ADDR=" + addr}
	}
	return []string{
		"LOQUAT_NAPCAT_URL=" + cfg.Napcat.URL,
		"LOQUAT_NAPCAT_TOKEN=" + cfg.Napcat.Token,
	}
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

// SendAction 供宿主（Rust FFI）透传平台动作，按 platform 路由。并发安全。
// platform 来自调用方解析（消息上下文跟随来源平台），核心不猜。
func SendAction(platform, action string, params []byte, timeout time.Duration) (*pluginapi.SendActionResponse, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return nil, ErrNotStarted
	}
	rt, err := c.runtimeFor(platform)
	if err != nil {
		return nil, err
	}
	if timeout <= 0 {
		timeout = actionTimeout
	}
	ctx, cancel := context.WithTimeout(c.ctx, timeout)
	defer cancel()
	return rt.adapter.SendAction(ctx, &pluginapi.SendActionRequest{
		Action:   action,
		Params:   params,
		Platform: platform,
	})
}

// InjectEvent 注入一条本地事件（沙箱调试用），按 platform 路由。并发安全。
func InjectEvent(platform, text string, atBot bool, mode string) error {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return ErrNotStarted
	}
	rt, err := c.runtimeFor(platform)
	if err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(c.ctx, actionTimeout)
	defer cancel()
	_, err = rt.adapter.InjectEvent(ctx, &pluginapi.InjectEventRequest{
		Text: text, AtBot: atBot, Mode: mode,
	})
	return err
}

// SendMessage 统一发送：段列表交适配器翻译成平台消息。并发安全。
func SendMessage(req *pluginapi.SendMessageRequest, timeout time.Duration) (*pluginapi.SendMessageResponse, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return nil, ErrNotStarted
	}
	rt, err := c.runtimeFor(req.GetPlatform())
	if err != nil {
		return nil, err
	}
	if timeout <= 0 {
		timeout = actionTimeout
	}
	ctx, cancel := context.WithTimeout(c.ctx, timeout)
	defer cancel()
	return rt.adapter.SendMessage(ctx, req)
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

// Status 返回核心状态快照 JSON，供宿主的 status 命令展示。
func Status() string {
	mu.Lock()
	c := active
	mu.Unlock()
	started := c != nil
	dropped := int64(0)
	adapters := []map[string]any{}
	if started {
		dropped = c.eventsDropped.Load()
		c.runtimesMu.Lock()
		for name, rt := range c.runtimes {
			adapters = append(adapters, map[string]any{
				"platform": name,
				"alive":    !rt.client.Exited(),
				"pid":      rt.pid,
			})
		}
		c.runtimesMu.Unlock()
		sort.Slice(adapters, func(i, j int) bool {
			return adapters[i]["platform"].(string) < adapters[j]["platform"].(string)
		})
	}
	b, err := json.Marshal(map[string]any{
		"started": started, "adapters": adapters,
		"events_dropped": dropped,
	})
	if err != nil {
		return "{}"
	}
	return string(b)
}

// ErrFirstRun 标记首运引导：模板已生成，等用户填 token 重启。
// 非故障，调用方（exe 自测入口）按 Info 级提示而非 Error（26102202#10）。
var ErrFirstRun = errors.New("first-run bootstrap")

// loadConfig 读取并校验配置；文件不存在时从模板生成一份并返回
// ErrFirstRun 标记的引导错误。
func loadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		// 首次运行：从模板生成一份，让用户有现成文件可填
		if genErr := generateFromExample(path); genErr != nil {
			return nil, fmt.Errorf("读取配置 %s: %w（且 %v）", path, err, genErr)
		}
		return nil, fmt.Errorf("已从模板生成 %s，请填入真实 token 后重启: %w", path, ErrFirstRun)
	}
	if err != nil {
		return nil, err
	}
	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("解析 %s: %w", path, err)
	}
	// 按启用的适配器校验必填项：不用到的平台不要求配置
	if len(cfg.Plugins.Adapters) == 0 {
		cfg.Plugins.Adapters = []string{"napcat"} // 缺省 napcat，兼容旧配置
	}
	for _, name := range cfg.Plugins.Adapters {
		switch name {
		case "sandbox":
			if cfg.Plugins.Sandbox.Path == "" {
				return nil, fmt.Errorf("配置缺项：plugins.sandbox.path 必填（adapters 含 sandbox）")
			}
		case "napcat":
			if cfg.Napcat.URL == "" || cfg.Plugins.Napcat.Path == "" {
				return nil, fmt.Errorf("配置缺项：napcat.url 与 plugins.napcat.path 必填")
			}
		default:
			return nil, fmt.Errorf("未知适配器 %q（支持：napcat、sandbox）", name)
		}
	}
	return &cfg, nil
}

// generateFromExample 用 <配置路径>.example 模板生成配置文件。
func generateFromExample(path string) error {
	data, err := os.ReadFile(path + ".example")
	if err != nil {
		return fmt.Errorf("找不到配置模板: %w", err)
	}
	return os.WriteFile(path, data, 0o644)
}
