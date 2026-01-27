# Loquat Kernel

微内核层，负责管理Engine生命周期、进程监控、资源分配。

## 📋 概述

Loquat Kernel是Loquat项目的核心管理组件，提供以下功能：
- Engine进程生命周期管理
- gRPC和HTTP API接口
- 健康检查和监控
- 自动重启和故障恢复
- 配置管理

## 🚀 快速开始

### 构建项目

```bash
cd loquat-kernel
cargo build --release
```

### 运行Kernel

```bash
cargo run --bin loquat-kernel
```

或指定配置文件：

```bash
cargo run --bin loquat-kernel -- --config custom.toml
```

## 📁 项目结构

```
loquat-kernel/
├── Cargo.toml                    # 项目配置
├── config/
│   └── kernel.toml             # 默认配置
├── src/
│   ├── lib.rs                  # 库入口
│   ├── config.rs               # 配置管理
│   ├── kernel/mod.rs            # Kernel核心
│   ├── engine/mod.rs            # Engine管理器
│   ├── monitor/mod.rs          # 监控器
│   ├── api/mod.rs              # API服务器框架
│   ├── grpc_server.rs          # gRPC服务实现
│   ├── http_server.rs          # HTTP服务实现
│   └── process_manager.rs     # 进程管理器
└── README.md
```

## 🔧 配置

### 配置文件示例

```toml
[kernel]
name = "Loquat Kernel"
host = "127.0.0.1"
grpc_port = 50051
http_port = 3000

[engine]
default_host = "127.0.0.1"
default_port_range = [50052, 60000]
auto_restart = true
restart_delay_seconds = 5

[monitoring]
health_check_interval = 10
metric_collection_interval = 60
enable_auto_restart = true
restart_failure_threshold = 3

[logging]
level = "info"
file = "./logs/kernel.log"
max_size_mb = 100
```

### 配置说明

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `kernel.name` | Kernel名称 | "Loquat Kernel" |
| `kernel.host` | 监听地址 | "127.0.0.1" |
| `kernel.grpc_port` | gRPC端口 | 50051 |
| `kernel.http_port` | HTTP端口 | 3000 |
| `engine.auto_restart` | 自动重启失败Engine | true |
| `monitoring.health_check_interval` | 健康检查间隔（秒） | 10 |
| `monitoring.enable_auto_restart` | 启用自动重启 | true |

## 📡 API接口

### gRPC接口

**地址**: `127.0.0.1:50051`

主要方法：
- `RegisterEngine` - 注册新Engine
- `UnregisterEngine` - 注销Engine
- `RestartEngine` - 重启Engine
- `StopEngine` - 停止Engine
- `GetEngineStatus` - 获取Engine状态
- `ListEngines` - 列出所有Engine
- `HealthCheck` - 健康检查
- `GetConfig` - 获取配置
- `SetConfig` - 更新配置

详细API文档参见 [proto/kernel.proto](../proto/kernel.proto)

### HTTP REST API

**地址**: `http://127.0.0.1:3000`

主要端点：
- `GET /api/health` - 健康检查
- `GET /api/engines` - 列出所有Engine
- `POST /api/engines` - 创建新Engine
- `GET /api/engines/:id` - 获取Engine详情
- `DELETE /api/engines/:id` - 删除Engine
- `POST /api/engines/:id/restart` - 重启Engine
- `GET /api/config` - 获取配置
- `PUT /api/config` - 更新配置
- `GET /api/system/info` - 系统信息

#### API示例

**健康检查**:
```bash
curl http://127.0.0.1:3000/api/health
```

**列出所有Engine**:
```bash
curl http://127.0.0.1:3000/api/engines
```

**创建新Engine**:
```bash
curl -X POST http://127.0.0.1:3000/api/engines \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Engine",
    "port": 50052,
    "command": "loquat-engine"
  }'
```

## 🔍 监控和日志

### 健康检查

Kernel会定期检查所有Engine的健康状态：
- 检查进程是否运行
- 检查响应时间
- 自动重启失败的Engine（如果启用）

### 日志

日志文件位置：`logs/kernel.log`

日志级别可通过配置文件调整：
- `trace` - 最详细的日志
- `debug` - 调试信息
- `info` - 一般信息（默认）
- `warn` - 警告信息
- `error` - 错误信息

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_kernel_creation

# 带输出的测试
cargo test -- --nocapture
```

## 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

## 📄 许可证

本项目采用MIT许可证 - 详见 [LICENSE](../LICENSE) 文件

## 🔗 相关链接

- [主项目文档](../README.md)
- [API文档](../docs/api/WEB_API_DOCUMENTATION.md)
- [实现进度](../IMPLEMENTATION_PROGRESS.md)
- [proto定义](../proto/)

## 💡 常见问题

### Q: 如何修改监听端口？

修改配置文件中的 `kernel.grpc_port` 和 `kernel.http_port`。

### Q: 如何禁用自动重启？

设置 `monitoring.enable_auto_restart = false`。

### Q: 如何查看Engine日志？

Engine进程的日志由Engine自己管理，通常在其工作目录的 `logs/` 文件夹中。

### Q: Kernel崩溃了怎么办？

1. 检查日志文件：`logs/kernel.log`
2. 验证配置文件格式
3. 检查端口是否被占用
4. 如果是Bug，请提交Issue

---

*最后更新: 2026-01-26*
