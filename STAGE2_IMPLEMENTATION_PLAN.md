# Loquat 重构 - 阶段 2：Engine 核心实现（7 天）

## 目标
实现完整的 Engine 核心，支持三池子流、事件系统和 Pool 管理。

## 每日任务

### 第 1 天：Engine 类型定义增强
- [ ] 更新 EngineConfig 添加新配置项
- [ ] 更新 EngineStats 添加统计信息
- [ ] 更新 ProcessingContext 添加上下文信息
- [ ] 更新 PoolState 枚举支持更多池类型

### 第 2 天：Engine Trait 增强
- [ ] 定义 EngineEvent 枚举
- [ ] 定义 EventCallback trait
- [ ] 扩展 Engine trait 添加新方法
- [ ] 实现基础事件系统

### 第 3 天：StandardEngine 重构（上）
- [ ] 更新 StandardEngine 内部结构
- [ ] 实现池管理功能
- [ ] 实现事件总线
- [ ] 编译验证

### 第 4 天：Package 处理流程
- [ ] 重构 process() 方法
- [ ] 实现 process_in_pool() 方法
- [ ] 实现循环检测
- [ ] 添加错误处理

### 第 5 天：Pool 集成
- [ ] 修改 Pool trait
- [ ] 实现三池子流
- [ ] 实现 Pool 间传递逻辑
- [ ] 添加 Pool 状态管理

### 第 6 天：事件处理
- [ ] 完善 EventBus 实现
- [ ] 定义完整的事件类型
- [ ] 实现回调机制
- [ ] 添加事件订阅管理

### 第 7 天：测试和优化
- [ ] 单元测试 - Engine 生命周期
- [ ] 单元测试 - Package 处理
- [ ] 集成测试 - 多池流转
- [ ] 性能优化和基准测试

## 关键技术决策

1. **简化事件系统**：使用内存事件总线，暂时不使用 channels
2. **保持向后兼容**：不修改 StandardPool 接口
3. **渐进增强**：先实现三池子流（Input → Process → Output）
4. **所有权传递**：避免 Package 克隆，使用所有权转移

## 预期成果

- ✅ 完整的 Engine 类型定义
- ✅ 增强的 Engine trait
- ✅ 重构的 StandardEngine 实现
- ✅ 可用的 Package 处理流程
- ✅ 集成的 Pool 系统
- ✅ 基础事件处理机制
- ✅ 全面的测试覆盖
