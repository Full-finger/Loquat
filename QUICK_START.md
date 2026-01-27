# Loquat快速开始指南

## 🚀 立即开始（5分钟）

### 第1步：配置protoc（1分钟）

**临时配置**（当前会话有效）：
```powershell
$env:PATH += ";C:\Program Files\MATLAB\R2024b\bin\win64"
protoc --version
```

**永久配置**（推荐）：
1. 右键"此电脑" → 属性
2. 高级系统设置 → 环境变量
3. 在"系统变量"中找到"Path"
4. 点击"编辑"，添加新条目：
   ```
   C:\Program Files\MATLAB\R2024b\bin\win64
   ```
5. 确定并重启终端

### 第2步：重新启用workspace成员（1分钟）

编辑根目录的`Cargo.toml`，取消注释：

```toml
[workspace]
resolver = "2"
members = [
    "loquat-tool",
    "loquat-kernel",  # 取消注释
    "loquat-engine",  # 取消注释
]
```

### 第3步：验证编译（2分钟）

```powershell
# 验证protoc
protoc --version

# 编译proto文件
cd proto
cargo build

# 编译loquat-kernel
cd ../loquat-kernel
cargo build

# 编译loquat-engine
cd ../loquat-engine
cargo build

# 回到根目录
cd ..
```

### 第4步：修复编译错误（持续进行）

如果编译失败，查看错误信息并参考`DEVELOPMENT_ROADMAP.md`中的修复方案。

---

## 📋 现在的状态

### ✅ 可以立即使用的功能

#### loquat-tool（完全可用）
```bash
# 创建新插件
cargo run --bin loquat-tool -- new plugin --name my_plugin --type rust

# 创建新适配器
cargo run --bin loquat-tool -- new adapter --name my_adapter --type rust

# 列出所有插件
cargo run --bin loquat-tool -- list plugin

# 运行框架
cargo run --bin loquat-tool -- run
```

#### src（框架核心）
- 编译通过，功能完整
- 包含REPL、TUI、Web服务等
- 80个警告但不影响使用

### ❌ 需要修复的部分

#### loquat-kernel（52个错误）
- 进程管理、gRPC/HTTP API
- 监控和健康检查
- 需要protoc编译proto文件

#### loquat-engine（待编译）
- 插件系统、事件处理
- 适配器管理
- 需要protoc编译proto文件

---

## 🎯 建议的开发顺序

### 方案A：保守方案（推荐）

**优先级**：
1. 先完善loquat-tool的功能
2. 清理src的警告
3. 最后修复loquat-kernel和engine

**优点**：
- 低风险，快速见效
- 可以立即使用loquat-tool开发插件
- 学习曲线平缓

### 方案B：激进方案

**优先级**：
1. 立即修复所有编译错误
2. 实现所有缺失功能
3. 完善测试和文档

**优点**：
- 一步到位，无技术债
- 适合有经验的开发者

**缺点**：
- 时间投入大
- 可能遇到未知问题

---

## 🛠️ 推荐的工作流程

### 每日工作流

```bash
# 1. 拉取最新代码
git pull origin main

# 2. 配置环境（如果需要）
$env:PATH += ";C:\Program Files\MATLAB\R2024b\bin\win64"

# 3. 编译检查
cargo build

# 4. 运行测试
cargo test

# 5. 代码检查
cargo clippy
```

### 开发新功能

```bash
# 1. 创建新分支
git checkout -b feature/your-feature

# 2. 开发功能
# 编辑代码...

# 3. 测试
cargo test

# 4. 提交
git add .
git commit -m "feat: add your feature"

# 5. 推送
git push origin feature/your-feature
```

---

## 📚 学习资源

### 立即阅读

1. **DEVELOPMENT_ROADMAP.md** - 完整的开发计划
2. **README.md** - 项目概述
3. **docs/tutorials/loquat_tutorial_for_beginners.md** - 初学者教程

### 需要时阅读

- **docs/api/WEB_API_DOCUMENTATION.md** - API文档
- **proto/** - gRPC协议定义
- **examples/** - 示例代码

---

## 🆘 遇到问题？

### 常见问题

#### Q1: protoc命令找不到
**A**: 检查PATH配置，确保包含MATLAB的bin目录

#### Q2: cargo build失败
**A**: 
1. 检查protoc是否正确安装
2. 查看错误信息，参考DEVELOPMENT_ROADMAP.md
3. 清理缓存：`cargo clean && cargo build`

#### Q3: loquat-tool创建的插件无法加载
**A**: 
1. 检查插件路径是否正确
2. 查看插件日志
3. 确认插件实现了正确的trait

#### Q4: 如何调试？
**A**: 
```bash
# 启用详细日志
RUST_LOG=debug cargo run --bin loquat-kernel

# 使用调试器
cargo build && rust-gdb target/debug/loquat-kernel
```

### 获取帮助

- **GitHub Issues**: https://github.com/Full-finger/Loquat/issues
- **GitHub Discussions**: https://github.com/Full-finger/Loquat/discussions
- **查看文档**: docs/目录

---

## ✅ 检查清单

### 开始开发前
- [ ] 已配置protoc
- [ ] 已启用workspace成员
- [ ] 能成功编译loquat-tool
- [ ] 已阅读DEVELOPMENT_ROADMAP.md
- [ ] 已选择开发方案

### 提交代码前
- [ ] 代码通过`cargo clippy`
- [ ] 测试通过`cargo test`
- [ ] 代码格式化`cargo fmt`
- [ ] 更新相关文档
- [ ] Commit message符合规范

---

## 🎉 快速开始示例

### 示例1：创建第一个插件

```bash
# 1. 创建插件
cargo run --bin loquat-tool -- new plugin --name hello_world --type rust

# 2. 进入插件目录
cd plugins/hello_world

# 3. 编辑插件代码
# 编辑 src/lib.rs，实现你的逻辑

# 4. 构建插件
cargo build

# 5. 测试插件
# 使用loquat-tool运行框架
cargo run --bin loquat-tool -- run
```

### 示例2：使用loquat-tool管理项目

```bash
# 检查项目
cargo run --bin loquat-tool -- check

# 列出所有插件
cargo run --bin loquat-tool -- list plugin

# 列出所有适配器
cargo run --bin loquat-tool -- list adapter

# 运行框架
cargo run --bin loquat-tool -- run
```

---

## 📞 下一步

### 如果你是新手
1. 阅读`loquat_tutorial_for_beginners.md`
2. 使用loquat-tool创建示例插件
3. 运行并观察结果
4. 逐步修改和扩展

### 如果你有经验
1. 配置protoc并启用workspace
2. 修复loquat-kernel的编译错误
3. 实现缺失的RPC方法
4. 完善HTTP REST API
5. 添加测试和文档

---

## 📊 项目进度

### 当前进度
- **loquat-tool**: ✅ 100% 完成
- **src**: ✅ 90% 完成（80个警告）
- **loquat-kernel**: ❌ 30% 完成（52个错误）
- **loquat-engine**: ❌ 20% 完成（待编译）
- **proto**: ✅ 100% 完成

### 目标进度（1个月）
- **loquat-tool**: ✅ 100% 完成
- **src**: ✅ 100% 完成（无警告）
- **loquat-kernel**: ✅ 80% 完成
- **loquat-engine**: ✅ 80% 完成
- **测试**: ✅ 60% 覆盖率
- **文档**: ✅ 90% 完成

---

**最后更新**: 2026-01-26  
**版本**: 0.2.0
**状态**: 🟢 loquat-tool可用，🟡 kernel/engine待修复
