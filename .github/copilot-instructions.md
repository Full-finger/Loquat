# Loquat Framework - AI Coding Guidelines

## Project Overview
Loquat is a Rust-based framework for building robot/agent services with clean architecture principles. It features a 9-stage pipeline architecture, AOP (Aspect-Oriented Programming), comprehensive logging, and hot-reload capabilities for plugins and adapters.

**Key Characteristics:**
- **Rust Edition 2024**: Modern Rust with latest features
- **Windows-First Development**: Primary development on Windows with batch scripts for automation
- **Chinese Documentation**: Chinese README and configuration examples
- **Multi-Environment Support**: dev/test/prod configurations with easy switching

## Architecture Overview

### Core Flow
```
Engine → Router → ChannelManager → Stream (9 Pools) → Workers
```

### 9-Stage Pipeline Architecture
The Stream contains 9 processing pools that process packages sequentially:

1. **PreInput** - Pre-input pool (internal)
2. **Input** - Input pool (supports third-party Worker registration)
3. **InputMiddle** - Input middle pool (internal)
4. **PreProcess** - Pre-processing pool (supports third-party Worker registration)
5. **ProcessMiddle** - Processing middle pool (internal)
6. **Process** - Processing pool (supports third-party Worker registration)
7. **PostProcess** - Post-processing pool (internal)
8. **Output** - Output pool (supports third-party Worker registration)
9. **PostOutput** - Post-output pool (internal)

**Extensible Pools**: Only Input, PreProcess, Process, and Output pools allow third-party Worker registration.

### Data Hierarchy
- **Package** - Top-level unit, contains target_sites and blocks
- **Block** - Array of events with metadata
- **Group** - Event groups within blocks
- **Event** - Individual message/notice/request (EventEnum: Message, Notice, Request, Meta)

### Channel Types
Channels are extracted from package IDs:
- `group:123` → `ChannelType::group("123")`
- `private:123` → `ChannelType::private("123")`
- `channel:123` → `ChannelType::channel("123")`

## Configuration System

### Configuration Files
Located in `config/` directory:
- `default.toml` - Base configuration (shared across all environments)
- `dev.toml` - Development environment
- `test.toml` - Testing environment
- `prod.toml` - Production environment

Configuration merging: `default.toml` + `{environment}.toml`

### Key Configuration Sections

```toml
[general]
environment = "dev"  # dev/test/prod
name = "Loquat Framework"

[logging]
level = "Debug"      # Trace/Debug/Info/Warn/Error
format = "text"      # text/json
output = "console"   # console/file/combined

[plugins]
enabled = true
auto_load = true
enable_hot_reload = true
hot_reload_interval = 5  # seconds

[adapters]
enabled = true
auto_load = true
enable_hot_reload = true
hot_reload_interval = 10
```

## Worker System

### Worker Traits
All workers must implement:
```rust
#[async_trait]
pub trait Worker: Debug + Send + Sync {
    fn name(&self) -> &str;
    fn worker_type(&self) -> WorkerType;
    fn matches(&self, target_site: &TargetSite) -> bool;
    async fn handle_batch(&self, packages: Vec<Package>) -> WorkerResult;
    fn is_output_safe(&self, output: &Package) -> bool;
}
```

### Worker Registration
Workers register in specific pools with priority:
```rust
let registration = WorkerRegistration::new(
    Box::new(MyWorker::new()),
    MatchingRule::All,  // or Group/Worker/Regex
    0,  // priority (lower = earlier execution)
);
pool.register(registration)?;
```

### MatchingRule Types
- `All` - Matches all target sites
- `Worker(String)` - Matches specific worker name
- `Group(String)` - Matches specific group
- `Regex(String)` - Matches by regex pattern

### WorkerResult Types
- `WorkerResult::Release` - Package moves to next pool
- `WorkerResult::Modify(Vec<Package>)` - Modified packages continue in current pool

**Dead Loop Prevention**: Workers must implement `is_output_safe()` to prevent infinite loops. If a worker returns the same package, it's considered unsafe and logged as a dead loop warning.

### Pool Processing Logic
Pools process packages through workers in priority order:
1. Check if worker matches any target_site in package
2. If matched, execute worker's `handle_batch()`
3. Based on `WorkerResult`:
   - Release → Package moves to next pool
   - Modify → Packages continue in current pool for next iteration
4. If no worker matches, package moves to next pool

## Plugin System

### Plugin Manager
```rust
pub struct PluginManager {
    registry: Arc<PluginRegistry>,
    loader: Arc<CompositePluginLoader>,
    config: PluginConfig,
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
}
```

### Plugin Lifecycle
1. **Discovery**: Scan plugin directory for `.dll`, `.so`, `.dylib`, `.py`, `.js`, `.mjs`, `.ts` files
2. **Loading**: Load plugin via CompositePluginLoader, check whitelist/blacklist
3. **Registration**: Register metadata in PluginRegistry
4. **Hot Reload**: Monitor file modifications and reload automatically

### Hot Reload
HotReloadManager monitors plugins for changes:
```rust
HotReloadManager::new(manager, Duration::from_secs(5)).start().await?;
```

### Plugin Traits
```rust
#[async_trait]
pub trait Plugin: Debug + Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
    async fn initialize(&mut self) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn reload(&mut self) -> Result<()>;
}
```

## Adapter System

Similar to plugins, adapters:
- Support hot-reload with configurable intervals
- Convert external events to `Package` via `Adapter` trait
- Managed by AdapterManager (similar to PluginManager)
- Auto-load from `adapters/` directory

## AOP (Aspect-Oriented Programming)

### Aspect Trait
```rust
#[async_trait]
pub trait Aspect: Debug + Send + Sync {
    fn name(&self) -> &str;
    async fn before(&self, context: &mut AspectContext) -> Result<()>;
    async fn after(&self, context: &mut AspectContext, result: &Result<()>) -> Result<()>;
}
```

### Built-in Aspects
- **LoggingAspect** - Log method execution with context
- **PerformanceAspect** - Track execution time
- **ErrorTrackingAspect** - Capture and log errors

### AOP Proxy
```rust
AopProxy::new(target, vec![aspect1, aspect2]).execute_with_aspects().await
```

## Logging System

### Structured Logging
```rust
pub trait Logger: Debug + Send + Sync {
    fn log(&self, level: LogLevel, message: &str, context: &LogContext);
}

pub struct LogContext {
    pub component: Option<String>,
    pub fields: HashMap<String, String>,
}
```

### Formatters
- `JsonFormatter` - JSON output
- `TextFormatter` - Human-readable text

### Writers
- `ConsoleWriter` - stdout/stderr
- `FileWriter` - File output with rotation

### Logging Best Practices
```rust
let mut log_context = LogContext::new();
log_context.component = Some("Engine".to_string());
log_context.add("package_id", package.package_id.to_string());
log_context.add("event_type", "process_success");
logger.log(LogLevel::Info, &message, &log_context);
```

## Engine Architecture

### StandardEngine
The core coordinator with:
- **Config**: EngineConfig with auto_route, auto_create_channels flags
- **Stats**: EngineStats tracking package counts and timing
- **State**: EngineStatus (Idle/Processing/Stopped) with AtomicU8 for sync access
- **Router**: Routes packages to adapter targets
- **ChannelManager**: Manages channel instances
- **Logger**: Structured logging

### Processing Pipeline
```rust
async fn process(&mut self, package: Package) -> Result<Package> {
    // 1. Get processing context (route + channel type)
    let context = self.get_processing_context(&package).await?;
    
    // 2. Process through stream (9 pools)
    let result = self.process_pipeline(&package, &context).await?;
    
    // 3. Update stats and reset state
    // ...
    
    Ok(result)
}
```

### Status Management
Uses AtomicU8 for thread-safe status checks:
- `0` = Idle
- `1` = Processing
- `2` = Stopped

This allows `is_running()` to be synchronous while other methods are async.

## Development Workflow

### Windows Quick Start
```batch
# Start with dev environment (default)
start.bat

# Start with specific environment
start.bat prod
start.bat test

# Rebuild then start
start.bat --rebuild

# Combined options
start.bat test --rebuild
```

### Development Tools
```batch
# Rebuild project
dev-tools\rebuild.bat

# Clean build artifacts
dev-tools\clean.bat

# Clean everything (logs, temp files)
dev-tools\clean.bat --all

# Run checks (check, clippy, test)
dev-tools\check.bat
```

### Building
```bash
cargo build
cargo build --release
cargo check
```

### Testing
```bash
cargo test
RUST_LOG=debug cargo test  # Enable logging
```

### Running Examples
```bash
cargo run --example basic_usage
```

## Code Style Conventions

### Naming
- **Types**: PascalCase (e.g., `StandardEngine`, `WorkerRegistration`)
- **Functions/Methods**: snake_case (e.g., `get_processing_context`, `handle_batch`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `STATUS_IDLE`, `STATUS_PROCESSING`)
- **Private fields**: snake_case with leading underscore for pattern matching (e.g., `_config`)

### Module Organization
- Flat structure with `mod.rs` for exports
- `traits.rs` for behavior definitions
- `types.rs` for data structures
- `standard_*.rs` for implementations (e.g., `standard_pool.rs`)

### Concurrency Patterns
- **Shared State**: `Arc<RwLock<T>>` for mutable state
- **Status Flags**: `AtomicU8` for simple status checks
- **Async Coordination**: Use `tokio::sync` primitives

### Error Handling
```rust
pub type Result<T> = std::result::Result<T, LoquatError>;

pub enum LoquatError {
    Config(ConfigError),
    Plugin(PluginError),
    Adapter(AdapterError),
    Io(std::io::Error),
    Unknown(String),
}

// Use ? for propagation
async fn process(&self) -> Result<()> {
    let result = self.do_something().await?;
    Ok(())
}
```

### Async Patterns
```rust
#[async_trait]
impl Engine for StandardEngine {
    async fn process(&mut self, package: Package) -> Result<Package> {
        // Implementation
    }
}
```

## Dependencies

### Core Dependencies
```toml
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
```

### Logging
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

### Web/HTTP
```toml
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace"] }
```

### Utilities
```toml
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
toml = "0.8"
regex = "1.10"
atty = "0.2"  # Terminal detection
tempfile = "3.8"
```

## Integration Points

### Web Services
Use `axum` for HTTP endpoints:
```rust
let app = Router::new()
    .route("/health", get(health_check))
    .route("/packages", post(handle_package));
```

### External Logging
Bridge to `tracing` ecosystem:
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

### Third-party Workers
Register workers in extensible pools:
```rust
let worker = Box::new(MyWorker::new());
let registration = WorkerRegistration::new(
    worker,
    MatchingRule::All,
    10,  // priority
);
stream.register_worker_in_pool(PoolType::Input, registration)?;
```

## Common Implementation Examples

### Creating a Custom Worker
```rust
use loquat::prelude::*;
use async_trait::async_trait;

pub struct MyWorker {
    name: String,
}

impl MyWorker {
    pub fn new() -> Self {
        Self {
            name: "my_worker".to_string(),
        }
    }
}

#[async_trait]
impl Worker for MyWorker {
    fn name(&self) -> &str {
        &self.name
    }

    fn worker_type(&self) -> WorkerType {
        WorkerType::Input
    }

    fn matches(&self, target_site: &TargetSite) -> bool {
        // Match specific criteria
        target_site.group_name == Some("special_group".to_string())
    }

    async fn handle_batch(&self, packages: Vec<Package>) -> WorkerResult {
        // Process packages
        for mut package in packages {
            // Modify package
            package.add_target_site(TargetSite::new("processed"));
            // Return modified packages
            return WorkerResult::modify(vec![package]);
        }
        WorkerResult::release()
    }

    fn is_output_safe(&self, output: &Package) -> bool {
        // Prevent dead loops by checking if output differs from input
        !output.target_sites.is_empty()
    }
}
```

### Creating a Custom Plugin
```rust
use loquat::plugins::{Plugin, PluginType};

pub struct MyPlugin {
    name: String,
    initialized: bool,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            name: "my_plugin".to_string(),
            initialized: false,
        }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Worker
    }

    async fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        self.initialized = false;
        self.initialize().await
    }
}
```

### Engine Initialization with Config
```rust
use loquat::config::LoquatConfig;

#[tokio::main]
async fn main() -> loquat::Result<()> {
    // Load configuration
    let config = LoquatConfig::from_environment("config", "dev")?;
    
    // Create application
    let app = LoquatApplication::from_config(config)?;
    
    // Run application
    app.run().await;
    
    Ok(())
}
```

## Testing Guidelines

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_registration() {
        let mut pool = create_test_pool(PoolType::Input);
        let worker = create_test_worker();
        let registration = WorkerRegistration::new(
            Box::new(worker),
            MatchingRule::All,
            0,
        );
        
        assert!(pool.register(registration).is_ok());
        assert_eq!(pool.worker_count(), 1);
    }
}
```

### Test Helpers
```rust
fn create_test_logger() -> Arc<dyn Logger> {
    let formatter = Arc::new(JsonFormatter::new());
    let writer = Arc::new(ConsoleWriter::new());
    Arc::new(StructuredLogger::new(formatter, writer))
}
```

## Project Structure

```
Loquat/
├── config/              # Configuration files
│   ├── default.toml
│   ├── dev.toml
│   ├── test.toml
│   └── prod.toml
├── dev-tools/          # Development scripts
│   ├── rebuild.bat
│   ├── clean.bat
│   └── check.bat
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library exports
│   ├── aop/            # AOP implementation
│   ├── adapters/       # Adapter system
│   ├── channel_manager/# Channel management
│   ├── channels/       # Channel types
│   ├── config/         # Configuration loading
│   ├── engine/         # Core engine
│   ├── errors/         # Error types
│   ├── events/         # Event structures
│   ├── logging/        # Logging system
│   ├── plugins/        # Plugin system
│   ├── pools/          # Pool implementations
│   ├── routers/        # Routing logic
│   ├── streams/        # Stream processing
│   └── workers/        # Worker system
├── plugins/            # Auto-created plugin directory
├── adapters/           # Auto-created adapter directory
├── logs/               # Auto-created log directory
├── start.bat           # Windows startup script
└── Cargo.toml
```

## Best Practices

### Performance
- Use `Arc` for shared data to avoid cloning
- Implement efficient `matches()` logic for workers
- Use batch processing in `handle_batch()` when possible
- Cache frequently accessed data

### Error Handling
- Always use `?` for error propagation
- Provide meaningful error messages
- Log errors at appropriate levels
- Handle errors gracefully in async contexts

### Concurrency
- Use `Arc<RwLock<T>>` for shared mutable state
- Prefer atomic operations for simple flags
- Be aware of deadlocks when acquiring multiple locks
- Use tokio primitives for async coordination

### Logging
- Always create `LogContext` with component name
- Add relevant metadata to log context
- Use appropriate log levels (Trace/Debug/Info/Warn/Error)
- Avoid logging in performance-critical loops

## Git Repository
- **Remote**: https://github.com/Full-finger/Loquat.git
- **Latest Commit**: 13249fc8ab8d573360ed28650fc3bf9133c099e8

## Notes
- This is an active project in development
- Windows batch scripts are the primary development tools
- Chinese documentation is available in README.md
- Framework is designed for extensibility and modularity
- Follow SOLID principles throughout the codebase
