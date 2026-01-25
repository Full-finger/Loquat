# Loquat Web API 文档

本文档详细说明了Loquat框架的Web API端点。

## 基础信息

- **基础URL**: `http://localhost:8080`
- **内容类型**: `application/json`
- **认证**: 目前未实现（待添加）

## API响应格式

所有API响应遵循统一格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

失败响应：

```json
{
  "success": false,
  "data": null,
  "error": "错误信息",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

---

## 基础端点

### 欢迎页面

```http
GET /
```

返回欢迎信息和可用端点列表。

### 健康检查

```http
GET /health
```

返回系统健康状态和各子系统状态。

---

## 配置管理API（12个端点）

### 通用配置

**获取通用配置**
```http
GET /api/config/general
```

**更新通用配置**
```http
PATCH /api/config/general
Content-Type: application/json

{
  "environment": "development",
  "name": "loquat"
}
```

### 日志配置

**获取日志配置**
```http
GET /api/config/logging
```

**更新日志配置**
```http
PATCH /api/config/logging
Content-Type: application/json

{
  "level": "info",
  "format": "json",
  "output": "file",
  "file_path": "./logs/loquat.log",
  "enable_colors": true
}
```

### 插件配置

**获取插件配置**
```http
GET /api/config/plugins
```

**更新插件配置**
```http
PATCH /api/config/plugins
Content-Type: application/json

{
  "enabled": true,
  "plugin_dir": "./plugins",
  "auto_load": true,
  "enable_hot_reload": true,
  "hot_reload_interval": 5,
  "whitelist": [],
  "blacklist": []
}
```

### 适配器配置

**获取适配器配置**
```http
GET /api/config/adapters
```

**更新适配器配置**
```http
PATCH /api/config/adapters
Content-Type: application/json

{
  "enabled": true,
  "adapter_dir": "./adapters",
  "auto_load": true,
  "enable_hot_reload": true,
  "hot_reload_interval": 5,
  "whitelist": [],
  "blacklist": []
}
```

### 引擎配置

**获取引擎配置**
```http
GET /api/config/engine
```

**更新引擎配置**
```http
PATCH /api/config/engine
Content-Type: application/json

{
  "auto_route": true,
  "auto_create_channels": true,
  "auto_initialize": true
}
```

### Web配置

**获取Web配置**
```http
GET /api/config/web
```

**更新Web配置**
```http
PATCH /api/config/web
Content-Type: application/json

{
  "enabled": true,
  "host": "127.0.0.1",
  "port": 8080,
  "enable_cors": true
}
```

### 配置验证

**验证配置**
```http
POST /api/config/validate
```

验证当前配置的有效性，不保存更改。

**重新加载配置**
```http
POST /api/config/reload
```

从磁盘重新加载配置文件。

**保存配置**
```http
POST /api/config/save
```

将当前配置保存到磁盘。

### 配置备份

**获取备份列表**
```http
GET /api/config/backups
```

**创建备份**
```http
POST /api/config/backup
```

**恢复备份**
```http
POST /api/config/restore/:backup_id
```

从指定备份恢复配置。

---

## 引擎控制API（7个端点）

### 引擎状态

**获取引擎状态**
```http
GET /api/engine/status
```

返回引擎当前状态、运行状态和最后错误信息。

**启动引擎**
```http
POST /api/engine/start
```

**停止引擎**
```http
POST /api/engine/stop
```

### 引擎统计

**获取引擎统计**
```http
GET /api/engine/stats
```

返回处理的包数量、成功/失败统计、平均处理时间和运行时间。

**重置统计**
```http
POST /api/engine/reset-stats
```

### 引擎配置和健康

**更新引擎配置**
```http
PATCH /api/engine/config
Content-Type: application/json

{
  "auto_route": true,
  "auto_create_channels": true,
  "auto_initialize": true
}
```

**引擎健康检查**
```http
GET /api/engine/health
```

返回引擎健康状态和最后检查时间。

---

## 系统管理API（6个端点）

### 系统状态

**获取系统状态**
```http
GET /api/system/status
```

返回系统运行状态、运行时间、内存/CPU使用情况和活跃连接数。

**获取系统信息**
```http
GET /api/system/info
```

返回系统名称、操作系统版本、架构、Rust版本、Loquat版本和主机名。

**系统诊断**
```http
GET /api/system/diagnostics
```

返回整体健康状况和各子系统状态的详细诊断信息，包括问题和警告列表。

### 系统控制

**关闭系统**
```http
POST /api/system/shutdown
```

**重启系统**
```http
POST /api/system/restart
```

**清除缓存**
```http
POST /api/system/clear-cache
```

清除所有系统缓存（LRU缓存、热重载历史等）。

---

## 插件管理扩展API（8个端点）

### 插件启用/禁用

**禁用插件**
```http
POST /api/plugins/:name/disable
```

将插件添加到黑名单。

**启用插件**
```http
POST /api/plugins/:name/enable
```

从黑名单移除插件。

### 插件白名单管理

**添加到白名单**
```http
POST /api/plugins/:name/whitelist
```

**从白名单移除**
```http
POST /api/plugins/:name/unwhitelist
```

### 插件黑名单管理

**添加到黑名单**
```http
POST /api/plugins/:name/blacklist
```

**从黑名单移除**
```http
POST /api/plugins/:name/unblacklist
```

### 插件配置

**获取插件配置**
```http
GET /api/plugins/:name/config
```

**更新插件配置**
```http
PATCH /api/plugins/:name/config
Content-Type: application/json

{
  "enabled": true,
  "config": { ... }
}
```

---

## 适配器管理扩展API（10个端点）

### 适配器启用/禁用

**禁用适配器**
```http
POST /api/adapters/:name/disable
```

**启用适配器**
```http
POST /api/adapters/:name/enable
```

### 适配器白名单管理

**添加到白名单**
```http
POST /api/adapters/:name/whitelist
```

**从白名单移除**
```http
POST /api/adapters/:name/unwhitelist
```

### 适配器黑名单管理

**添加到黑名单**
```http
POST /api/adapters/:name/blacklist
```

**从黑名单移除**
```http
POST /api/adapters/:name/unblacklist
```

### 适配器配置

**获取适配器配置**
```http
GET /api/adapters/:name/config
```

**更新适配器配置**
```http
PATCH /api/adapters/:name/config
Content-Type: application/json

{
  "enabled": true,
  "config": { ... }
}
```

### 适配器控制

**启动适配器**
```http
POST /api/adapters/:name/start
```

**停止适配器**
```http
POST /api/adapters/:name/stop
```

---

## 传统端点（向后兼容）

以下端点保留用于向后兼容：

```http
GET /api/plugins                      # 列出所有插件
GET /api/plugins/:name               # 获取插件详情
POST /api/plugins/reload              # 重新加载插件
GET /api/adapters                    # 列出所有适配器
GET /api/adapters/:name              # 获取适配器详情
POST /api/adapters/reload            # 重新加载适配器
POST /api/reload                    # 重新加载所有
GET /api/config                     # 获取配置
```

---

## 错误代码

| HTTP状态码 | 说明 |
|-----------|------|
| 200 | 请求成功 |
| 201 | 资源创建成功 |
| 400 | 请求参数错误 |
| 404 | 资源未找到 |
| 500 | 服务器内部错误 |

---

## 使用示例

### 获取系统健康状态

```bash
curl http://localhost:8080/health
```

### 列出所有插件

```bash
curl http://localhost:8080/api/plugins
```

### 更新日志配置

```bash
curl -X PATCH http://localhost:8080/api/config/logging \
  -H "Content-Type: application/json" \
  -d '{"level": "debug", "format": "json"}'
```

### 禁用特定插件

```bash
curl -X POST http://localhost:8080/api/plugins/my-plugin/disable
```

### 获取引擎统计

```bash
curl http://localhost:8080/api/engine/stats
```

---

## 注意事项

1. **配置更新**: 大部分配置更新需要重启相关服务才能生效
2. **热重载**: 某些配置支持热重载，无需重启
3. **权限**: 部分操作（如系统关机）可能需要管理员权限
4. **错误处理**: 所有错误都会在响应的`error`字段中返回详细信息
5. **时间戳**: 所有响应都包含ISO 8601格式的时间戳

---

## 未来扩展

计划添加的API端点：

- 用户认证和授权
- WebSocket支持
- 文件上传/下载
- 事件订阅
- 批量操作
- API版本控制
- 速率限制
- 请求日志查看
