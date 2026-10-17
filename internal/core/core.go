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
	"sync"
	"sync/atomic"
	"time"

	"github.com/hashicorp/go-hclog"
	"github.com/hashicorp/go-plugin"
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
		Napcat struct {
			Path string `yaml:"path"`
		} `yaml:"napcat"`
	} `yaml:"plugins"`
}

var ErrNotStarted = errors.New("core 未启动")

// pluginRuntime 打包一次 spawn 的产物：插件客户端与 Dispense 出的适配器桩。
// 经 atomic.Pointer 整体换装——写端 Store 新实例，读端 Load 一次快照用到底，
// 避免 client/adapter 两字段在 spawn 重启与并发调用间的散装数据竞争。
// 读到旧快照意味着旧适配器正被替换，调用随其 Kill 返回错误，属预期行为。
type pluginRuntime struct {
	client  *plugin.Client
	adapter pluginapi.AdapterClient
}

type Core struct {
	cfg     *Config
	logger  hclog.Logger
	runtime atomic.Pointer[pluginRuntime]
	events  chan *pluginapi.Event
	onEvent func(payload []byte)
	ctx     context.Context
	cancel  context.CancelFunc
}

// killRuntime 杀掉当前插件客户端，未 spawn 成功过则无事发生。可重复调用。
func (c *Core) killRuntime() {
	if rt := c.runtime.Load(); rt != nil {
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
	logger.Info("配置已加载", "napcat", cfg.Napcat.URL, "adapter", cfg.Plugins.Napcat.Path)

	ctx, cancel := context.WithCancel(context.Background())
	c := &Core{
		cfg:     cfg,
		logger:  logger,
		events:  make(chan *pluginapi.Event, eventsBuffer),
		onEvent: onEvent,
		ctx:     ctx,
		cancel:  cancel,
	}
	// spawn 含子进程握手，可能耗时；不持锁，避免 Status 等查询被拖死。
	// 完成后持锁双检，保证全局只有一个实例。
	if err := c.spawn(); err != nil {
		cancel()
		return err
	}
	mu.Lock()
	if active != nil {
		mu.Unlock()
		cancel()
		c.killRuntime()
		return errors.New("core 已启动（并发）")
	}
	active = c
	mu.Unlock()

	go c.consume()
	go c.run()
	c.logger.Info("核心已启动", "napcat", cfg.Napcat.URL, "adapter", cfg.Plugins.Napcat.Path)
	return nil
}

// spawn 创建插件子进程并完成握手，成功后原子换装新 runtime。
// 断流重启时会被再次调用。失败路径回收已创建的客户端，避免泄漏子进程。
func (c *Core) spawn() error {
	cmd := exec.Command(c.cfg.Plugins.Napcat.Path)
	cmd.Env = append(os.Environ(),
		"LOQUAT_NAPCAT_URL="+c.cfg.Napcat.URL,
		"LOQUAT_NAPCAT_TOKEN="+c.cfg.Napcat.Token,
	)
	client := plugin.NewClient(&plugin.ClientConfig{
		Cmd:              cmd,
		HandshakeConfig:  pluginapi.Handshake,
		Plugins:          map[string]plugin.Plugin{"adapter": &pluginapi.AdapterPlugin{}},
		AllowedProtocols: []plugin.Protocol{plugin.ProtocolGRPC},
		Logger:           c.logger.Named("plugin"),
	})
	c.logger.Info("插件客户端已创建，开始握手")
	rpcClient, err := client.Client()
	if err != nil {
		client.Kill()
		return fmt.Errorf("插件握手: %w", err)
	}
	c.logger.Info("适配器握手完成")
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
	c.runtime.Store(&pluginRuntime{client: client, adapter: adapter})
	return nil
}

// run 维护事件流：断流就退避重启插件子进程，直到 ctx 取消。
// 这是崩溃隔离的另一半：插件死活不影响调用方，最多丢它自己那条流。
func (c *Core) run() {
	backoff := firstBackoff
	for c.ctx.Err() == nil {
		err := c.pump()
		if c.ctx.Err() != nil {
			return
		}
		c.logger.Error("事件流断开", "err", err, "backoff", backoff.String())
		c.killRuntime()
		select {
		case <-time.After(backoff):
		case <-c.ctx.Done():
			return
		}
		if backoff < maxBackoff {
			backoff *= 2
		}
		if err := c.spawn(); err != nil {
			c.logger.Error("插件重启失败", "err", err)
			continue
		}
		backoff = firstBackoff
		c.logger.Info("适配器插件已重启")
	}
}

// pump 建立事件流并逐条入队；ctx 取消返回 nil，其余返回错误。
func (c *Core) pump() error {
	rt := c.runtime.Load()
	if rt == nil {
		return errors.New("插件运行时未就绪")
	}
	c.logger.Info("订阅适配器事件流")
	stream, err := rt.adapter.StreamEvents(c.ctx, &pluginapi.StreamEventsRequest{})
	if err != nil {
		return err
	}
	for {
		ev, err := stream.Recv()
		if err != nil {
			if c.ctx.Err() != nil {
				return nil
			}
			return err
		}
		select {
		case c.events <- ev:
		default:
			c.logger.Warn("队列已满，丢弃一条事件")
		}
	}
}

// consume 消费队列，把事件投递给宿主回调。
func (c *Core) consume() {
	for {
		select {
		case <-c.ctx.Done():
			return
		case ev := <-c.events:
			c.onEvent(ev.GetPayload())
		}
	}
}

// SendAction 供宿主（exe 自测 / Rust FFI）下发动作。并发安全。
func SendAction(action string, params []byte, timeout time.Duration) (*pluginapi.SendActionResponse, error) {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return nil, ErrNotStarted
	}
	if timeout <= 0 {
		timeout = actionTimeout
	}
	rt := c.runtime.Load()
	if rt == nil {
		return nil, ErrNotStarted
	}
	ctx, cancel := context.WithTimeout(c.ctx, timeout)
	defer cancel()
	return rt.adapter.SendAction(ctx, &pluginapi.SendActionRequest{
		Action: action,
		Params: params,
	})
}

// Stop 停止核心并杀掉插件子进程。重复调用安全。
func Stop() {
	mu.Lock()
	c := active
	active = nil
	mu.Unlock()
	if c == nil {
		return
	}
	c.cancel()
	c.killRuntime()
	c.logger.Info("核心已停止")
}

// RestartAdapters 杀掉插件客户端；run 循环检测到断流后会退避重启。
func RestartAdapters() error {
	mu.Lock()
	c := active
	mu.Unlock()
	if c == nil {
		return ErrNotStarted
	}
	c.killRuntime()
	return nil
}

// Status 返回核心状态快照 JSON，供宿主的 status 命令展示。
func Status() string {
	mu.Lock()
	c := active
	mu.Unlock()
	started := c != nil
	alive := false
	if started {
		if rt := c.runtime.Load(); rt != nil {
			alive = !rt.client.Exited()
		}
	}
	b, err := json.Marshal(map[string]any{"started": started, "adapter_alive": alive})
	if err != nil {
		return "{}"
	}
	return string(b)
}

func loadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		// 首次运行：从模板生成一份，让用户有现成文件可填
		if genErr := generateFromExample(path); genErr != nil {
			return nil, fmt.Errorf("读取配置 %s: %w（且 %v）", path, err, genErr)
		}
		return nil, fmt.Errorf("已从模板生成 %s，请填入真实 token 后重启", path)
	}
	if err != nil {
		return nil, err
	}
	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("解析 %s: %w", path, err)
	}
	if cfg.Napcat.URL == "" || cfg.Plugins.Napcat.Path == "" {
		return nil, fmt.Errorf("配置缺项：napcat.url 与 plugins.napcat.path 必填")
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
