# 阶段 1：清理 + Proto 定义 - 执行计划

## 现状分析

### 重复的类型定义

1. **Package 定义冲突**
   - `src/events/package.rs`: 使用 `Block` 结构
   - `proto/package.proto`: 使用 `TargetSite` 结构
   - 两者设计理念不同，需要统一

2. **Payload 系统**
   - 旧系统：MessagePayload, NoticePayload, RequestPayload（事件元数据）
   - 新系统 v2.0：TextPayload, BlobPayload, EventPayload（Package 内容）
   - 使用 `BoxedPayload` (trait object)
   - 新 proto 使用 `google.protobuf.Any`

3. **缺失的类型**
   - `PoolState`: 新 proto 定义了九池子状态
   - `WorkerInfo`: 新 proto 定义了 Worker 信息
   - `ProcessResult`: 新 proto 定义了处理结果

## 清理策略

### 策略 1：渐进式迁移（推荐）

**原则**: 保留现有 API，内部逐步迁移

**步骤 1：创建适配层**
```
src/payloads/
├── mod.rs              # 模块入口
├── adapter.rs          # 旧 Payload → proto.Any 适配
├── types.rs           # 具体类型定义
└── registry.rs        # 类型注册表
```

**步骤 2：更新 Package 定义**
```rust
// src/events/package.rs - 迁移到 proto
pub use loquat_proto::v1::Package;
pub use loquat_proto::v1::TargetSite;
pub use loquat_proto::v1::PoolState;

// 保留便捷方法作为适配器
impl PackageExt for loquat_proto::v1::Package {
    // 添加便捷方法
}
```

**步骤 3：Payload 系统统一**
```rust
// src/payloads/adapter.rs
pub trait PayloadAdapter {
    fn to_any(&self) -> Result<prost_types::Any>;
    fn from_any(any: &prost_types::Any) -> Result<Self>;
}

// 为旧类型实现适配
impl PayloadAdapter for TextPayload { ... }
impl PayloadAdapter for BlobPayload { ... }
```

### 策略 2：直接迁移（激进）

直接删除旧代码，强制使用新 proto 定义。优点：干净彻底；缺点：破坏所有现有依赖。

## 推荐的执行顺序

### 第 1 天：基础结构搭建

1. **创建 payloads 模块**
   - `src/payloads/mod.rs`: 模块入口
   - `src/payloads/types.rs`: 具体类型（Text, Image, Event）
   - `src/payloads/adapter.rs`: Proto Any 适配器
   - `src/payloads/registry.rs`: 类型注册表

2. **定义 PayloadTrait**
   ```rust
   pub trait UniversalPayload: Send + Sync + 'static {
       fn type_url(&self) -> String;
       fn to_any(&self) -> Result<prost_types::Any>;
       fn from_any(any: &prost_types::Any) -> Result<Self>;
   }
   ```

3. **更新 events/mod.rs**
   - 导出新的 proto 类型
   - 标记旧类型为 deprecated

### 第 2 天：清理 events 模块

1. **废弃 package.rs**
   - 将便捷方法移到 `loquat_proto::v1::Package` 的扩展 trait
   - 更新所有 `use crate::events::Package` 为 `loquat_proto::v1::Package`

2. **整合 payloads.rs**
   - 将 v2.0 Universal Payload 系统移到 `src/payloads/`
   - 保留 Legacy Payload 用于向后兼容（标记 deprecated）

3. **更新依赖关系**
   - 搜索所有使用旧类型的地方
   - 逐步替换为 proto 类型

### 第 3 天：验证和测试

1. **单元测试**
   - 测试 Payload 序列化/反序列化
   - 测试 Package 在不同池子间流转
   - 测试 TargetSite 匹配

2. **集成测试**
   - 运行现有的 358 个测试
   - 修复失败的测试
   - 确保没有编译错误

3. **文档更新**
   - 更新 README.md
   - 添加迁移指南
   - 标记废弃的 API

## 风险和缓解

### 风险 1：破坏性变更
- **风险**: 现有代码可能无法编译
- **缓解**: 使用 re-export 保持向后兼容
- **时间估计**: 2-3 天

### 风险 2：测试失败
- **风险**: 2 个现有测试失败
- **缓解**: 优先修复测试，确保 CI 通过
- **时间估计**: 1 天

### 风险 3：性能影响
- **风险**: Proto Any 序列化可能更慢
- **缓解**: Benchmark 对比，优化关键路径
- **时间估计**: 持续进行

## 完成标准

### 阶段 1 完成标志

- [x] 所有类型定义统一使用 proto
- [ ] 编译无警告
- [ ] 所有测试通过（358/358）
- [ ] 文档更新完成
- [ ] 向后兼容性保持

### 阶段 1 成功后

代码将进入"半干净"状态：
- 类型定义统一（来自 proto）
- Payload 系统现代化
- 为阶段 2（Engine 重构）做好准备
