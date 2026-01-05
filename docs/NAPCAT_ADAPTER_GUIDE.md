# NapCat Adapter 使用指南

## 概述

NapCat Adapter 是 Loquat 框架的一个适配器，用于与 NapCat QQ 机器人框架进行通信。NapCat 基于 OneBot 标准，通过 WebSocket 协议提供事件和 API 调用。

## 功能特性

- ✅ 通过 WebSocket 与 NapCat 服务器连接
- ✅ 接收 QQ 消息事件
- ✅ 发送消息到 QQ（私聊和群聊）
- ✅ 支持 OneBot 标准消息格式
- ✅ 事件统计和监控
- ✅ 自动重连机制

## 安装

### 1. 添加依赖

依赖已添加到 `Cargo.toml`：

```toml
tokio-tungstenite = "0.20"
futures-util = "0.3"
```

### 2. 注册 Adapter

在 `main.rs` 中注册 NapCat adapter factory：

```rust
use loquat::adapters::{AdapterManager, NapCatAdapterFactory};

fn main() {
    let mut adapter_manager = AdapterManager::new();
    
    // 注册 NapCat adapter
    adapter_manager.register_factory(Box::new(NapCatAdapterFactory))
        .expect("Failed to register NapCat adapter");
    
    // ... 其他初始化代码
}
```

## 配置

### 配置文件

在 `adapters/napcat.json` 中配置 NapCat adapter：

```json
{
  "adapter_type": "napcat",
  "adapter_id": "napcat-001",
  "enabled": true,
  "name": "NapCat Adapter",
  "connection": {
    "conn_type": "ws",
    "url": "ws://127.0.0.1:3001",
    "timeout": 30,
    "use_tls": false,
    "keep_alive": 30,
    "max_reconnect": 5,
    "params": {
      "access_token": ""
    }
  },
  "heartbeat": {
    "interval": 30,
    "timeout": 10,
    "enabled": true
  },
  "retry": {
    "max_attempts": 3,
    "initial_delay": 1000,
    "max_delay": 30000,
    "backoff_multiplier": 2.0
  },
  "platform": {
    "api_base": "http://127.0.0.1:3000",
    "qq_account": ""
  },
  "extra": {}
}
```

### 配置说明

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `adapter_type` | Adapter 类型，必须为 "napcat" | - |
| `adapter_id` | Adapter 唯一标识符 | - |
| `connection.url` | NapCat WebSocket 地址 | ws://127.0.0.1:3001 |
| `connection.timeout` | 连接超时时间（秒） | 30 |
| `connection.use_tls` | 是否使用 TLS | false |
| `connection.max_reconnect` | 最大重连次数 | 5 |
| `platform.api_base` | NapCat HTTP API 地址 | http://127.0.0.1:3000 |
| `platform.qq_account` | QQ 账号（可选） | "" |

## NapCat 服务器设置

### 1. 安装 NapCat

请参考 NapCat 官方文档进行安装和配置：
- [NapCat GitHub](https://github.com/NapNeko/NapCatQQ)

### 2. 配置 WebSocket

在 NapCat 配置中启用 WebSocket：

```yaml
# NapCat 配置示例
ws:
  host: 0.0.0.0
  port: 3001
  enable: true
```

### 3. 配置 HTTP API

```yaml
http:
  host: 0.0.0.0
  port: 3000
  enable: true
  accessToken: ""  # 如果需要鉴权，请配置
```

## 使用示例

### 基本使用

```rust
use loquat::adapters::{AdapterManager, NapCatAdapterFactory};
use loquat::events::EventEnum;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 adapter manager
    let mut adapter_manager = AdapterManager::new();
    
    // 注册 NapCat adapter
    adapter_manager.register_factory(Box::new(NapCatAdapterFactory))?;
    
    // 从配置文件加载 adapter
    adapter_manager.load_from_dir("./adapters").await?;
    
    // 启动所有 adapters
    adapter_manager.start_all().await?;
    
    // 订阅事件
    let event_receiver = adapter_manager.get_event_receiver();
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                EventEnum::Message(msg) => {
                    println!("收到消息: {:?}", msg);
                }
                _ => {
                    println!("收到其他事件: {:?}", event);
                }
            }
        }
    });
    
    // 运行应用
    tokio::signal::ctrl_c().await?;
    
    // 停止所有 adapters
    adapter_manager.stop_all().await?;
    
    Ok(())
}
```

### 发送消息

```rust
use loquat::adapters::NapCatAdapter;

async fn send_example_message(adapter: &NapCatAdapter) -> Result<(), Box<dyn std::error::Error>> {
    // 发送私聊消息
    adapter.send_message("user:123456789", "你好！").await?;
    
    // 发送群聊消息
    adapter.send_message("group:987654321", "大家好！").await?;
    
    Ok(())
}
```

### 处理消息事件

```rust
use loquat::events::{EventEnum, Event, message::MessageEvent};

fn handle_message_event(event: &EventEnum) {
    if let EventEnum::Message(MessageEvent::Text { text, metadata }) = event {
        println!("来自 {} 的消息: {}", 
            metadata.user_id.as_ref().unwrap_or(&"未知".to_string()), 
            text
        );
        
        if let Some(group_id) = &metadata.group_id {
            println!("群聊 ID: {}", group_id);
        }
    }
}
```

## 支持的 OneBot 事件类型

当前版本支持以下事件类型：

### 消息事件 (Message)
- ✅ 私聊消息
- ✅ 群聊消息
- ⚠️ 消息类型（文本、图片、语音等）- 部分支持

### 通知事件 (Notice)
- 📝 计划中

### 请求事件 (Request)
- 📝 计划中

### 元事件 (Meta Event)
- 📝 生命周期事件 - 记录日志

## 消息段 (Message Segment)

支持以下消息段类型：

| 类型 | 说明 | 状态 |
|------|------|------|
| `text` | 文本消息 | ✅ 已支持 |
| `image` | 图片消息 | 📝 解析支持 |
| `at` | @消息 | ✅ 已支持 |
| `face` | 表情 | 📝 解析支持 |
| `record` | 语音 | 📝 解析支持 |
| `video` | 视频 | 📝 解析支持 |
| `reply` | 回复 | 📝 解析支持 |
| `location` | 位置 | 📝 解析支持 |
| `json` | JSON 消息 | 📝 解析支持 |
| `xml` | XML 消息 | 📝 解析支持 |

## API 调用

### 发送消息

```rust
// 私聊
adapter.send_message("user:QQ号", "消息内容").await?;

// 群聊
adapter.send_message("group:群号", "消息内容").await?;
```

### 获取 Adapter 状态

```rust
let adapter = adapter_manager.get_adapter("napcat-001")?;
println!("Adapter 状态: {:?}", adapter.status());
println!("是否运行中: {}", adapter.is_running());
println!("统计信息: {:?}", adapter.statistics());
```

## 故障排查

### 连接失败

**问题**: 无法连接到 NapCat 服务器

**解决方案**:
1. 检查 NapCat 是否正在运行
2. 确认 WebSocket 地址和端口正确
3. 检查防火墙设置
4. 查看 Loquat 日志中的错误信息

```bash
# 检查 NapCat 是否监听端口
netstat -an | findstr 3001
```

### 消息未收到

**问题**: 收不到 QQ 消息

**解决方案**:
1. 检查 Adapter 状态是否为 `Running`
2. 确认 NapCat 的 WebSocket 配置正确
3. 查看日志中是否有解析错误
4. 验证消息格式是否符合 OneBot 标准

### 发送消息失败

**问题**: 无法发送消息

**解决方案**:
1. 确认 WebSocket 连接正常
2. 检查目标用户/群号是否正确
3. 验证 NapCat HTTP API 是否正常工作
4. 查看错误日志获取详细信息

## 日志和调试

启用详细日志：

```rust
use loquat::logging::{init_with_config, LogLevel, LogFormat};

let config = loquat::logging::LoggerConfig {
    level: LogLevel::Debug,
    format: LogFormat::Text,
    // ... 其他配置
};

init_with_config(config)?;
```

查看 NapCat 相关日志：

```
[napcat-001] Connected to NapCat server
[napcat-001] Received message event
[napcat-001] Failed to parse event: ...
```

## 性能优化

### 连接池

多个 NapCat adapter 实例可以共享连接（未来版本）。

### 异步处理

所有 I/O 操作都是异步的，不会阻塞主线程：

```rust
tokio::spawn(async move {
    // 异步发送消息
    adapter.send_message(target, message).await?;
});
```

### 批量处理

对于大量消息，考虑批量处理（未来版本）。

## 安全建议

1. **使用 TLS**: 生产环境建议启用 `use_tls: true`
2. **访问令牌**: 配置 `access_token` 进行身份验证
3. **网络安全**: 限制访问来源 IP
4. **日志脱敏**: 避免在日志中记录敏感信息

## 未来计划

- [ ] 支持更多消息类型（图片、语音、视频等）
- [ ] 实现 HTTP API 调用封装
- [ ] 支持群操作（加群、退群、禁言等）
- [ ] 支持好友管理
- [ ] 实现 OneBot 12 标准
- [ ] 添加单元测试和集成测试
- [ ] 性能优化和连接池

## 参考资源

- [OneBot 11 标准](https://11.onebot.dev/)
- [NapCat GitHub](https://github.com/NapNeko/NapCatQQ)
- [Loquat 框架文档](https://github.com/Full-finger/Loquat)

## 许可证

本适配器遵循 Loquat 框架的许可证。
