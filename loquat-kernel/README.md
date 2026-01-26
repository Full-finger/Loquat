# Loquat Kernel - 微内核

## 概述

Loquat Kernel是Loquat框架的微内核组件，负责管理Engine进程的生命周期、监控健康状态、协调资源分配。

## 架构

```
┌─────────────────────────────────────────┐
│       Loquat Kernel                │
│  ┌─────────────────────────────┐    │
│  │   Engine Manager          │    │
│  │  - 进程注册               │    │
│  │  - 状态管理               │    │
│  │  - 端口分配               │    │
│  └──────────┬──────────────┘    │
│             │                      │
│  ┌──────────▼──────────────┐    │
│  │   Monitor                │    │
│  │  - 健康检查               │    │
│  │  - 指标收集               │    │
│  │  - 自动重启               │    │
│  └──────────┬──────────────┘    │
│             │                      │
│  ┌──────────▼──────────────┐    │
│  │   API Server             │    │
│  │  - gRPC Service          │    │
│  │  - HTTP API               │    │
│  └──────────────────────────┘    │
└─────────────────────────────────────────┘
             │
             ▼
        loquat-engine(s)
```

## 核心功能

### 1. Engine管理
- 注册新的Engine实例
- 管理Engine状态（启动、运行、停止、错误）
- 自动分配端口号
- Engine数量限制

### 2. 进程监控
- 定期健康检查
- 指标收集和聚合
- 自动重启失败的Engine
- 故障阈值管理

### 3. API服务
- **gRPC服务**：高性能进程间通信
  - Engine生命周期管理
  - 健康检查
  - 配置管理
  - 指标流式传输
  
- **HTTP服务**：RESTful API
  - Engine管理端点
  - 配置查询
  - 日志查看
  - Web界面支持

### 4. 资源管理
- 内存限制
- CPU使用限制
- 网络访问控制
- 沙箱隔离

## 配置

配置文件：`config/kernel.toml`

```toml
[kernel]
bind_address = "127.0.0.1:50051"  # gRPC服务地址
web_address = "127.0.0.1:8080"    # HTTP服务地址
max_engines = 10                  # 最大Engine数量

[monitoring]
health_check_interval = 30          # 健康检查间隔（秒）
enable_auto_restart = true           # 启用自动重启
```

## gRPC接口

### KernelService

```protobuf
service KernelService {
    rpc RegisterEngine(EngineInfo) returns (EngineId);
    rpc UnregisterEngine(EngineId) returns (Empty);
    rpc RestartEngine(EngineId) returns (EngineStatus);
    rpc HealthCheck(EngineId) returns (HealthStatus);
    rpc StreamMetrics(Empty) returns (stream Metric);
    rpc StreamLogs(EngineId) returns (stream LogEntry);
}
```

## HTTP API端点

| 方法 | 端点 | 描述 |
|------|--------|------|
| GET | /api/engines | 列出所有Engine |
| POST | /api/engines | 创建新Engine |
| GET | /api/engines/:id | 获取Engine详情 |
| DELETE | /api/engines/:id | 停止Engine |
| POST | /api/engines/:id/restart | 重启Engine |
| GET | /api/config | 获取配置 |
| PUT | /api/config | 更新配置 |

## 当前实现状态

### ✅ 已完成
- [x] Proto定义（common.proto, kernel.proto, engine.proto）
- [x] 项目结构和配置
- [x] 配置管理系统
- [x] Engine管理器框架
- [x] 监控器框架
- [x] API服务器框架

### 🚧 进行中
- [ ] 实现gRPC服务（tonic集成）
- [ ] 实现HTTP服务（axum集成）
- [ ] 实现进程管理
- [ ] 实现健康检查逻辑
- [ ] 实现指标收集
- [ ] 实现自动重启

### 📋 待实现
- [ ] 沙箱隔离（cgroups/namespace）
- [ ] 资源限制实现
- [ ] 认证和授权
- [ ] 集成测试
- [ ] 性能优化
- [ ] 文档完善

## 构建和运行

```bash
# 构建
cargo build --release

# 运行
cargo run --release

# 使用自定义配置
./target/release/loquat-kernel --config config/kernel.toml
```

## 下一步

1. 实现gRPC服务，基于proto定义
2. 实现HTTP REST API
3. 添加进程管理和监控逻辑
4. 实现沙箱和资源限制
5. 编写集成测试
6. 完善文档

## 相关链接

- [Loquat Engine](../loquat-engine/)
- [Loquat Tool](../loquat-tool/)
- [Proto定义](../proto/)
- [主README](../README.md)
