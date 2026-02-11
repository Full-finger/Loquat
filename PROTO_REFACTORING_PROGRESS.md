# Proto 定义重构进度报告

## 已完成工作（阶段 0：准备工作）

### 1. 创建新的 package.proto
- **位置**: `proto/package.proto`
- **核心类型**:
  - `Package`: 流水线中的基本处理单元
  - `TargetSite`: Worker 用于匹配 Package 的标记（含 Domain/Motif/State/Context 四种类型）
  - `PayloadMeta`: Payload 元数据
  - `PoolState`: 九池子流水线状态枚举
  - `WorkerInfo`: Worker 信息结构
  - `ProcessResult`: Package 处理结果

### 2. 更新 engine.proto
- **位置**: `proto/engine.proto`
- **主要改动**:
  - 导入 `package.proto` 和 `google/protobuf/timestamp.proto`
  - 使用 `loquat.v1.*` 引用 package 类型
  - 修复 `WorkerInfo` 的引用路径

### 3. 更新 kernel.proto
- **位置**: `proto/kernel.proto`
- **主要改动**:
  - 导入 `package.proto` 和 `google/protobuf/timestamp.proto`
  - 保留现有服务定义，仅修复类型引用

### 4. 更新构建配置
- **build.rs**: 配置了 4 个独立的生成输出目录
  - `src/gen/v1/`: package.proto 生成的代码
  - `src/gen/common/`: common.proto 生成的代码
  - `src/gen/kernel/`: kernel.proto 生成的代码
  - `src/gen/engine/`: engine.proto 生成的代码

- **proto/Cargo.toml**: 添加了 `serde` 依赖（用于序列化支持）

### 5. 更新导出模块
- **proto/src/lib.rs**: 
  - 新增 `v1` 模块导出
  - 重新导出常用类型: `Package`, `TargetSite`, `PayloadMeta`, `PoolState`, `WorkerInfo`, `ProcessResult`

## 验证结果

✅ **编译成功**: proto 包可以正常编译
✅ **代码生成**: 生成了正确的 Rust 代码结构
✅ **类型完整**: 所有关键类型都已正确定义

## 生成的代码结构

```
proto/src/gen/
├── v1/
│   └── loquat.v1.rs          # Package, TargetSite, PoolState, WorkerInfo, ProcessResult
├── common/
│   └── loquat.common.rs       # 通用类型
├── kernel/
│   ├── loquat.common.rs       # Kernel 依赖的通用类型
│   └── loquat.kernel.rs      # Kernel 服务定义
└── engine/
    ├── loquat.common.rs       # Engine 依赖的通用类型
    └── loquat.engine.rs     # Engine 服务定义
```

## 下一步工作（阶段 1：清理 + Proto 定义）

### 目标：实现 Payload 系统

1. **定义具体 Payload 类型**
   - 创建 `loquat/types/` 目录
   - 定义 `Text`, `Image`, `Event` 等具体类型
   - 实现 `PayloadTrait` 统一接口

2. **清理重复模块**
   - 合并 `src/events/` 和 `proto` 定义
   - 统一使用 `loquat_proto::v1::*` 作为唯一数据源
   - 删除冗余的类型定义

3. **更新依赖关系**
   - 修改现有代码使用新的 proto 类型
   - 确保向后兼容性

## 注意事项

1. **Package 池子状态映射**
   ```rust
   // 九池子状态：
   PRE_INPUT (0) → INPUT (1) → POST_INPUT (2) → 
   PRE_PROCESS (3) → MID_PROCESS (4) → PROCESS (5) → 
   POST_PROCESS (6) → OUTPUT (7) → POST_OUTPUT (8)
   ```

2. **TargetSite 四种类型**
   - `Domain`: 物质类型（text, image, event...）
   - `Motif`: 结构特征（command, mention, url...）
   - `State`: 功能状态（intent_weather, spam_suspected...）
   - `Context`: 上下文（user_vip, group_night_mode...）

3. **gRPC 服务位置**
   - **Engine**: 负责处理流水线和 Worker 管理
   - **Kernel**: 负责 Engine 生命周期管理和 Package 路由

## 技术栈版本检查

需要检查的依赖版本：
- ✅ `tonic`: 0.11
- ✅ `prost`: 0.12
- ✅ `tokio`: 1.35
- ✅ `axum`: 0.7

所有版本都是最新的稳定版本，无需升级。
