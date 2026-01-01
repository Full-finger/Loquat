# Loquat框架新手开发教程

> 面向Rust开发者，从零开始学习Loquat机器人/代理开发框架

---

## 目录

- [第1章：认识Loquat框架](#第1章认识loquat框架)
- [第2章：快速上手](#第2章快速上手)
- [第3章：核心概念详解](#第3章核心概念详解)
- [第4章：创建你的第一个插件](#第4章创建你的第一个插件)
- [第5章：创建你的第一个适配器](#第5章创建你的第一个适配器)
- [第6章：使用事件系统](#第6章使用事件系统)
- [第7章：AOP和日志系统](#第7章aop和日志系统)
- [第8章：Web API和REPL](#第8章web-api和repl)
- [第9章：配置和部署](#第9章配置和部署)
- [第10章：最佳实践和进阶](#第10章最佳实践和进阶)

---

## 第1章：认识Loquat框架

### 1.1 什么是Loquat？

Loquat是一个基于Rust的**清洁架构机器人/代理开发框架**，专为处理即时消息场景而设计。它采用现代化的架构设计，提供了强大的可扩展性和灵活性。

**核心特性：**

- 🏗️ **清洁架构**：遵循SOLID原则，模块间解耦良好
- 🚀 **一键启动**：支持多环境配置的快速启动系统
- 🔄 **九阶段工作流**：采用9个处理阶段的流水线架构
- 🔧 **AOP支持**：面向切面编程，支持日志、错误跟踪和性能监控
- 📡 **多通道支持**：支持群组、私聊、频道等不同类型的消息通道
- 🧩 **可扩展性**：支持第三方Worker在特定阶段注册
- 📊 **结构化日志**：详细的处理日志和上下文信息
- 🔒 **并发安全**：使用Arc和RwLock确保多线程安全
- 🌐 **Web API服务**：内置RESTful API，支持远程管理
- 💻 **REPL模式**：交互式命令行界面，运行时管理
- 🛡️ **优雅关闭**：分阶段协调关闭机制
- ♻️ **热重载**：支持插件和适配器的自动热重载

### 1.2 适用场景

Loquat框架特别适合以下场景：

- ✅ **聊天机器人开发**：微信、QQ、Telegram等
- ✅ **消息处理系统**：批量处理、路由、转发
- ✅ **自动化代理**：定时任务、监控、自动化流程
- ✅ **多平台适配**：统一接口对接多个消息平台
- ✅ **插件化应用**：需要高度可扩展性的应用

### 1.3 技术栈

Loquat框架基于以下技术构建：

- **编程语言**：Rust 1.70+
- **异步运行时**：Tokio
- **Web框架**：Axum (用于RESTful API)
- **序列化**：serde (JSON/TOML)
- **日志**：自定义结构化日志系统
- **并发**：async/await + Arc/RwLock/mpsc

### 1.4 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                    Loquat Framework                     │
└─────────────────────────────────────────────────────────┘
                          │
         ┌────────────────┼────────────────┐
         ▼                ▼                ▼
   ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ Adapters │    │ Plugins  │    │   Web    │
   │  适配器   │    │  插件系统 │    │  Web API │
   └──────────┘    └──────────┘    └──────────┘
         │                │                │
         └────────────────┼────────────────┘
                          ▼
                    ┌──────────┐
                    │  Engine  │
                    │  引擎核心 │
                    └──────────┘
                          │
         ┌────────────────┼────────────────┐
         ▼                ▼                ▼
   ┌──────────┐    ┌──────────┐    ┌──────────┐
    │  Stream  │    │ChannelMgr│    │   AOP    │
    │ 数据流   │    │ 通道管理  │    │ 切面编程 │
    └──────────┘    └──────────┘    └──────────┘
```

---

## 第2章：快速上手

### 2.1 环境准备

#### 安装Rust工具链

```bash
# Windows
# 访问 https://rustup.rs/ 下载rustup-init.exe
# 或使用以下命令（需要先安装Visual Studio Build Tools）

# Linux/Mac
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

验证安装：

```bash
rustc --version
cargo --version
```

#### 获取源码

```bash
# 克隆项目
git clone https://github.com/Full-finger/Loquat.git
cd Loquat
```

### 2.2 第一次运行

Loquat提供了便捷的一键启动脚本（Windows）：

#### Windows启动

```batch
# 使用默认开发环境启动
start.bat

# 指定环境启动（dev/test/prod）
start.bat prod

# 重新编译后启动
start.bat --rebuild

# 启动REPL模式
start.bat --repl

# 组合使用
start.bat test --rebuild
```

#### Linux/Mac启动

```bash
# 开发环境
cargo run

# 生产环境
cargo run -- --environment prod

# REPL模式
cargo run -- --repl
```

### 2.3 REPL交互

REPL（Read-Eval-Print Loop）模式提供交互式命令行界面：

```bash
# 启动REPL
start.bat --repl
```

在REPL中可用的命令：

```
loquat> help              # 显示帮助信息
loquat> status            # 显示系统状态
loquat> plugins           # 列出所有插件
loquat> plugins reload    # 重新加载所有插件
loquat> adapters          # 列出所有适配器
loquat> adapters reload   # 重新加载所有适配器
loquat> config            # 显示当前配置
loquat> logs              # 查看日志
loquat> engine start      # 启动引擎
loquat> engine stop       # 停止引擎
loquat> clear             # 清屏
loquat> exit              # 退出REPL
```

### 2.4 配置文件

配置文件位于 `config/` 目录：

- `default.toml` - 默认配置（所有环境共用）
- `dev.toml` - 开发环境配置
- `test.toml` - 测试环境配置
- `prod.toml` - 生产环境配置

启动时会自动合并 `default.toml` 和指定环境的配置文件。

#### 配置示例

```toml
# config/dev.toml

[general]
environment = "dev"
name = "Loquat Framework (Dev)"

[logging]
level = "Debug"              # 日志级别：Debug, Info, Warn, Error
format = "text"              # 格式：text, json
output = "console"           # 输出：console, file, combined
file_path = "logs/loquat.log"

[plugins]
enabled = true               # 启用插件系统
auto_load = true             # 自动加载插件
enable_hot_reload = true      # 启用热重载
hot_reload_interval = 5       # 热重载间隔（秒）

[adapters]
enabled = true               # 启用适配器系统
auto_load = true             # 自动加载适配器
enable_hot_reload = true      # 启用热重载
hot_reload_interval = 10     # 热重载间隔（秒）

[web]
enabled = true               # 启用Web服务
host = "127.0.0.1"           # 监听地址
port = 8080                  # 监听端口
```

### 2.5 开发工具

Loquat提供了多个开发辅助脚本：

```batch
# Windows
dev-tools\rebuild.bat        # 重新编译项目
dev-tools\clean.bat          # 清理构建产物
dev-tools\clean.bat --all    # 完全清理（包括日志和临时文件）
dev-tools\check.bat          # 运行检查（check、clippy、test）

# Linux/Mac
cargo clean                  # 清理构建产物
cargo check                  # 快速检查
cargo clippy                 # 代码检查
cargo test                   # 运行测试
```

---

## 第3章：核心概念详解

### 3.1 九阶段工作流系统

Loquat采用9个处理阶段的流水线架构，每个阶段都是一个"池"（Pool），可以注册多个Worker来处理任务。

```
输入消息
    │
    ▼
┌─────────────┐
│ PreInput   │  预输入池
└─────────────┘
    │
    ▼
┌─────────────┐  ← 第三方可注册Worker
│ Input       │  输入池
└─────────────┘
    │
    ▼
┌─────────────┐
│ InputMiddle │  输入中间池
└─────────────┘
    │
    ▼
┌─────────────┐  ← 第三方可注册Worker
│ PreProcess  │  预处理池
└─────────────┘
    │
    ▼
┌─────────────┐
│ ProcessMiddle│ 处理中间池
└─────────────┘
    │
    ▼
┌─────────────┐  ← 第三方可注册Worker
│ Process     │  处理池
└─────────────┘
    │
    ▼
┌─────────────┐
│ PostProcess │  后处理池
└─────────────┘
    │
    ▼
┌─────────────┐  ← 第三方可注册Worker
│ Output      │  输出池
└─────────────┘
    │
    ▼
┌─────────────┐
│ PostOutput  │  后输出池
└─────────────┘
    │
    ▼
输出消息
```

**阶段说明：**

1. **PreInput**：预输入处理，在最开始进行一些准备工作
2. **Input**：输入处理，支持第三方注册Worker
3. **InputMiddle**：输入中间处理，连接Input和PreProcess
4. **PreProcess**：预处理，支持第三方注册Worker
5. **ProcessMiddle**：处理中间，连接PreProcess和Process
6. **Process**：主要处理，支持第三方注册Worker
7. **PostProcess**：后处理，清理和整理
8. **Output**：输出处理，支持第三方注册Worker
9. **PostOutput**：后输出，最后的清理工作

### 3.2 数据结构层次

Loquat使用层次化的数据结构来组织和处理消息：

```
Package（包）
  └─ TargetSites（目标站点列表）
      └─ Block（事件块）
          └─ Group（事件组）
              └─ Event（单个事件）
```

#### Package（包）

Package是流上处理的基本单元，包含：
- `target_sites`：目标站点列表
- `blocks`：事件块数组
- `metadata`：包的元数据

#### Block（块）

Block是事件块的集合，包含：
- `groups`：事件组数组
- `metadata`：块的元数据

#### Group（组）

Group是相关事件的集合，包含：
- `events`：事件数组
- `metadata`：组的元数据

#### Event（事件）

Event是单个事件单元，包含：
- `event_data`：具体事件数据
- `metadata`：事件元数据

### 3.3 事件系统

Loquat支持丰富的事件类型，涵盖即时通讯的各种场景。

#### 消息事件（MessageEvent）

```rust
pub enum MessageEvent {
    Text {
        text: String,
        metadata: EventMetadata,
    },
    Image {
        url: String,
        caption: Option<String>,
        metadata: EventMetadata,
    },
    Voice {
        url: String,
        duration: u32,  // 秒
        metadata: EventMetadata,
    },
    Video {
        url: String,
        caption: Option<String>,
        metadata: EventMetadata,
    },
    File {
        url: String,
        filename: String,
        metadata: EventMetadata,
    },
}
```

#### 通知事件（NoticeEvent）

```rust
pub enum NoticeEvent {
    SystemNotice {
        notice_type: String,
        content: String,
        metadata: EventMetadata,
    },
    FriendRequest {
        user_id: String,
        message: Option<String>,
        metadata: EventMetadata,
    },
    GroupInvite {
        group_id: String,
        inviter_id: String,
        metadata: EventMetadata,
    },
    MemberChange {
        group_id: String,
        user_id: String,
        change_type: String,  // "join", "leave", "kick", etc.
        metadata: EventMetadata,
    },
}
```

#### 事件元数据（EventMetadata）

每个事件都包含丰富的元数据信息：

```rust
pub struct EventMetadata {
    pub event_type: String,
    pub source: EventSource,      // User, System, Timer, etc.
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub timestamp: Option<i64>,
    pub event_id: Option<String>,
    // ... 其他元数据
}
```

### 3.4 适配器系统

适配器是连接外部系统和Loquat框架的桥梁。Loquat提供了多种内置适配器。

#### ConsoleAdapter（控制台适配器）

用于从控制台接收输入并处理消息，适合开发测试和简单场景。

**特点：**
- ✅ 交互式命令行输入
- ✅ 支持多种命令
- ✅ 适合快速原型开发

**配置示例：**
```json
{
  "adapter_type": "console",
  "adapter_id": "console-001",
  "platform": "console://stdin"
}
```

#### EchoAdapter（回显适配器）

将接收到的消息原样返回，用于测试和验证消息流程。

**特点：**
- ✅ 简单的消息确认
- ✅ 测试消息路由
- ✅ 验证事件处理

**配置示例：**
```json
{
  "adapter_type": "echo",
  "adapter_id": "echo-001",
  "platform": "echo://test"
}
```

#### MockTestAdapter（测试适配器）

定时生成测试事件，用于测试框架完整功能。

**特点：**
- ✅ 定时生成测试事件（默认每5秒）
- ✅ 轮换事件类型（文本、图片、语音、通知、群组）
- ✅ 统计监控功能
- ✅ 可配置生成间隔

**配置示例：**
```json
{
  "adapter_type": "mock_test",
  "adapter_id": "mock-test-001",
  "platform": "mock://test",
  "platform_config": {
    "event_interval_seconds": 5
  }
}
```

### 3.5 插件系统

插件是Loquat框架的可扩展组件，可以在特定阶段注册Worker来处理事件。

#### Plugin trait

所有插件都需要实现 `Plugin` trait：

```rust
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
    
    async fn init(&mut self) -> Result<()>;
    async fn load(&mut self) -> Result<()>;
    async fn unload(&mut self) -> Result<()>;
    
    fn health_status(&self) -> PluginHealth;
    async fn update_config(&mut self, config: serde_json::Value) -> Result<()>;
}
```

#### 插件类型

```rust
pub enum PluginType {
    Native,        // 原生Rust插件
    Processor,     // 处理器插件
    Adapter,       // 适配器插件
    Custom(String), // 自定义类型
}
```

### 3.6 AOP系统

AOP（面向切面编程）允许你在不修改原有代码的情况下，为服务添加横切关注点（如日志、错误跟踪、性能监控）。

#### 切面（Aspect）

切面定义了在方法执行前后要执行的行为：

```rust
#[async_trait::async_trait]
pub trait Aspect: Send + Sync {
    async fn before(&self, join_point: &JoinPoint) -> ExecutionResult<()>;
    async fn after(&self, join_point: &JoinPoint, result: &ExecutionResult<()>, context: &LogContext) -> ExecutionResult<()>;
}
```

#### 内置切面

Loquat提供了多个内置切面：

1. **LoggingAspect**：日志记录
2. **ErrorTrackingAspect**：错误跟踪
3. **PerformanceAspect**：性能监控

---

## 第4章：创建你的第一个插件

### 4.1 使用插件生成器

Loquat提供了便捷的插件生成器，可以快速创建插件模板。

#### 交互式创建

```bash
cargo run -- plugin
```

按照提示输入插件信息：
```
Plugin name: my_first_plugin
Version: 0.1.0
Author: Your Name
Description: My first Loquat plugin
```

#### 命令行创建

```bash
cargo run -- plugin create \
  --name my_first_plugin \
  --version 0.1.0 \
  --author "Your Name" \
  --description "My first Loquat plugin"
```

这将创建一个新的插件目录：
```
plugins/my_first_plugin/
├── Cargo.toml
├── config.json
└── src/
    └── lib.rs
```

### 4.2 插件结构

生成的插件模板包含以下文件：

#### Cargo.toml

```toml
[package]
name = "my_first_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
loquat = { path = "../../" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### src/lib.rs

```rust
use async_trait::async_trait;
use loquat::plugins::{Plugin, PluginHealth, PluginType};
use loquat::errors::Result;
use serde::Deserialize;

/// Plugin configuration
#[derive(Debug, Deserialize)]
pub struct Config {
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Main plugin struct
pub struct MyFirstPlugin {
    name: String,
    version: String,
    description: String,
    author: String,
    config: Config,
}

impl MyFirstPlugin {
    pub fn new() -> Self {
        Self {
            name: "MyFirstPlugin".to_string(),
            version: "0.1.0".to_string(),
            description: "My first Loquat plugin".to_string(),
            author: "Your Name".to_string(),
            config: Config::default(),
        }
    }
}

#[async_trait]
impl Plugin for MyFirstPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Native
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn author(&self) -> Option<&str> {
        Some(&self.author)
    }

    async fn init(&mut self) -> Result<()> {
        println!("{} v{} initialized!", self.name, self.version);
        Ok(())
    }

    async fn load(&mut self) -> Result<()> {
        println!("{} loaded!", self.name);
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        println!("{} unloaded!", self.name);
        Ok(())
    }

    fn health_status(&self) -> PluginHealth {
        PluginHealth::Healthy
    }

    async fn update_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let Ok(new_config) = serde_json::from_value::<Config>(config) {
            self.config = new_config;
            println!("{} config updated!", self.name);
        }
        Ok(())
    }
}

/// Plugin constructor (called by the loader)
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = MyFirstPlugin::new();
    Box::into_raw(Box::new(plugin))
}

/// Plugin destructor (called by the loader)
#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}
```

### 4.3 编译插件

在插件目录下编译：

```bash
cd plugins/my_first_plugin
cargo build --release
```

编译成功后会生成动态库：
- Windows: `target/release/my_first_plugin.dll`
- Linux: `target/release/libmy_first_plugin.so`
- Mac: `target/release/libmy_first_plugin.dylib`

### 4.4 加载插件

插件会自动被Loquat的插件管理器加载。确保插件目录配置正确：

```toml
[plugins]
enabled = true
auto_load = true
```

启动Loquat，插件会自动加载：

```bash
start.bat
```

你将看到类似的输出：
```
[INFO] Loading plugins from: plugins/
[INFO] Found plugin: my_first_plugin v0.1.0
[INFO] MyFirstPlugin v0.1.0 initialized!
[INFO] MyFirstPlugin loaded!
```

### 4.5 实现一个简单的问候插件

让我们创建一个简单的插件，当收到文本消息时，自动回复问候。

```rust
// src/lib.rs

use async_trait::async_trait;
use loquat::plugins::{Plugin, PluginHealth, PluginType};
use loquat::errors::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub greeting_message: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            greeting_message: "Hello from Loquat!".to_string(),
        }
    }
}

pub struct GreetingPlugin {
    name: String,
    version: String,
    config: Config,
    message_count: u64,
}

impl GreetingPlugin {
    pub fn new() -> Self {
        Self {
            name: "GreetingPlugin".to_string(),
            version: "1.0.0".to_string(),
            config: Config::default(),
            message_count: 0,
        }
    }

    async fn handle_message(&mut self, text: &str) -> Result<String> {
        self.message_count += 1;
        
        let response = if text.contains("hello") || text.contains("hi") {
            format!("{} (Processed {} messages)", 
                self.config.greeting_message, 
                self.message_count)
        } else {
            format!("Received: {}", text)
        };
        
        Ok(response)
    }
}

#[async_trait]
impl Plugin for GreetingPlugin {
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
        println!("🎉 GreetingPlugin initialized!");
        Ok(())
    }

    async fn load(&mut self) -> Result<()> {
        println!("✅ GreetingPlugin loaded!");
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        println!("👋 GreetingPlugin unloaded! Processed {} messages", 
            self.message_count);
        Ok(())
    }

    fn health_status(&self) -> PluginHealth {
        PluginHealth::Healthy
    }

    async fn update_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let Ok(new_config) = serde_json::from_value::<Config>(config) {
            self.config = new_config;
            println!("📝 GreetingPlugin config updated!");
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = GreetingPlugin::new();
    Box::into_raw(Box::new(plugin))
}

#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}
```

### 4.6 测试插件

1. 编译插件：
```bash
cd plugins/greeting_plugin
cargo build --release
```

2. 启动Loquat：
```bash
start.bat
```

3. 使用ConsoleAdapter发送消息：
```
hello
```

你应该会收到回复：
```
Hello from Loquat! (Processed 1 messages)
```

4. 查看插件状态：
```bash
loquat> plugins
```

输出：
```
Loaded Plugins:
  - greeting_plugin v1.0.0 (Healthy)
    Type: Processor
    Author: Your Name
    Description: A simple greeting plugin
```

5. 更新插件配置（通过Web API）：
```bash
curl -X PATCH http://localhost:8080/api/plugins/greeting_plugin \
  -H "Content-Type: application/json" \
  -d '{"greeting_message": "你好！欢迎使用Loquat！"}'
```

### 4.7 热重载

Loquat支持插件的热重载，无需重启即可重新加载插件：

```bash
# 修改插件代码
# 重新编译
cd plugins/my_first_plugin
cargo build --release

# 在REPL中重新加载
loquat> plugins reload
```

或者等待自动热重载（如果启用）：

```toml
[plugins]
enable_hot_reload = true
hot_reload_interval = 5  # 每5秒检查一次
```

---

## 第5章：创建你的第一个适配器

### 5.1 适配器概念

适配器（Adapter）是连接外部系统和Loquat框架的桥梁。它负责：

- 从外部系统接收消息
- 将消息转换为Loquat事件格式
- 发送Loquat事件到外部系统
- 管理连接状态和统计信息

**为什么要使用适配器？**

- ✅ 统一接口：不同的消息平台通过统一接口接入
- ✅ 解耦系统：框架不依赖具体平台实现
- ✅ 易于扩展：添加新平台只需实现适配器
- ✅ 并发安全：适配器内部处理线程安全问题

### 5.2 Adapter trait

所有适配器都需要实现 `Adapter` trait：

```rust
use std::any::Any;
use std::fmt::Debug;
use tokio::sync::mpsc;

pub trait Adapter: Send + Sync + Debug + Any {
    // 基本信息
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn adapter_id(&self) -> &str;
    fn config(&self) -> AdapterConfig;
    
    // 状态查询（同步）
    fn status(&self) -> AdapterStatus;
    fn is_running(&self) -> bool;
    fn is_connected(&self) -> bool;
    fn statistics(&self) -> AdapterStatistics;
    
    // 事件发送
    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>);
    fn send_event(&self, event: EventEnum) -> Result<()>;
    
    // 类型转换
    fn as_any(&self) -> &dyn Any;
}
```

### 5.3 配置适配器

适配器配置文件位于 `adapters/` 目录：

```json
{
  "adapter_type": "my_adapter",
  "adapter_id": "my-adapter-001",
  "platform": "myplatform://config",
  "platform_config": {
    "enabled": true,
    "some_option": "value"
  }
}
```

### 5.4 创建一个简单的HTTP适配器

让我们创建一个简单的HTTP适配器，接收HTTP POST请求并转换为Loquat事件。

#### 步骤1：创建适配器文件

创建 `src/adapters/http_adapter.rs`：

```rust
//! HTTP Adapter - 接收HTTP POST请求并转换为Loquat事件

use crate::adapters::{
    Adapter, AdapterConfig, AdapterStatus, AdapterStatistics,
    config::AdapterConfig,
    types::AdapterStatistics,
};
use crate::events::{EventEnum, EventMetadata, message::MessageEvent};
use crate::errors::{AdapterError, LoquatError, Result};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// HTTP请求消息
#[derive(Debug, Deserialize, Serialize)]
pub struct HttpMessage {
    pub text: String,
    pub user_id: String,
    pub group_id: Option<String>,
}

/// HTTP Adapter
#[derive(Debug)]
pub struct HttpAdapter {
    config: AdapterConfig,
    status: Arc<RwLock<AdapterStatus>>,
    statistics: Arc<RwLock<AdapterStatistics>>,
    event_sender: Arc<RwLock<Option<mpsc::UnboundedSender<EventEnum>>>>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl HttpAdapter {
    /// Create a new HTTP adapter
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
            event_sender: Arc::new(RwLock::new(None)),
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<()> {
        let mut status = self.status.write().await;
        if *status == AdapterStatus::Running {
            return Err(LoquatError::Adapter(AdapterError::LoadFailed(
                "Adapter is already running".to_string()
            )));
        }

        *status = AdapterStatus::Starting;
        drop(status);

        // Get host and port from config
        let host = self.config.platform.get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1");
        let port = self.config.platform.get("port")
            .and_then(|v| v.as_u16())
            .unwrap_or(3000);

        let addr = format!("{}:{}", host, port);
        println!("[HttpAdapter] Starting HTTP server on {}", addr);

        // Clone references for the server task
        let event_sender_clone = Arc::clone(&self.event_sender);
        let adapter_id = self.config.adapter_id.clone();
        let statistics_clone = Arc::clone(&self.statistics);

        // Create router
        let app = Router::new()
            .route("/message", post(handle_message))
            .with_state((event_sender_clone, adapter_id, statistics_clone));

        // Start server
        let handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect("Failed to bind to address");
            
            println!("[HttpAdapter] Server listening on {}", addr);
            
            axum::serve(listener, app)
                .await
                .expect("Server error");
        });

        // Store handle
        *self.server_handle.write().await = Some(handle);
        
        // Update status
        *self.status.write().await = AdapterStatus::Running;
        
        println!("[HttpAdapter] Started successfully");
        Ok(())
    }

    /// Stop the HTTP server
    pub async fn stop(&self) -> Result<()> {
        println!("[HttpAdapter] Stopping...");
        
        // Abort server task if exists
        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
        }
        
        *self.status.write().await = AdapterStatus::Stopped;
        println!("[HttpAdapter] Stopped");
        Ok(())
    }
}

/// HTTP message handler
async fn handle_message(
    State((event_sender, adapter_id, statistics)): State<(
        Arc<RwLock<Option<mpsc::UnboundedSender<EventEnum>>>>,
        String,
        Arc<RwLock<AdapterStatistics>>,
    )>,
    Json(msg): Json<HttpMessage>,
) -> Json<serde_json::Value> {
    // Update statistics
    {
        let mut stats = statistics.write().await;
        stats.events_received += 1;
        stats.messages_sent += 1;
        stats.last_activity = Some(chrono::Utc::now().timestamp());
    }

    // Create event
    let metadata = EventMetadata::new("message.text")
        .with_source(crate::events::EventSource::User)
        .with_user_id(&msg.user_id)
        .with_group_id(msg.group_id.as_deref());

    let event = EventEnum::Message(MessageEvent::Text {
        text: msg.text,
        metadata,
    });

    // Send event
    if let Some(sender) = event_sender.read().await.as_ref() {
        if let Err(e) = sender.send(event) {
            eprintln!("[HttpAdapter] Failed to send event: {}", e);
            
            // Update error statistics
            let mut stats = statistics.write().await;
            stats.errors += 1;
            
            return Json(serde_json::json!({
                "success": false,
                "error": "Failed to process message"
            }));
        }
    }

    Json(serde_json::json!({
        "success": true,
        "message": "Message received"
    }))
}

impl Adapter for HttpAdapter {
    fn name(&self) -> &str {
        "HttpAdapter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn adapter_id(&self) -> &str {
        &self.config.adapter_id
    }

    fn config(&self) -> AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> AdapterStatus {
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.status.read());
            guard.clone()
        })
    }

    fn is_running(&self) -> bool {
        self.status() == AdapterStatus::Running
    }

    fn is_connected(&self) -> bool {
        self.status().is_active()
    }

    fn statistics(&self) -> AdapterStatistics {
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.statistics.read());
            guard.clone()
        })
    }

    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>) {
        tokio::task::block_in_place(|| {
            let mut guard = tokio::runtime::Handle::current()
                .block_on(self.event_sender.write());
            *guard = sender;
        });
    }

    fn send_event(&self, event: EventEnum) -> Result<()> {
        // For HTTP adapter, we don't send events back to HTTP clients
        // This could be implemented with websockets if needed
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

#### 步骤2：创建适配器工厂

创建 `src/adapters/http_factory.rs`：

```rust
//! HTTP Adapter Factory

use crate::adapters::{
    Adapter, AdapterConfig,
    config::AdapterConfig,
    http_adapter::HttpAdapter,
};
use crate::errors::{AdapterError, Result};

pub struct HttpAdapterFactory;

impl HttpAdapterFactory {
    pub fn create(config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        // Validate configuration
        if config.adapter_type != "http" {
            return Err(AdapterError::LoadFailed(
                "Invalid adapter type for HttpAdapterFactory".to_string()
            ).into());
        }

        let adapter = HttpAdapter::new(config);
        Ok(Box::new(adapter))
    }
}
```

#### 步骤3：更新模块导出

在 `src/adapters/mod.rs` 中添加：

```rust
pub mod http_adapter;
pub mod http_factory;

pub use http_adapter::HttpAdapter;
pub use http_factory::HttpAdapterFactory;
```

#### 步骤4：创建配置文件

在 `adapters/` 目录创建 `http.json`：

```json
{
  "adapter_type": "http",
  "adapter_id": "http-001",
  "platform": "http://server",
  "platform_config": {
    "host": "127.0.0.1",
    "port": 3000
  }
}
```

### 5.5 测试HTTP适配器

1. 启动Loquat：
```bash
start.bat
```

2. 发送测试消息：
```bash
curl -X POST http://localhost:3000/message \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Hello from HTTP!",
    "user_id": "user123",
    "group_id": "group456"
  }'
```

响应：
```json
{
  "success": true,
  "message": "Message received"
}
```

3. 查看适配器状态：
```bash
loquat> adapters
```

输出：
```
Loaded Adapters:
  - http-001 (Running)
    Type: HttpAdapter
    Platform: http://server
    Connected: true
    Events Sent: 0
    Events Received: 1
    Last Activity: 2024-01-01 12:00:00
```

---

## 第6章：使用事件系统

### 6.1 事件类型概览

Loquat支持多种事件类型，覆盖即时通讯的各种场景。

#### 事件枚举

```rust
pub enum EventEnum {
    Message(MessageEvent),
    Notice(NoticeEvent),
    Request(RequestEvent),
    Other(serde_json::Value),
}
```

### 6.2 创建文本事件

```rust
use loquat::events::{EventEnum, EventMetadata, message::MessageEvent};

let metadata = EventMetadata::new("message.text")
    .with_source(crate::events::EventSource::User)
    .with_user_id("user123")
    .with_group_id(Some("group456"));

let event = EventEnum::Message(MessageEvent::Text {
    text: "Hello, World!".to_string(),
    metadata,
});
```

### 6.3 创建图片事件

```rust
use loquat::events::{EventEnum, EventMetadata, message::MessageEvent};

let metadata = EventMetadata::new("message.image")
    .with_source(crate::events::EventSource::User)
    .with_user_id("user123");

let event = EventEnum::Message(MessageEvent::Image {
    url: "https://example.com/image.jpg".to_string(),
    caption: Some("A beautiful image".to_string()),
    metadata,
});
```

### 6.4 创建语音事件

```rust
use loquat::events::{EventEnum, EventMetadata, message::MessageEvent};

let metadata = EventMetadata::new("message.voice")
    .with_source(crate::events::EventSource::User)
    .with_user_id("user123");

let event = EventEnum::Message(MessageEvent::Voice {
    url: "https://example.com/voice.mp3".to_string(),
    duration: 30,  // 30 seconds
    metadata,
});
```

### 6.5 创建系统通知

```rust
use loquat::events::{EventEnum, EventMetadata, notice::NoticeEvent};

let metadata = EventMetadata::new("notice.system")
    .with_source(crate::events::EventSource::System);

let event = EventEnum::Notice(NoticeEvent::SystemNotice {
    notice_type: "maintenance".to_string(),
    content: "System will be down for maintenance at 3:00 AM".to_string(),
    metadata,
});
```

### 6.6 使用EventBuilder

EventBuilder提供了便捷的事件创建方法。

```rust
use loquat::events::EventBuilder;

// 创建文本事件
let event = EventBuilder::text_message("Hello, World!")
    .with_user_id("user123")
    .with_group_id("group456")
    .build();

// 创建图片事件
let event = EventBuilder::image_message("https://example.com/image.jpg")
    .with_caption("A beautiful image")
    .with_user_id("user123")
    .build();

// 创建语音事件
let event = EventBuilder::voice_message("https://example.com/voice.mp3", 30)
    .with_user_id("user123")
    .build();

// 创建系统通知
let event = EventBuilder::system_notice("maintenance", "System maintenance")
    .build();
```

### 6.7 处理事件

在插件或Worker中处理事件：

```rust
use loquat::events::{EventEnum, message::MessageEvent};

async fn handle_event(event: EventEnum) -> Result<()> {
    match event {
        EventEnum::Message(msg) => {
            match msg {
                MessageEvent::Text { text, metadata } => {
                    println!("Text message: {}", text);
                    println!("From user: {:?}", metadata.user_id);
                }
                MessageEvent::Image { url, caption, .. } => {
                    println!("Image: {}", url);
                    if let Some(cap) = caption {
                        println!("Caption: {}", cap);
                    }
                }
                MessageEvent::Voice { url, duration, .. } => {
                    println!("Voice: {} ({}s)", url, duration);
                }
                _ => {
                    println!("Other message type");
                }
            }
        }
        EventEnum::Notice(notice) => {
            println!("Notice: {:?}", notice);
        }
        _ => {
            println!("Other event type");
        }
    }
    Ok(())
}
```

### 6.8 事件过滤和转发

创建一个插件来过滤和转发事件：

```rust
pub struct EventFilterPlugin {
    allowed_users: Vec<String>,
    forward_channel: Option<mpsc::UnboundedSender<EventEnum>>,
}

impl EventFilterPlugin {
    async fn process_event(&mut self, event: EventEnum) -> Result<Option<EventEnum>> {
        match &event {
            EventEnum::Message(MessageEvent::Text { metadata, .. }) => {
                // 检查用户是否在允许列表中
                if let Some(user_id) = &metadata.user_id {
                    if !self.allowed_users.contains(user_id) {
                        println!("Filtered message from user: {}", user_id);
                        return Ok(None);  // 过滤掉
                    }
                }
            }
            _ => {}
        }
        
        // 转发事件
        if let Some(ref sender) = self.forward_channel {
            let _ = sender.send(event.clone());
        }
        
        Ok(Some(event))
    }
}
```

---

## 第7章：AOP和日志系统

### 7.1 AOP基础

AOP（面向切面编程）允许你在不修改原有代码的情况下，为服务添加横切关注点。

#### 切面（Aspect）

切面定义了在方法执行前后要执行的行为：

```rust
use loquat::aop::traits::{Aspect, JoinPoint, ExecutionResult};

#[async_trait::async_trait]
pub trait Aspect: Send + Sync {
    async fn before(&self, join_point: &JoinPoint) -> ExecutionResult<()>;
    
    async fn after(
        &self, 
        join_point: &JoinPoint, 
        result: &ExecutionResult<()>, 
        context: &LogContext
    ) -> ExecutionResult<()>;
}
```

### 7.2 创建日志切面

```rust
use loquat::aop::traits::{Aspect, JoinPoint, ExecutionResult};
use loquat::logging::traits::{Logger, LogLevel, LogContext};
use std::sync::Arc;

pub struct LoggingAspect {
    logger: Arc<dyn Logger>,
}

impl LoggingAspect {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }
}

#[async_trait::async_trait]
impl Aspect for LoggingAspect {
    async fn before(&self, join_point: &JoinPoint) -> ExecutionResult<()> {
        let mut context = LogContext::new();
        context.add("method", join_point.method_name);
        context.add("status", "before");
        
        self.logger.log(
            LogLevel::Info,
            &format!("Executing method: {}", join_point.method_name),
            &context
        );
        
        ExecutionResult::Continue(())
    }

    async fn after(
        &self, 
        join_point: &JoinPoint, 
        result: &ExecutionResult<()>, 
        context: &LogContext
    ) -> ExecutionResult<()> {
        let mut new_context = context.clone();
        new_context.add("method", join_point.method_name);
        new_context.add("status", "after");
        
        match result {
            ExecutionResult::Continue(_) => {
                self.logger.log(
                    LogLevel::Info,
                    &format!("Method {} completed successfully", join_point.method_name),
                    &new_context
                );
            }
            ExecutionResult::Return(_) => {
                self.logger.log(
                    LogLevel::Info,
                    &format!("Method {} returned early", join_point.method_name),
                    &new_context
                );
            }
            ExecutionResult::Error(e) => {
                new_context.add("error", e.to_string());
                self.logger.log(
                    LogLevel::Error,
                    &format!("Method {} failed: {}", join_point.method_name, e),
                    &new_context
                );
            }
        }
        
        ExecutionResult::Continue(())
    }
}
```

### 7.3 创建性能监控切面

```rust
use std::time::Instant;

pub struct PerformanceAspect {
    logger: Arc<dyn Logger>,
}

impl PerformanceAspect {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }
}

#[async_trait::async_trait]
impl Aspect for PerformanceAspect {
    async fn before(&self, join_point: &JoinPoint) -> ExecutionResult<()> {
        let start_time = Instant::now();
        
        // Store start time in context
        let mut context = LogContext::new();
        context.add("start_time", start_time.elapsed().as_millis());
        
        ExecutionResult::ContinueWithContext(context, ())
    }

    async fn after(
        &self, 
        join_point: &JoinPoint, 
        result: &ExecutionResult<()>, 
        context: &LogContext
    ) -> ExecutionResult<()> {
        let mut new_context = context.clone();
        new_context.add("method", join_point.method_name);
        
        // Calculate execution time
        let duration = if let Some(start) = context.get("start_time") {
            Instant::now().elapsed().as_millis() - start.parse::<u128>().unwrap_or(0)
        } else {
            0
        };
        
        new_context.add("duration_ms", duration);
        
        self.logger.log(
            LogLevel::Info,
            &format!("Method {} executed in {}ms", join_point.method_name, duration),
            &new_context
        );
        
        ExecutionResult::Continue(())
    }
}
```

### 7.4 应用切面

使用AopProxy应用切面：

```rust
use loquat::aop::proxy::AopProxy;
use std::sync::Arc;

struct MyService {
    name: String,
}

impl MyService {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    fn process(&self, data: &str) -> Result<String> {
        println!("Processing: {}", data);
        Ok(format!("Processed: {}", data))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create service
    let service = MyService::new("MyService");
    
    // Create aspects
    let logger = Arc::new(/* create logger */);
    let logging_aspect = Arc::new(LoggingAspect::new(logger.clone()));
    let performance_aspect = Arc::new(PerformanceAspect::new(logger));
    
    // Create proxy with multiple aspects
    let proxy = AopProxy::new_with_aspects(
        service,
        vec![logging_aspect, performance_aspect]
    );
    
    // Execute with AOP
    let result = proxy.execute("process", |svc| {
        svc.process("test data")
    }).await?;
    
    println!("Result: {}", result);
    Ok(())
}
```

### 7.5 日志系统

Loquat提供了灵活的日志系统，支持多种格式和输出。

#### 创建Logger

```rust
use loquat::logging::{
    logger::StructuredLogger,
    formatters::{TextFormatter, JsonFormatter},
    writers::{ConsoleWriter, FileWriter, CombinedWriter},
};
use std::sync::Arc;

// 文本格式 + 控制台输出
let formatter = Arc::new(TextFormatter::detailed());
let writer = Arc::new(ConsoleWriter::new());
let logger: Arc<dyn Logger> = Arc::new(StructuredLogger::new(formatter, writer));

// JSON格式 + 文件输出
let formatter = Arc::new(JsonFormatter::new());
let writer = Arc::new(FileWriter::new("logs/loquat.log"));
let logger: Arc<dyn Logger> = Arc::new(StructuredLogger::new(formatter, writer));

// 组合输出（控制台 + 文件）
let formatter = Arc::new(TextFormatter::detailed());
let console_writer = Arc::new(ConsoleWriter::new());
let file_writer = Arc::new(FileWriter::new("logs/loquat.log"));
let writer = Arc::new(CombinedWriter::new(console_writer, file_writer));
let logger: Arc<dyn Logger> = Arc::new(StructuredLogger::new(formatter, writer));
```

#### 使用Logger

```rust
use loquat::logging::traits::{Logger, LogLevel, LogContext};

// 创建上下文
let mut context = LogContext::new();
context.add("user_id", "user123");
context.add("action", "login");

// 记录日志
logger.log(LogLevel::Info, "User logged in", &context);

// 不同级别的日志
logger.log(LogLevel::Debug, "Debug message", &context);
logger.log(LogLevel::Info, "Info message", &context);
logger.log(LogLevel::Warn, "Warning message", &context);
logger.log(LogLevel::Error, "Error message", &context);
```

---

## 第8章：Web API和REPL

### 8.1 Web API服务

Loquat内置了基于Axum的RESTful API服务，支持远程管理。

#### 启动Web服务

在配置文件中启用：

```toml
[web]
enabled = true
host = "127.0.0.1"
port = 8080
```

启动Loquat后，Web服务会自动启动。

### 8.2 API端点

#### 健康检查

```bash
curl http://localhost:8080/health
```

响应：
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### 列出所有插件

```bash
curl http://localhost:8080/api/plugins
```

响应：
```json
{
  "plugins": [
    {
      "name": "greeting_plugin",
      "version": "1.0.0",
      "type": "Processor",
      "status": "Healthy"
    }
  ]
}
```

#### 获取特定插件信息

```bash
curl http://localhost:8080/api/plugins/greeting_plugin
```

响应：
```json
{
  "name": "greeting_plugin",
  "version": "1.0.0",
  "type": "Processor",
  "status": "Healthy",
  "author": "Your Name",
  "description": "A simple greeting plugin"
}
```

#### 重新加载插件

```bash
curl -X POST http://localhost:8080/api/plugins/reload
```

响应：
```json
{
  "success": true,
  "message": "All plugins reloaded"
}
```

#### 列出所有适配器

```bash
curl http://localhost:8080/api/adapters
```

响应：
```json
{
  "adapters": [
    {
      "adapter_id": "console-001",
      "type": "ConsoleAdapter",
      "platform": "console://stdin",
      "status": "Running",
      "connected": true,
      "statistics": {
        "events_sent": 0,
        "events_received": 10,
        "errors": 0
      }
    }
  ]
}
```

#### 获取配置

```bash
curl http://localhost:8080/api/config
```

响应：
```json
{
  "general": {
    "environment": "dev",
    "name": "Loquat Framework (Dev)"
  },
  "logging": {
    "level": "Debug",
    "format": "text",
    "output": "console"
  }
}
```

### 8.3 REPL命令

#### status

显示系统运行状态：

```
loquat> status
```

输出：
```
System Status:
  Environment: dev
  Engine: Running
  Plugins: 2 loaded
  Adapters: 3 loaded
  Web Service: Running (http://127.0.0.1:8080)
  Uptime: 1h 23m 45s
```

#### plugins

列出所有插件：

```
loquat> plugins
```

输出：
```
Loaded Plugins:
  1. greeting_plugin v1.0.0 (Healthy)
     Type: Processor
     Author: Your Name
     Description: A simple greeting plugin

  2. http_adapter v1.0.0 (Healthy)
     Type: Native
     Author: Your Name
     Description: HTTP adapter
```

重新加载插件：

```
loquat> plugins reload
```

#### adapters

列出所有适配器：

```
loquat> adapters
```

输出：
```
Loaded Adapters:
  1. console-001 (Running)
     Type: ConsoleAdapter
     Platform: console://stdin
     Connected: true
     Statistics:
       Events Sent: 0
       Events Received: 15
       Errors: 0

  2. echo-001 (Running)
     Type: EchoAdapter
     Platform: echo://test
     Connected: true
     Statistics:
       Events Sent: 15
       Events Received: 15
       Errors: 0
```

重新加载适配器：

```
loquat> adapters reload
```

#### config

显示当前配置：

```
loquat> config
```

输出：
```
Current Configuration:
  [general]
  environment = "dev"
  name = "Loquat Framework (Dev)"

  [logging]
  level = "Debug"
  format = "text"
  output = "console"

  [plugins]
  enabled = true
  auto_load = true
  enable_hot_reload = true

  [adapters]
  enabled = true
  auto_load = true
  enable_hot_reload = true

  [web]
  enabled = true
  host = "127.0.0.1"
  port = 8080
```

#### logs

查看日志：

```
loquat> logs
```

输出：
```
[2024-01-01 12:00:00] [INFO] Loquat started
[2024-01-01 12:00:01] [INFO] Loading plugins...
[2024-01-01 12:00:02] [INFO] Plugin greeting_plugin loaded
[2024-01-01 12:00:03] [INFO] Plugin http_adapter loaded
[2024-01-01 12:00:04] [INFO] Loading adapters...
[2024-01-01 12:00:05] [INFO] Adapter console-001 started
[2024-01-01 12:00:06] [INFO] Adapter echo-001 started
```

查看特定级别的日志：

```
loquat> logs error
```

#### engine

启动引擎：

```
loquat> engine start
```

停止引擎：

```
loquat> engine stop
```

#### clear

清除屏幕：

```
loquat> clear
```

#### help

显示帮助信息：

```
loquat> help
```

输出：
```
Available Commands:
  help              - Show this help message
  status            - Show system status
  plugins           - List all plugins
  plugins reload    - Reload all plugins
  adapters          - List all adapters
  adapters reload   - Reload all adapters
  config            - Show current configuration
  logs [level]      - View logs (optional: error, warn, info, debug)
  engine start      - Start the engine
  engine stop       - Stop the engine
  clear             - Clear the screen
  exit              - Exit REPL
```

---

## 第9章：配置和部署

### 9.1 配置系统

Loquat使用多环境配置系统，支持配置合并和覆盖。

#### 配置文件结构

```
config/
├── default.toml    # 默认配置（所有环境共用）
├── dev.toml        # 开发环境
├── test.toml       # 测试环境
└── prod.toml       # 生产环境
```

#### 配置合并规则

启动时，Loquat会按以下顺序合并配置：

1. 加载 `default.toml`
2. 加载指定环境的配置（如 `dev.toml`）
3. 环境特定的配置会覆盖默认配置

### 9.2 开发环境配置

```toml
# config/dev.toml

[general]
environment = "dev"
name = "Loquat Framework (Dev)"

[logging]
level = "Debug"              # 详细日志
format = "text"              # 可读格式
output = "console"           # 输出到控制台
file_path = "logs/loquat.log"

[plugins]
enabled = true               # 启用插件
auto_load = true             # 自动加载
enable_hot_reload = true      # 启用热重载
hot_reload_interval = 5       # 每5秒检查

[adapters]
enabled = true               # 启用适配器
auto_load = true             # 自动加载
enable_hot_reload = true      # 启用热重载
hot_reload_interval = 10     # 每10秒检查

[web]
enabled = true               # 启用Web服务
host = "127.0.0.1"           # 只监听本地
port = 8080                  # 开发端口

[engine]
pool_size = 4                # 较小的线程池
max_concurrent_tasks = 100   # 较少的并发任务
```

### 9.3 生产环境配置

```toml
# config/prod.toml

[general]
environment = "prod"
name = "Loquat Framework (Production)"

[logging]
level = "Warn"               # 只记录警告和错误
format = "json"              # JSON格式便于解析
output = "combined"           # 同时输出到文件和控制台
file_path = "logs/loquat.log"

[plugins]
enabled = true
auto_load = true
enable_hot_reload = false     # 生产环境禁用热重载
hot_reload_interval = 60

[adapters]
enabled = true
auto_load = true
enable_hot_reload = false     # 生产环境禁用热重载
hot_reload_interval = 60

[web]
enabled = true
host = "0.0.0.0"            # 监听所有网络接口
port = 8080                  # 生产端口

[engine]
pool_size = 8                # 较大的线程池
max_concurrent_tasks = 1000  # 更多的并发任务

[performance]
enable_metrics = true         # 启用性能监控
metrics_interval = 60         # 每60秒输出指标
```

### 9.4 日志配置

#### 日志级别

- `Debug`：最详细的日志，包含所有信息
- `Info`：一般信息，不包含调试细节
- `Warn`：警告信息
- `Error`：只记录错误

#### 日志格式

- `text`：人类可读的文本格式
- `json`：机器可解析的JSON格式

#### 日志输出

- `console`：只输出到控制台
- `file`：只输出到文件
- `combined`：同时输出到控制台和文件

### 9.5 热重载

Loquat支持插件和适配器的热重载，无需重启即可更新代码。

#### 启用热重载

```toml
[plugins]
enable_hot_reload = true
hot_reload_interval = 5  # 每5秒检查一次

[adapters]
enable_hot_reload = true
hot_reload_interval = 10  # 每10秒检查一次
```

#### 热重载工作流程

1. 修改插件或适配器代码
2. 重新编译
3. 等待热重载间隔（自动）
4. 或者手动触发重新加载

```bash
# 在REPL中手动重新加载
loquat> plugins reload
loquat> adapters reload

# 通过API重新加载
curl -X POST http://localhost:8080/api/plugins/reload
curl -X POST http://localhost:8080/api/adapters/reload
```

### 9.6 部署建议

#### 开发环境

1. 使用dev配置
2. 启用详细日志
3. 启用热重载
4. 使用小规模线程池

#### 测试环境

1. 使用test配置
2. 中等详细日志
3. 启用热重载
4. 中等规模线程池

#### 生产环境

1. 使用prod配置
2. 只记录警告和错误
3. 禁用热重载（通过外部进程管理）
4. 大规模线程池
5. 启用性能监控
6. 使用进程管理器（如systemd、supervisor）

#### 使用systemd部署

创建 `/etc/systemd/system/loquat.service`：

```ini
[Unit]
Description=Loquat Framework
After=network.target

[Service]
Type=simple
User=loquat
WorkingDirectory=/opt/loquat
ExecStart=/opt/loquat/target/release/loquat --environment prod
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
sudo systemctl enable loquat
sudo systemctl start loquat
sudo systemctl status loquat
```

查看日志：

```bash
sudo journalctl -u loquat -f
```

---

## 第10章：最佳实践和进阶

### 10.1 代码组织

#### 模块划分

```
src/
├── main.rs              # 主入口
├── config/              # 配置模块
├── engine/              # 引擎核心
├── plugins/             # 插件系统
├── adapters/            # 适配器系统
├── events/              # 事件定义
├── logging/             # 日志系统
├── aop/                 # AOP系统
└── utils/               # 工具函数
```

#### 命名规范

- **结构体**：`PascalCase`（如 `HttpAdapter`）
- **函数/方法**：`snake_case`（如 `handle_message`）
- **常量**：`SCREAMING_SNAKE_CASE`（如 `MAX_RETRIES`）
- **模块**：`snake_case`（如 `http_adapter`）

### 10.2 错误处理

#### 使用Result类型

```rust
use loquat::errors::{Result, LoquatError};

pub fn process_data(data: &str) -> Result<String> {
    if data.is_empty() {
        return Err(LoquatError::Validation(
            "Data cannot be empty".to_string()
        ));
    }
    Ok(format!("Processed: {}", data))
}
```

#### 错误传播

```rust
pub async fn handle_message(msg: Message) -> Result<()> {
    let data = extract_data(msg)?;  // ? 自动传播错误
    let processed = process_data(&data)?;
    send_response(&processed)?;
    Ok(())
}
```

#### 自定义错误

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<MyError> for LoquatError {
    fn from(err: MyError) -> Self {
        LoquatError::Custom(err.to_string())
    }
}
```

### 10.3 并发安全

#### 使用Arc共享所有权

```rust
use std::sync::Arc;

let data = Arc::new(MyData::new());
let data_clone = Arc::clone(&data);

tokio::spawn(async move {
    // 使用 data_clone
});
```

#### 使用RwLock读写锁

```rust
use tokio::sync::RwLock;

let state = Arc::new(RwLock::new(MyState::new()));

// 读取
{
    let guard = state.read().await;
    println!("State: {:?}", *guard);
}

// 写入
{
    let mut guard = state.write().await;
    guard.update();
}
```

#### 使用消息传递

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(100);

// 发送端
tokio::spawn(async move {
    tx.send("message".to_string()).await.unwrap();
});

// 接收端
while let Some(msg) = rx.recv().await {
    println!("Received: {}", msg);
}
```

### 10.4 性能优化

#### 避免阻塞

```rust
// ❌ 错误：阻塞异步任务
pub fn blocking_operation() -> String {
    std::thread::sleep(std::time::Duration::from_secs(1));
    "done".to_string()
}

// ✅ 正确：使用spawn_blocking
pub async fn async_operation() -> String {
    tokio::task::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        "done".to_string()
    }).await.unwrap()
}
```

#### 批量处理

```rust
// ❌ 错误：逐个处理
for item in items {
    process_item(item).await?;
}

// ✅ 正确：批量处理
let handles: Vec<_> = items.into_iter()
    .map(|item| tokio::spawn(async move {
        process_item(item).await
    }))
    .collect();

for handle in handles {
    handle.await??;
}
```

#### 缓存结果

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct Cache<T> {
    data: Arc<RwLock<HashMap<String, T>>>,
}

impl<T: Clone> Cache<T> {
    pub async fn get_or_insert<F, Fut>(&self, key: &str, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // 先尝试从缓存读取
        {
            let guard = self.data.read().await;
            if let Some(value) = guard.get(key) {
                return Ok(value.clone());
            }
        }
        
        // 缓存未命中，计算并存储
        let value = f().await?;
        let mut guard = self.data.write().await;
        guard.insert(key.to_string(), value.clone());
        Ok(value)
    }
}
```

### 10.5 测试策略

#### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_data() {
        let result = process_data("test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: test");
    }

    #[test]
    fn test_process_data_empty() {
        let result = process_data("");
        assert!(result.is_err());
    }
}
```

#### 集成测试

```rust
#[tokio::test]
async fn test_plugin_lifecycle() {
    let mut plugin = MyPlugin::new();
    
    // 初始化
    assert!(plugin.init().await.is_ok());
    
    // 加载
    assert!(plugin.load().await.is_ok());
    
    // 卸载
    assert!(plugin.unload().await.is_ok());
}
```

#### 并发测试

```rust
#[tokio::test]
async fn test_concurrent_access() {
    let state = Arc::new(RwLock::new(0i32));
    let mut handles = vec![];
    
    for _ in 0..100 {
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            let mut guard = state_clone.write().await;
            *guard += 1;
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let guard = state.read().await;
    assert_eq!(*guard, 100);
}
```

### 10.6 常见问题

#### Q1: 如何调试异步代码？

```rust
// 使用日志记录
logger.log(LogLevel::Debug, "Starting operation", &context);

// 使用tokio-console
// 在Cargo.toml中添加：
// [dependencies]
// console-subscriber = "0.1"

// 在main.rs中：
use console_subscriber::ConsoleLayer;

#[tokio::main]
async fn main() {
    ConsoleLayer::builder().init();
    // ... your code
}

// 运行：
// tokio-console
```

#### Q2: 如何处理大量并发请求？

```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(100)); // 最多100个并发

for request in requests {
    let permit = semaphore.clone().acquire_owned().await.unwrap();
    
    tokio::spawn(async move {
        let _permit = permit; // 持有permit直到任务完成
        process_request(request).await;
    });
}
```

#### Q3: 如何优雅地关闭？

```rust
use tokio::signal;

// 监听Ctrl+C
#[tokio::main]
async fn main() -> Result<()> {
    tokio::select! {
        _ = run_server() => {}
        _ = signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            shutdown().await?;
        }
    }
    Ok(())
}

async fn shutdown() -> Result<()> {
    // 1. 停止接受新请求
    // 2. 等待现有请求完成
    // 3. 保存状态
    // 4. 关闭连接
    Ok(())
}
```

#### Q4: 如何监控性能？

```rust
use std::time::Instant;

pub async fn with_timing<F, Fut, T>(f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    
    println!("Operation took: {:?}", duration);
    result
}

// 使用
let result = with_timing(|| {
    expensive_operation().await
}).await;
```

---

## 附录

### A. 完整示例项目

一个简单的聊天机器人示例：

```rust
// plugins/chat_bot.rs

use async_trait::async_trait;
use loquat::plugins::{Plugin, PluginHealth, PluginType};
use loquat::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct BotCommand {
    pub command: String,
    pub response: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub commands: Vec<BotCommand>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commands: vec![
                BotCommand {
                    command: "hello".to_string(),
                    response: "你好！很高兴见到你！".to_string(),
                },
                BotCommand {
                    command: "time".to_string(),
                    response: "现在是".to_string(),
                },
            ],
        }
    }
}

pub struct ChatBotPlugin {
    name: String,
    version: String,
    config: Config,
    command_map: HashMap<String, String>,
    message_count: u64,
}

impl ChatBotPlugin {
    pub fn new() -> Self {
        let config = Config::default();
        let command_map: HashMap<_, _> = config.commands
            .iter()
            .map(|cmd| (cmd.command.clone(), cmd.response.clone()))
            .collect();
        
        Self {
            name: "ChatBotPlugin".to_string(),
            version: "1.0.0".to_string(),
            config,
            command_map,
            message_count: 0,
        }
    }

    pub fn process_message(&mut self, text: &str) -> Option<String> {
        self.message_count += 1;
        
        // 查找命令
        if let Some(response) = self.command_map.get(text) {
            if text == "time" {
                return Some(format!("{} {}", response, chrono::Local::now().format("%H:%M:%S")));
            }
            return Some(response.clone());
        }
        
        None
    }
}

#[async_trait]
impl Plugin for ChatBotPlugin {
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
        println!("🤖 ChatBotPlugin initialized with {} commands", 
            self.command_map.len());
        Ok(())
    }

    async fn load(&mut self) -> Result<()> {
        println!("✅ ChatBotPlugin loaded!");
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        println!("👋 ChatBotPlugin unloaded! Processed {} messages", 
            self.message_count);
        Ok(())
    }

    fn health_status(&self) -> PluginHealth {
        PluginHealth::Healthy
    }

    async fn update_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let Ok(new_config) = serde_json::from_value::<Config>(config) {
            self.config = new_config;
            self.command_map = self.config.commands
                .iter()
                .map(|cmd| (cmd.command.clone(), cmd.response.clone()))
                .collect();
            println!("📝 ChatBotPlugin config updated!");
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = ChatBotPlugin::new();
    Box::into_raw(Box::new(plugin))
}

#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}
```

### B. 参考资源

- **Loquat GitHub**: https://github.com/Full-finger/Loquat
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Documentation**: https://tokio.rs/
- **Axum Documentation**: https://docs.rs/axum/

### C. 贡献指南

欢迎提交Issue和Pull Request！

1. Fork项目
2. 创建特性分支
3. 提交更改
4. 推送到分支
5. 创建Pull Request

---

## 结语

恭喜你完成了Loquat框架的新手教程！你现在应该已经掌握了：

- ✅ Loquat框架的核心概念和架构
- ✅ 如何创建和部署插件
- ✅ 如何创建和配置适配器
- ✅ 如何使用事件系统
- ✅ 如何使用AOP和日志系统
- ✅ 如何使用Web API和REPL
- ✅ 如何配置和部署应用
- ✅ 最佳实践和进阶技巧

继续探索Loquat框架，构建你自己的机器人/代理应用吧！

如有问题，欢迎在GitHub上提交Issue。

**Happy Coding! 🎉**
