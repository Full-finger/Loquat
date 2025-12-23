# Loquat - 机器人/代理开发框架

一个基于 Rust 的清洁架构机器人/代理开发框架，采用 AOP（面向切面编程）和日志功能，专为处理即时消息场景而设计。

## 特性

- **清洁架构** - 遵循 SOLID 原则，模块间解耦良好
- **九阶段工作流** - 采用 9 个处理阶段的流水线架构，支持灵活的消息处理
- **AOP 支持** - 提供面向切面编程能力，支持日志、错误跟踪和性能监控
- **多通道支持** - 支持群组、私聊、频道等不同类型的消息通道
- **可扩展性** - 支持第三方 Worker 在特定阶段注册
- **结构化日志** - 提供详细的处理日志和上下文信息
- **并发安全** - 使用 Arc 和 RwLock 确保多线程安全

## 架构概述

### 九阶段工作流系统

项目采用 9 个处理阶段（池）的流水线架构：

- **PreInput** - 预输入池
- **Input** - 输入池（支持第三方注册）
- **InputMiddle** - 输入中间池
- **PreProcess** - 预处理池（支持第三方注册）
- **ProcessMiddle** - 处理中间池
- **Process** - 处理池（支持第三方注册）
- **PostProcess** - 后处理池
- **Output** - 输出池（支持第三方注册）
- **PostOutput** - 后输出池

### 数据结构层次

- **Package** - 流上处理的基本单元，包含 target_sites 和 blocks
- **Block** - 事件块数组
- **Group** - 事件组
- **Event** - 单个事件

### 核心组件

#### Engine（引擎）
- **StandardEngine** - 核心协调器，负责协调所有模块
- 接收输入 Package
- 通过 Router 路由到适配器目标
- 通过 ChannelManager 获取/创建 Channel
- 通过 Stream 处理 Package

#### ChannelManager（通道管理器）
- **StandardChannelManager** - 管理多个通道实例
- 根据 ChannelType（group、private、channel）管理不同的通道
- 自动创建和清理闲置通道

#### Stream（数据流）
- **StandardStream** - 包含 9 个池，按顺序处理包
- **StreamProcessor** - 在序列中的池间处理包

#### Pool（处理池）
- **StandardPool** - 标准池实现，管理 Worker
- 按优先级处理 Worker
- Worker 处理包并决定是否释放到下一池或在当前池继续处理

#### AOP（面向切面编程）
- 支持日志、错误跟踪和性能监控等切面
- 提供 AopManager 和 AopProxy 用于动态应用切面
- 可以在方法执行前后插入横切关注点

#### Logging（日志系统）
- 结构化日志系统，支持多种格式（JSON、文本）
- 支持多种写入器（控制台、文件）
- 提供全局日志器和上下文信息

## 快速开始
