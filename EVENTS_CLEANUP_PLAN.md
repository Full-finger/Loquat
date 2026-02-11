# Events 模块清理计划

## 问题分析

### 当前状态
- `src/events/payloads.rs` 包含两套 Payload 定义：
  1. 旧的 MessagePayload, NoticePayload, RequestPayload（用于事件元数据）
  2. 新的 TextPayload, BlobPayload, EventPayload（与 src/payloads/ 重复）

### 目标
统一使用 `src/payloads/` 中的新 Payload 系统，清理重复代码。

## 清理步骤

### 第 1 步：重命名旧的 payloads.rs
```bash
src/events/payloads.rs → src/events/payloads_legacy.rs
```

### 第 2 步：创建新的 events/payloads.rs
使用新的 Payload 系统，提供：
- 便捷的从旧系统到新系统的转换函数
- 向后兼容的包装器

### 第 3 步：更新 events/mod.rs
- 保留旧系统的导出（标记为 deprecated）
- 导出新的 Payload 系统

### 第 4 步：更新依赖
- 更新使用旧 Payload 的代码
- 提供迁移指南

## 迁移策略

### 向后兼容
```rust
// 旧代码仍然可用，但会有警告
use loquat::events::MessagePayload; // deprecated

// 新代码推荐方式
use loquat::payloads::TextPayload;
```

### 转换函数
提供便捷函数帮助迁移：
```rust
// 从旧到新
impl From<MessagePayload> for TextPayload { ... }

// 从新到旧（如果需要）
impl From<TextPayload> for MessagePayload { ... }
```

## 依赖关系分析

需要检查以下模块是否使用旧的 Payload 系统：
- src/adapters/
- src/workers/
- src/engine/
- src/pools/

## 测试计划
1. 单元测试验证转换函数
2. 集成测试验证兼容性
3. 性能测试确保无退化
