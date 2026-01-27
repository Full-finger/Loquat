# Loquat项目完整开发路线图

## 📋 项目状态总览

### ✅ 已完成
- **loquat-tool**: CLI工具完全可用，支持创建/管理插件和适配器
- **src**: 框架核心编译通过（80个警告，不影响功能）
- **proto定义**: gRPC协议定义完整

### ❌ 待修复
- **loquat-kernel**: 52个编译错误，需要修复
- **loquat-engine**: 需要protoc编译器
- **workspace配置**: kernel和engine被注释掉

---

## 🔧 P0 - 立即解决（本周完成）

### 1. 配置protoc编译器

**问题**: protoc不在系统PATH中，无法编译proto文件

**解决方案A**: 添加到系统PATH（推荐）
```powershell
# 临时添加（当前会话）
$env:PATH += ";C:\Program Files\MATLAB\R2024b\bin\win64"

# 永久添加（系统环境变量）
# 右键"此电脑" -> 属性 -> 高级系统设置 -> 环境变量
# 在Path中添加: C:\Program Files\MATLAB\R2024b\bin\win64
```

**解决方案B**: 修改build.rs指定protoc路径
```rust
// loquat-engine/build.rs
fn main() {
    let protoc_path = "C:\\Program Files\\MATLAB\\R2024b\\bin\\win64\\protoc.exe";
    tonic_build::configure()
        .protoc_path(protoc_path)
        .compile(&["proto/common.proto", "proto/kernel.proto"], &["proto/"])
        .expect("Failed to compile protos");
}
```

**解决方案C**: 使用cargo安装protoc
```bash
cargo install protoc
```

### 2. 修复workspace配置

**问题**: loquat-kernel和loquat-engine被注释掉

**修复**: 在根Cargo.toml中重新启用
```toml
[workspace]
resolver = "2"
members = [
    "loquat-tool",
    "loquat-kernel",  # 重新启用
    "loquat-engine",  # 重新启用
]
```

### 3. 验证编译

```bash
# 1. 验证protoc
protoc --version

# 2. 编译proto文件
cd proto
cargo build

# 3. 编译loquat-kernel
cd ../loquat-kernel
cargo build

# 4. 编译loquat-engine
cd ../loquat-engine
cargo build

# 5. 全部编译
cd ..
cargo build
```

---

## 🐛 P1 - 修复编译错误（1-2周）

### loquat-kernel（52个错误）

#### 1. Result<T>类型冲突

**问题**: 多个Result类型定义冲突

**修复**: 统一使用workspace的Result类型
```rust
// 修改前
use std::result::Result;

// 修改后
use crate::errors::Result;
```

#### 2. EngineInfo字段不匹配

**问题**: proto使用`engine_id`，代码使用`id`

**修复**: 统一字段命名
```rust
// proto/common.proto
message EngineInfo {
    string id = 1;  // 改为 id（或代码改为 engine_id）
    // ...
}

// 或修改代码
engine_info.id  // 改为 engine_info.engine_id
```

**推荐**: 修改proto文件，因为`id`更简洁

#### 3. 缺少字段

**问题**: SystemInfo缺少`http_address`, `grpc_address`, `max_engines`

**修复**: 添加到proto
```protobuf
// proto/kernel.proto
message SystemInfo {
    string version = 1;
    string kernel_id = 2;
    google.protobuf.Timestamp start_time = 3;
    google.protobuf.Timestamp uptime = 4;
    int32 engine_count = 5;
    int32 max_engines = 6;
    string http_address = 7;      // 新增
    string grpc_address = 8;      // 新增
}
```

#### 4. EngineStatus::Error应为结构体

**问题**: 枚举不能包含复杂信息

**修复**: 修改proto定义
```protobuf
// proto/common.proto
message EngineStatusMessage {
    loquat.common.EngineStatus status = 1;
    string error_message = 2;  // 新增
    google.protobuf.Timestamp timestamp = 3;
    string pid = 4;
}

// 修改KernelService返回类型
rpc GetEngineStatus(EngineId) returns (EngineStatusMessage);
```

#### 5. 异步调用错误

**问题**: 同步函数使用了`.await`

**修复**: 移除`.await`或改为异步函数
```rust
// 修改前
let result = sync_function().await;

// 修改后
let result = sync_function();

// 或改为异步
async fn sync_function() -> Result<T> {
    // ...
}
```

#### 6. RwLockWriteGuard缺少get_mut

**问题**: 使用了不存在的方法

**修复**: 直接使用guard
```rust
// 修改前
let guard = lock.write().await;
let value = guard.get_mut();

// 修改后
let mut guard = lock.write().await;
let value = &mut *guard;
```

#### 7. child.kill()需要await

**问题**: 同步方法不需要await

**修复**: 移除await
```rust
// 修改前
child.kill().await?;

// 修改后
child.kill()?;
```

### loquat-engine编译错误

**预期错误**: 与kernel类似的Result类型、字段不匹配等问题

**修复策略**: 参考kernel的修复方案

---

## 📊 P2 - 完善核心功能（2-3周）

### loquat-kernel功能增强

#### 1. 进程管理（process_manager.rs）

**当前状态**: 基本启动/停止

**待完善**:
```rust
// 健康检查
async fn health_check(&self, pid: u32) -> bool;

// 优雅关闭
async fn graceful_shutdown(&self, pid: u32) -> Result<()>;

// 资源监控
async fn monitor_resources(&self, pid: u32) -> ResourceUsage;

// 信号处理
async fn send_signal(&self, pid: u32, signal: Signal) -> Result<()>;
```

#### 2. Engine管理器（engine/mod.rs）

**当前状态**: 基本注册/注销

**待完善**:
```rust
// 端口分配
async fn allocate_port(&self) -> Result<u16>;

// 状态跟踪
async fn track_status(&self, engine_id: &str) -> Result<()>;

// 健康检查
async fn health_check(&self, engine_id: &str) -> Result<bool>;

// 自动重启
async fn auto_restart(&self, engine_id: &str) -> Result<()>;
```

#### 3. 监控器（monitor/mod.rs）

**当前状态**: 基本监控框架

**待完善**:
```rust
// 指标收集
async fn collect_metrics(&self) -> Metrics;

// 告警系统
async fn check_alerts(&self, metrics: &Metrics) -> Vec<Alert>;

// 事件处理
async fn handle_event(&self, event: MonitoringEvent);
```

#### 4. gRPC服务器（grpc_server.rs）

**当前状态**: 部分实现

**待实现**:
```rust
// 需要实现的13个RPC方法
- RegisterEngine        ✅ 可能已实现
- UnregisterEngine      ⚠️ 需要检查
- RestartEngine         ❌ 待实现
- StopEngine           ❌ 待实现
- GetEngineStatus       ❌ 待实现
- ListEngines          ❌ 待实现
- GetEngineInfo        ❌ 待实现
- HealthCheck          ❌ 待实现
- GetConfig            ❌ 待实现
- UpdateConfig         ❌ 待实现
- ReloadConfig         ❌ 待实现
- StreamMetrics        ❌ 待实现（流式）
- StreamLogs           ❌ 待实现（流式）
- GetSystemInfo        ❌ 待实现
- Shutdown             ❌ 待实现
```

#### 5. HTTP服务器（http_server.rs）

**当前状态**: 可能未实现

**待实现的REST API**:
```
GET    /api/health              - 健康检查
GET    /api/engines            - 列出所有Engine
POST   /api/engines            - 创建新Engine
GET    /api/engines/:id        - 获取Engine详情
DELETE /api/engines/:id        - 删除Engine
POST   /api/engines/:id/restart - 重启Engine
GET    /api/config             - 获取配置
PUT    /api/config             - 更新配置
GET    /api/system/info        - 系统信息
```

### loquat-engine功能实现

#### 1. Engine核心（engine.rs）

**待实现**:
```rust
// 插件加载器
struct PluginLoader {
    async fn load(&self, path: &str) -> Result<Box<dyn Plugin>>;
    async fn unload(&self, name: &str) -> Result<()>;
    async fn reload(&self, name: &str) -> Result<()>;
}

// 事件处理器
struct EventHandler {
    async fn process(&self, event: Event) -> Result<EventResult>;
    async fn send(&self, event: Event) -> Result<()>;
}

// 适配器管理器
struct AdapterManager {
    async fn load(&self, path: &str) -> Result<Box<dyn Adapter>>;
    async fn unload(&self, name: &str) -> Result<()>;
}

// 配置管理
struct ConfigManager {
    async fn load(&self) -> Result<Config>;
    async fn update(&self, config: Config) -> Result<()>;
}
```

#### 2. Kernel客户端（kernel_client.rs）

**待实现**:
```rust
struct KernelClient {
    async fn register(&self, engine_info: EngineInfo) -> Result<String>;
    async fn unregister(&self, engine_id: &str) -> Result<()>;
    async fn heartbeat(&self, engine_id: &str) -> Result<()>;
    async fn stream_metrics(&self) -> impl Stream<Metric>;
}
```

---

## 🎯 P3 - 高级功能（3-4周）

### 1. 配置热重载
```rust
async fn watch_config(&self) -> Result<()>;
async fn reload_config(&self) -> Result<()>;
```

### 2. 指标导出
```rust
// Prometheus
async fn serve_metrics(&self) -> Result<()>;

// OpenTelemetry
async fn setup_tracing(&self) -> Result<()>;
```

### 3. 分布式追踪
```rust
// Jaeger/Zipkin
async fn setup_jaeger(&self) -> Result<()>;
```

### 4. 健康检查增强
```rust
struct HealthChecker {
    async fn check_database(&self) -> bool;
    async fn check_redis(&self) -> bool;
    async fn check_external_services(&self) -> Vec<ServiceStatus>;
}
```

### 5. 自动扩缩容
```rust
struct AutoScaler {
    async fn should_scale_up(&self, metrics: &Metrics) -> bool;
    async fn should_scale_down(&self, metrics: &Metrics) -> bool;
    async fn scale(&self, count: u32) -> Result<()>;
}
```

---

## 🧪 P4 - 测试完善（持续进行）

### 单元测试

**目标覆盖率**: 80%

```rust
// 核心模块测试
#[cfg(test)]
mod tests {
    #[test]
    fn test_engine_registration() {
        // 测试Engine注册
    }

    #[tokio::test]
    async fn test_grpc_client() {
        // 测试gRPC客户端
    }
}
```

### 集成测试

```rust
// tests/integration/kernel_engine_integration_test.rs
#[tokio::test]
async fn test_kernel_engine_communication() {
    // 测试Kernel和Engine通信
}

#[tokio::test]
async fn test_plugin_lifecycle() {
    // 测试插件生命周期
}
```

### 端到端测试

```bash
# 启动Kernel
cargo run --bin loquat-kernel &

# 启动Engine
cargo run --bin loquat-engine &

# 运行测试
cargo test --test e2e
```

---

## 📚 P5 - 文档完善（持续进行）

### API文档

#### gRPC文档
```markdown
# Kernel gRPC API

## 服务方法

### RegisterEngine
注册新Engine到Kernel

**请求**: EngineInfo
**响应**: EngineId
**示例**:
```rust
let engine_info = EngineInfo {
    id: "engine-001".to_string(),
    name: "Test Engine".to_string(),
    version: "0.1.0".to_string(),
    host: "127.0.0.1".to_string(),
    port: 50052,
    metadata: HashMap::new(),
};

let response = client.register_engine(engine_info).await?;
```
```

#### HTTP REST API文档
```markdown
# Kernel HTTP API

## 端点

### GET /api/health
健康检查

**响应**:
```json
{
  "status": "healthy",
  "version": "0.2.0"
}
```
```

### 用户文档

#### 快速开始
```markdown
# 快速开始

## 安装

```bash
# 克隆仓库
git clone https://github.com/Full-finger/Loquat.git
cd Loquat

# 构建项目
cargo build --release
```

## 启动Kernel

```bash
# 使用默认配置
cargo run --bin loquat-kernel

# 指定配置文件
cargo run --bin loquat-kernel -- --config custom.toml
```

## 启动Engine

```bash
cargo run --bin loquat-engine
```

## 创建插件

```bash
# 使用loquat-tool
cargo run --bin loquat-tool -- new plugin --name my_plugin --type rust
```
```

### 架构文档

```markdown
# 系统架构

## 整体架构

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ HTTP/gRPC
       ▼
┌─────────────────────┐
│  loquat-kernel    │  微内核层
│  - 进程管理       │
│  - 监控          │
│  - gRPC/HTTP API  │
└────────┬──────────┘
         │ gRPC
         ▼
┌─────────────────────┐
│  loquat-engine     │  引擎层
│  - 插件系统       │
│  - 事件处理       │
│  - 适配器管理     │
└─────┬─────┬──────┘
      │     │
      ▼     ▼
┌────────┐ ┌──────────┐
│ Plugins│ │ Adapters │
└────────┘ └──────────┘
```
```

---

## 🚀 开发时间线

### 第1周
- [ ] 配置protoc
- [ ] 修复workspace配置
- [ ] 编译kernel和engine
- [ ] 修复P0错误（Result类型、字段不匹配）

### 第2周
- [ ] 完成kernel所有RPC方法实现
- [ ] 完成engine核心功能
- [ ] 基本功能测试

### 第3-4周
- [ ] HTTP REST API实现
- [ ] 监控和指标收集
- [ ] 配置热重载
- [ ] 健康检查增强

### 第5-6周
- [ ] 单元测试覆盖率达到80%
- [ ] 集成测试完善
- [ ] 端到端测试
- [ ] 文档完善

### 第7-8周
- [ ] 性能优化
- [ ] 安全加固
- [ ] 生产环境部署
- [ ] 插件生态建设

---

## 🛠️ 开发工具链

### 代码质量
```bash
# 格式化代码
cargo fmt

# 代码检查
cargo clippy -- -D warnings

# 运行测试
cargo test

# 文档生成
cargo doc --open
```

### 性能分析
```bash
# 基准测试
cargo bench

# 性能分析
cargo flamegraph

# 内存检查
valgrind ./target/release/loquat-kernel
```

### CI/CD
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test
      - run: cargo clippy
```

---

## 📝 提交规范

### Commit Message格式
```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type类型
- `feat`: 新功能
- `fix`: Bug修复
- `docs`: 文档更新
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建/工具相关

### 示例
```
feat(kernel): implement RestartEngine RPC method

- Add restart method to KernelService
- Implement process restart logic
- Add unit tests

Closes #123
```

---

## 🤝 贡献指南

### 1. Fork仓库
```bash
# 在GitHub上Fork仓库
git clone https://github.com/YOUR_USERNAME/Loquat.git
cd Loquat
git remote add upstream https://github.com/Full-finger/Loquat.git
```

### 2. 创建分支
```bash
git checkout -b feature/your-feature
```

### 3. 提交更改
```bash
git add .
git commit -m "feat: add your feature"
git push origin feature/your-feature
```

### 4. 创建Pull Request
- 在GitHub上创建PR
- 填写PR模板
- 等待review

---

## 📞 获取帮助

### 文档
- [项目README](README.md)
- [API文档](docs/api/)
- [架构文档](docs/architecture/)
- [教程](docs/tutorials/)

### 社区
- [GitHub Issues](https://github.com/Full-finger/Loquat/issues)
- [GitHub Discussions](https://github.com/Full-finger/Loquat/discussions)

### 快速链接
- [Proto定义](proto/)
- [配置示例](config/)
- [测试用例](tests/)

---

## ✅ 检查清单

### 提交前检查
- [ ] 代码通过`cargo clippy`
- [ ] 所有测试通过`cargo test`
- [ ] 代码格式化`cargo fmt`
- [ ] 更新相关文档
- [ ] 添加必要的测试

### 发布前检查
- [ ] 版本号更新
- [ ] CHANGELOG.md更新
- [ ] 所有文档更新
- [ ] 性能测试通过
- [ ] 安全审计完成

---

## 🎓 学习资源

### Rust
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Rust Book](https://rust-lang.github.io/async-book/)

### gRPC
- [tonic Documentation](https://docs.rs/tonic/)
- [Protocol Buffers Guide](https://protobuf.dev/)

### 最佳实践
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://matklad.github.io/2021/10/16/how-to-learn-rust.html)

---

**最后更新**: 2026-01-26
**维护者**: Loquat Contributors
**版本**: 0.2.0
