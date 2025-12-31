# Loquat - 机器人/代理开发框架

一个基于 Rust 的清洁架构机器人/代理开发框架，采用 AOP（面向切面编程）和日志功能，专为处理即时消息场景而设计。

## 特性

- **清洁架构** - 遵循 SOLID 原则，模块间解耦良好
- **一键启动** - 支持多环境配置的快速启动系统
- **九阶段工作流** - 采用 9 个处理阶段的流水线架构，支持灵活的消息处理
- **AOP 支持** - 提供面向切面编程能力，支持日志、错误跟踪和性能监控
- **多通道支持** - 支持群组、私聊、频道等不同类型的消息通道
- **可扩展性** - 支持第三方 Worker 在特定阶段注册
- **结构化日志** - 提供详细的处理日志和上下文信息，支持 JSON 和文本格式
- **并发安全** - 使用 Arc 和 RwLock 确保多线程安全
- **Web API 服务** - 内置 RESTful API，支持插件和适配器的远程管理
- **REPL 模式** - 提供交互式命令行界面，支持运行时管理
- **优雅关闭** - 分阶段协调关闭机制，确保资源正确释放
- **热重载** - 支持插件和适配器的自动热重载
- **插件生成器** - 内置 CLI 工具，快速创建插件模板

## 快速开始

### Windows 一键启动

Loquat 提供了完整的一键启动系统，支持多环境配置：

```batch
# 使用默认开发环境启动
start.bat

# 指定环境启动（dev/test/prod）
start.bat prod

# 重新编译后启动
start.bat --rebuild

# 启动 REPL 模式
start.bat --repl

# 组合使用
start.bat test --rebuild --rebuild
```

### 插件创建

使用内置的插件生成器快速创建新插件：

```batch
# 交互式创建插件
cargo run -- plugin

# 使用命令行参数创建插件
cargo run -- plugin create --name my_plugin --version 0.1.0 --author "Your Name" --description "My awesome plugin"
```

### 配置文件

配置文件位于 `config/` 目录：

- `default.toml` - 默认配置（所有环境共用）
- `dev.toml` - 开发环境配置
- `test.toml` - 测试环境配置
- `prod.toml` - 生产环境配置

启动时会自动合并 `default.toml` 和指定环境的配置文件。

### 开发工具

提供了多个开发辅助脚本：

```batch
# 重新编译项目
dev-tools\rebuild.bat

# 清理构建产物
dev-tools\clean.bat

# 完全清理（包括日志和临时文件）
dev-tools\clean.bat --all

# 运行检查（check、clippy、test）
dev-tools\check.bat
```

## 配置说明

### 开发环境 (dev.toml)

```toml
[general]
environment = "dev"
name = "Loquat Framework (Dev)"

[logging]
level = "Debug"
format = "text"
output = "console"
file_path = "logs/loquat.log"

[plugins]
enabled = true
auto_load = true
enable_hot_reload = true
hot_reload_interval = 5

[adapters]
enabled = true
auto_load = true
enable_hot_reload = true
hot_reload_interval = 10

[web]
enabled = true
host = "127.0.0.1"
port = 8080
```

### 生产环境 (prod.toml)

```toml
[general]
environment = "prod"
name = "Loquat Framework (Production)"

[logging]
level = "Warn"
format = "json"
output = "combined"
file_path = "logs/loquat.log"

[plugins]
enabled = true
auto_load = true
enable_hot_reload = false

[adapters]
enabled = true
auto_load = true
enable_hot_reload = false

[web]
enabled = true
host = "0.0.0.0"
port = 8080
```

## 架构概述

### 九阶段工作流系统

项目采用 9 个处理阶段（池）的流水线架构：

- **PreInput** - 预输入池
- **Input** - 输入池（支持第三方注册）
- **InputMiddle** - 输入中间池
- **PreProcess** - 预处理池（支持第三方注册）
- **ProcessMiddle** - 处理中间池
- **Process** - 处理池（支持第三方注册）
- **PostProcess** - 后处理池
- **Output** - 输出池（支持第三方注册）
- **PostOutput** - 后输出池

### 数据结构层次

- **Package** - 流上处理的基本单元，包含 target_sites 和 blocks
- **Block** - 事件块数组
- **Group** - 事件组
- **Event** - 单个事件

### 核心组件

#### Engine（引擎）
- **StandardEngine** - 核心协调器，负责协调所有模块
- 接收输入 Package
- 通过 Router 路由到适配器目标
- 通过 ChannelManager 获取/创建 Channel
- 通过 Stream 处理 Package

#### ChannelManager（通道管理器）
- **StandardChannelManager** - 管理多个通道实例
- 根据 ChannelType（group、private、channel）管理不同的通道
- 自动创建和清理闲置通道

#### Stream（数据流）
- **StandardStream** - 包含 9 个池，按顺序处理包
- **StreamProcessor** - 在序列中的池间处理包

#### Pool（处理池）
- **StandardPool** - 标准池实现，管理 Worker
- 按优先级处理 Worker
- Worker 处理包并决定是否释放到下一池或在当前池继续处理

#### AOP（面向切面编程）
- 支持日志、错误跟踪和性能监控等切面
- 提供 AopManager 和 AopProxy 用于动态应用切面
- 可以在方法执行前后插入横切关注点

#### Logging（日志系统）
- 结构化日志系统，支持多种格式（JSON、文本）
- 支持多种写入器（控制台、文件、组合）
- 提供全局日志器和上下文信息

#### Web Service（Web 服务）
- 基于 Axum 框架的 RESTful API
- 支持插件和适配器的远程管理
- 提供 RESTful 接口，包括健康检查、配置查询、插件/适配器管理
- 支持优雅关闭和信号处理

#### REPL（交互式命令行）
- 提供交互式命令行界面
- 支持插件、适配器、配置、日志等管理命令
- 支持 REPL 历史记录和自动补全
- 可在运行时动态管理框架

#### Shutdown Coordinator（优雅关闭协调器）
- 分阶段协调关闭机制
- 支持超时处理和错误恢复
- 确保资源正确释放
- 提供详细的关闭状态和统计信息

## 项目结构

```
Loquat/
├── config/              # 配置文件目录
│   ├── default.toml     # 默认配置
│   ├── dev.toml         # 开发环境
│   ├── test.toml        # 测试环境
│   └── prod.toml        # 生产环境
├── dev-tools/           # 开发工具脚本
│   ├── rebuild.bat      # 重新编译
│   ├── clean.bat        # 清理构建
│   └── check.bat        # 代码检查
├── src/
│   ├── main.rs          # 主入口（配置驱动）
│   ├── config/          # 配置模块
│   ├── engine/          # 引擎模块
│   ├── logging/         # 日志模块
│   ├── plugins/         # 插件系统
│   ├── adapters/        # 适配器系统
│   ├── web/             # Web API 服务
│   ├── repl/            # REPL 交互式界面
│   ├── cli/             # CLI 工具（插件生成器）
│   ├── shutdown/        # 优雅关闭协调器
│   ├── channel_manager/ # 通道管理器
│   ├── streams/         # 数据流处理
│   ├── pools/           # 处理池
│   ├── routers/         # 路由器
│   ├── workers/         # Worker 注册
│   ├── events/          # 事件定义
│   ├── aop/             # AOP 切面编程
│   └── utils/           # 工具函数
├── plugins/             # 插件目录（自动创建）
├── adapters/            # 适配器目录（自动创建）
├── logs/                # 日志目录（自动创建）
├── webui/               # Web UI 界面
├── start.bat            # Windows启动脚本
└── Cargo.toml
```

## 使用示例

### 1. 启动开发环境

双击运行 `start.bat` 或在命令行执行：

```batch
start.bat dev
```

### 2. 启动 REPL 模式

REPL 模式提供交互式命令行界面，支持运行时管理：

```batch
start.bat --repl
```

在 REPL 中可用的命令：
- `help` - 显示帮助信息
- `status` - 显示系统状态
- `plugins` - 管理插件
- `adapters` - 管理适配器
- `config` - 查看配置
- `logs` - 查看日志
- `engine` - 引擎控制
- `clear` - 清屏
- `exit` - 退出

### 3. 使用 Web API

启动 Web 服务后，可以通过 HTTP API 管理框架：

```bash
# 健康检查
curl http://localhost:8080/health

# 列出所有插件
curl http://localhost:8080/api/plugins

# 获取特定插件信息
curl http://localhost:8080/api/plugins/my_plugin

# 重新加载插件
curl -X POST http://localhost:8080/api/plugins/reload

# 列出所有适配器
curl http://localhost:8080/api/adapters

# 获取配置
curl http://localhost:8080/api/config
```

### 4. 创建新插件

使用插件生成器创建插件模板：

```batch
# 交互式创建
cargo run -- plugin

# 使用参数创建
cargo run -- plugin create --name my_plugin --version 0.1.0
```

### 5. 修改配置后重新启动

编辑 `config/dev.toml`，然后：

```batch
start.bat
```

### 6. 完整重新编译

```batch
start.bat --rebuild
```

## 代码示例

### 基本使用

```rust
use loquat::config::LoquatConfig;
use loquat::engine::{Engine, StandardEngine};

#[tokio::main]
async fn main() -> loquat::Result<()> {
    // 使用配置系统自动启动
    let config = LoquatConfig::from_environment("config", "dev")?;
    let app = LoquatApplication::from_config(config)?;
    
    app.run().await;
    
    Ok(())
}
```

### 创建插件

```rust
use loquat::plugins::{Plugin, PluginType, PluginHealth};
use loquat::errors::Result;
use serde_json::Value;

pub struct MyPlugin {
    name: String,
    version: String,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            name: "MyPlugin".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Processor
    }

    async fn init(&mut self) -> Result<()> {
        println!("Plugin initialized");
        Ok(())
    }

    async fn load(&mut self) -> Result<()> {
        println!("Plugin loaded");
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        println!("Plugin unloaded");
        Ok(())
    }

    fn health_status(&self) -> PluginHealth {
        PluginHealth::Healthy
    }

    async fn update_config(&mut self, config: Value) -> Result<()> {
        println!("Config updated: {:?}", config);
        Ok(())
    }
}

// 插件导出函数
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(MyPlugin::new());
    Box::into_raw(plugin)
}

#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {
    unsafe {
        if !plugin.is_null() {
            let _ = Box::from_raw(plugin);
        }
    }
}
```

## API 文档

### Web API 端点

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/` | 欢迎页面 |
| GET | `/health` | 健康检查 |
| GET | `/api/plugins` | 列出所有插件 |
| GET | `/api/plugins/:name` | 获取特定插件信息 |
| POST | `/api/plugins/reload` | 重新加载所有插件 |
| GET | `/api/adapters` | 列出所有适配器 |
| GET | `/api/adapters/:name` | 获取特定适配器信息 |
| POST | `/api/adapters/reload` | 重新加载所有适配器 |
| POST | `/api/reload` | 重新加载所有组件 |
| GET | `/api/config` | 获取当前配置 |

### REPL 命令

| 命令 | 描述 |
|------|------|
| `help` | 显示帮助信息 |
| `status` | 显示系统运行状态 |
| `plugins` | 列出所有加载的插件 |
| `plugins reload` | 重新加载所有插件 |
| `adapters` | 列出所有加载的适配器 |
| `adapters reload` | 重新加载所有适配器 |
| `config` | 显示当前配置 |
| `logs [level]` | 查看日志（可选日志级别） |
| `engine start` | 启动引擎 |
| `engine stop` | 停止引擎 |
| `clear` | 清除屏幕 |
| `exit` | 退出 REPL |

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

[MIT License](LICENSE)

## 联系方式

- GitHub: [Full-finger/Loquat](https://github.com/Full-finger/Loquat)
