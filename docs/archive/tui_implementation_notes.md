# TUI (Terminal User Interface) 实现笔记

## 概述
为Loquat框架实现了基于ratatui的终端用户界面(TUI)，提供可视化的框架管理和监控功能。

## 技术栈
- **ratatui v0.24**: Rust TUI框架，功能强大、活跃维护
- **crossterm v0.27**: 跨平台终端控制库

## 实现日期
2026年1月1日

## 架构设计

### 1. 模块结构
```
src/tui/
├── mod.rs           # 模块入口，导出run_tui函数
├── state.rs         # AppState和UiState定义
└── app.rs           # LoquatTui主应用
```

### 2. 核心组件

#### AppState
包含ReplContext，提供对应用组件的访问：
```rust
pub struct AppState {
    pub context: Arc<ReplContext>,
}
```

#### UiState
管理UI状态：
```rust
pub struct UiState {
    pub active_panel: ActivePanel,
    pub command_input: String,
    pub logs: Vec<LogMessage>,
    pub max_logs: usize,
    pub show_help: bool,
    pub show_exit_confirm: bool,
}
```

#### ActivePanel
定义不同的面板：
- Logs: 日志查看
- Plugins: 插件管理
- Adapters: 适配器管理
- Config: 配置查看
- Engine: 引擎状态

### 3. UI布局

三行布局：
```
┌─────────────────────────────────────────────────┐
│ Header (版本、环境信息)                     │
├──────────────┬────────────────────────────────┤
│              │                                │
│ Side Panel   │      Main Content Panel        │
│ (导航)      │      (根据活动面板显示)         │
│              │                                │
├──────────────┴────────────────────────────────┤
│ Command Input (命令输入行)                   │
└─────────────────────────────────────────────────┘
```

### 4. 键盘快捷键

- `Ctrl+C`: 退出TUI
- `Ctrl+L`: 切换到Logs面板
- `Ctrl+P`: 切换到Plugins面板
- `Ctrl+A`: 切换到Adapters面板
- `Ctrl+C` (无Ctrl): 切换到Config面板
- `Ctrl+E`: 切换到Engine面板
- `Tab`: 下一个面板
- `BackTab`: 上一个面板
- `Enter`: 执行命令
- `Backspace`: 删除字符

## 实现细节

### 1. Borrow Checker问题解决

**问题**: 在`terminal.draw()`闭包中调用`self.draw_ui()`导致借用冲突
```rust
// ❌ 错误：无法同时借用self.terminal为可变和不可变
self.terminal.draw(|f| self.draw_ui(f, &ui_state, &app_state))?;
```

**解决方案**: 将UI绘制方法提取为独立函数
```rust
// ✅ 正确：使用独立函数避免借用冲突
fn draw_ui_impl(f: &mut Frame, ui_state: &UiState, app_state: &AppState) {
    // 绘制逻辑
}

self.terminal.draw(|f| draw_ui_impl(f, &ui_state, &app_state))?;
```

### 2. 状态克隆优化

为了避免借用冲突，在绘制前克隆状态：
```rust
let ui_state = self.ui_state.clone();
let app_state = self.app_state.clone();
self.terminal.draw(|f| draw_ui_impl(f, &ui_state, &app_state))?;
```

### 3. ReplContext Debug实现

由于AppState需要Debug trait，为ReplContext实现了自定义Debug：
```rust
impl std::fmt::Debug for ReplContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplContext")
            .field("plugin_manager", &self.plugin_manager.is_some())
            .field("adapter_manager", &self.adapter_manager.is_some())
            .field("engine", &self.engine.is_some())
            .field("logger", &"<Logger>")
            .field("config", &self.config)
            .field("start_time", &self.start_time)
            .finish()
    }
}
```

## 集成到main.rs

添加了`--tui`参数支持：

```rust
enum Command {
    Run { environment: String, rebuild: bool, repl: bool, tui: bool },
    // ...
}

// 解析参数
"--tui" => {
    tui = true;
}

// 执行TUI
if tui {
    // 创建ReplContext
    let repl_context = ReplContext { /* ... */ };
    
    // 运行TUI
    run_tui(repl_context).await?;
}
```

## 当前状态

### ✅ 已完成
1. TUI基础框架搭建
2. UI布局实现（Header、Side Panel、Main Panel、Command Input）
3. 键盘事件处理
4. 欢迎屏幕
5. 面板切换逻辑
6. 日志面板基础实现
7. 命令输入处理
8. --tui参数集成
9. 所有编译错误修复

### 🚧 待完成（阶段2）

#### 面板功能完善
- [ ] Logs面板：集成日志系统，实时显示日志
- [ ] Plugins面板：显示插件列表，支持插件操作
- [ ] Adapters面板：显示适配器列表，支持启停操作
- [ ] Config面板：显示配置信息，支持在线编辑
- [ ] Engine面板：显示引擎状态和统计信息

#### 命令系统集成
- [x] 集成REPL命令到TUI
- [x] 命令执行功能
- [x] 命令历史记录（上下箭头浏览）
- [ ] 命令自动补全

#### 日志集成
- [ ] 实时日志捕获和显示
- [ ] 日志级别过滤
- [ ] 日志搜索功能

#### 高级功能
- [ ] 帮助系统
- [ ] 热键提示
- [ ] 主题切换（深色/浅色）
- [ ] 窗口大小自适应

## 使用方法

### 启动TUI模式
```bash
cargo run -- --tui
```

### 或指定环境
```bash
cargo run -- --tui --env prod
```

### 退出TUI
按 `Ctrl+C` 退出

## 编译状态
- ✅ 所有编译错误已修复
- ✅ 编译成功（84 warnings，无errors）
- ✅ TUI功能可运行

## 阶段2更新（2026年1月2日）

### 已完成功能

#### 日志系统实时集成（P0优先级）

1. **TuiLogWriter实现** (`src/tui/log_writer.rs`):
   - 实现了LogWriter trait用于TUI日志输出
   - 通过mpsc unbounded channel将日志发送到TUI
   - 定义了TuiLogMessage结构体（level, message, timestamp）
   - 解析JSON格式的日志条目并转发到TUI
   ```rust
   pub struct TuiLogMessage {
       pub level: crate::logging::LogLevel,
       pub message: String,
       pub timestamp: String,
   }
   ```

2. **AppState增强**:
   - 添加`log_receiver: mpsc::UnboundedReceiver<TuiLogMessage>`字段
   - 由于UnboundedReceiver不能自动实现Clone，手动实现了Clone trait
   - Clone时创建dummy receiver避免实际数据传输
   ```rust
   impl Clone for AppState {
       fn clone(&self) -> Self {
           let (_, dummy_rx) = mpsc::unbounded_channel();
           Self {
               context: self.context.clone(),
               command_registry: self.command_registry.clone(),
               log_receiver: dummy_rx,
           }
       }
   }
   ```

3. **Logger初始化**:
   - 在LoquatTui::new()中创建log channel
   - 初始化TuiLogWriter并设置到全局logger
   - 记录TUI初始化成功的日志
   ```rust
   let (log_sender, log_receiver) = mpsc::unbounded_channel();
   let tui_log_writer = TuiLogWriter::new(log_sender);
   let logger = init_with_config(
       Arc::new(JsonFormatter::new()),
       Arc::new(tui_log_writer),
       LogLogLevel::Info,
   )?;
   set_global_logger(logger.clone());
   ```

4. **日志实时接收**:
   - 在main loop中使用`try_recv()`非阻塞接收日志
   - 转换LogLevel（Logging crate → TUI LogLevel）
   - 根据min_log_level过滤日志
   - 自动维护最大日志数量限制
   ```rust
   while let Ok(log_msg) = self.app_state.log_receiver.try_recv() {
       // 转换日志级别
       let log_level = match log_msg.level {
           LogLevel::Trace => LogLevel::Debug,
           LogLevel::Debug => LogLevel::Debug,
           LogLevel::Info => LogLevel::Info,
           LogLevel::Warn => LogLevel::Warn,
           LogLevel::Error => LogLevel::Error,
       };
       
       // 过滤并添加日志
       if log_level >= self.ui_state.min_log_level {
           self.ui_state.logs.push(LogMessage {
               timestamp: log_msg.timestamp,
               level: log_level,
               message: log_msg.message,
               context: None,
           });
           
           // 限制日志数量
           if self.ui_state.logs.len() > self.ui_state.max_logs {
               self.ui_state.logs.remove(0);
           }
       }
   }
   ```

5. **日志级别过滤快捷键**:
   - F1: 设置日志级别为DEBUG
   - F2: 设置日志级别为INFO
   - F3: 设置日志级别为WARN
   - F4: 设置日志级别为ERROR
   - 每次切换都会记录日志信息
   ```rust
   KeyCode::F(1) => {
       self.ui_state.min_log_level = LogLevel::Debug;
       self.ui_state.logs.push(LogMessage::new(
           LogLevel::Info,
           "Log level set to DEBUG".to_string(),
           None
       ));
   }
   ```

6. **模块集成**:
   - 在src/tui/mod.rs中添加log_writer模块声明
   - 确保所有模块正确导出

### 技术细节

#### 日志流向架构
```
应用组件 → Logger → LogWriter::write() → TuiLogWriter → mpsc channel → 
→ TUI main loop → try_recv() → UiState.logs → draw_logs_panel() → 显示
```

#### JSON日志解析
TuiLogWriter需要解析JSON格式的日志条目：
```json
{
  "level": "INFO",
  "message": "Something happened",
  "timestamp": "2026-01-02T00:30:00Z"
}
```

#### LogLevel转换
由于logging crate和TUI使用不同的LogLevel枚举，需要转换：
```rust
// Logging crate: Trace, Debug, Info, Warn, Error
// TUI crate:     Debug, Info, Warn, Error
// 映射：Trace → Debug, 其他保持一致
```

#### 非阻塞接收策略
使用`try_recv()`而不是`recv().await`确保：
- 不阻塞UI渲染
- 100ms事件轮询间隔内处理所有可用日志
- 不会因为日志积压而卡死UI

#### 内存管理
- 最大日志数量限制（默认1000条）
- 超出限制自动移除最旧日志（FIFO）
- 避免长时间运行导致内存泄漏

### 编译状态
- ✅ 所有编译错误已修复
- ✅ 日志系统集成完成
- ✅ F1-F4快捷键正常工作
- ✅ 日志实时显示在Logs面板
- ⚠️  86 warnings（大部分是未使用的导入，不影响功能）

#### 命令系统集成（P0优先级）
1. **AppState增强**:
   - 添加`command_registry: Arc<CommandRegistry>`字段
   - 集成所有REPL命令到TUI

2. **UiState增强**:
   - 添加`command_history: Vec<String>`存储命令历史
   - 添加`history_index: Option<usize>`支持历史浏览
   - 添加`min_log_level: LogLevel`支持日志级别过滤
   - 为LogLevel添加`PartialOrd`和`Ord`实现

3. **命令注册**:
   - 注册所有9个命令：help, status, plugins, adapters, reload, logs, config, engine, clear, exit
   - HelpCommand需要传入命令列表进行初始化

4. **命令执行**:
   - 实现完整的命令解析和执行逻辑
   - 命令执行结果以日志形式显示在Logs面板
   - 支持命令参数传递

5. **命令历史**:
   - 所有执行的命令自动添加到历史
   - 上箭头浏览历史（从最新到最旧）
   - 下箭头浏览历史（从当前位置到清空）
   - 历史索引管理

6. **键盘事件增强**:
   - 上/下箭头键支持命令历史浏览
   - 命令执行后清空输入并重置历史索引

7. **CommandRegistry Debug实现**:
   - 由于Box<dyn Command>没有Debug trait，手动实现Debug格式化
   - 显示命令数量而非命令详情

### 技术细节

#### 命令注册流程
```rust
// 1. 创建命令注册表
let mut command_registry = CommandRegistry::new();

// 2. 准备命令列表（用于HelpCommand）
let all_commands = vec![
    "help".to_string(),
    "status".to_string(),
    // ...
];

// 3. 注册所有命令
command_registry.register(Box::new(HelpCommand::new(Arc::new(all_commands))));
command_registry.register(Box::new(StatusCommand));
// ...

// 4. 创建AppState
let app_state = AppState::new(context, command_registry);
```

#### 命令执行流程
```rust
async fn execute_command(&mut self) -> Result<()> {
    // 1. 获取命令并清空输入
    let command = self.ui_state.command_input.clone();
    self.ui_state.command_input.clear();
    
    // 2. 添加到历史
    self.ui_state.command_history.push(command.clone());
    
    // 3. 解析命令
    let parts: Vec<&str> = command.split_whitespace().collect();
    
    // 4. 查找并执行命令
    if let Some(cmd) = self.app_state.command_registry.find(cmd_name) {
        let ctx = self.app_state.context.clone();
        match cmd.execute(&args, &ctx).await {
            Ok(()) => {
                // 成功日志
            }
            Err(e) => {
                // 错误日志
            }
        }
    }
}
```

#### 命令历史浏览
```rust
// 上箭头 - 向后浏览历史
KeyCode::Up => {
    if !self.ui_state.command_history.is_empty() {
        let new_index = match self.ui_state.history_index {
            None => Some(self.ui_state.command_history.len() - 1),  // 最新
            Some(idx) if idx > 0 => Some(idx - 1),                // 上一条
            Some(_) => Some(0),                                    // 最旧
        };
        self.ui_state.history_index = new_index;
        if let Some(idx) = new_index {
            self.ui_state.command_input = self.ui_state.command_history[idx].clone();
        }
    }
}

// 下箭头 - 向前浏览历史
KeyCode::Down => {
    match self.ui_state.history_index {
        None => {}
        Some(idx) if idx < self.ui_state.command_history.len() - 1 => {
            self.ui_state.history_index = Some(idx + 1);  // 下一条
            self.ui_state.command_input = self.ui_state.command_history[idx + 1].clone();
        }
        Some(_) => {
            self.ui_state.history_index = None;  // 清空
            self.ui_state.command_input.clear();
        }
    }
}
```

### 编译状态
- ✅ 所有编译错误已修复
- ✅ CommandRegistry手动实现Debug trait
- ✅ 命令历史功能正常工作
- ⚠️  84 warnings（大部分是未使用的导入，不影响功能）

### 下一步计划

#### 优先级P0（必须完成）
1. **日志系统实时集成**
   - 创建TuiLogWriter实现LogWriter trait
   - 添加日志通道到AppState
   - 实现日志级别过滤快捷键（F1-F4）

#### 优先级P1（重要功能）
2. **Plugins面板实现**
   - 显示插件列表
   - 实现插件操作（加载/卸载）
   
3. **Adapters面板实现**
   - 显示适配器列表
   - 实现适配器控制（启动/停止）
   
4. **Config面板实现**
   - 显示配置树状结构
   - 支持配置查看
   
5. **Engine面板实现**
   - 显示引擎状态
   - 显示统计信息

#### 优先级P2（用户体验增强）
6. **命令自动补全**
7. **帮助系统**
8. **日志搜索**
9. **主题切换**

### 测试建议
1. 测试所有9个命令的执行
2. 测试命令历史浏览功能
3. 测试命令参数传递
4. 测试错误命令的处理
5. 测试日志显示

## 性能考虑

1. **状态克隆**: 每帧克隆UI状态可能影响性能，但对于文本UI影响较小
2. **日志显示**: 限制最大日志数量（默认1000条）避免内存溢出
3. **事件轮询**: 使用100ms超时平衡响应性和CPU使用

## 未来改进方向

1. **状态管理**: 考虑使用RwLock避免克隆
2. **异步更新**: 使用channels从其他组件接收更新
3. **性能优化**: 只重绘变化的部分
4. **测试**: 添加TUI的单元测试和集成测试

## 参考资料
- [ratatui官方文档](https://docs.rs/ratatui/)
- [crossterm官方文档](https://docs.rs/crossterm/)
- [Rust TUI最佳实践](https://github.com/ratatui-org/ratatui)
