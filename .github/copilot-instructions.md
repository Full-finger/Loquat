# Loquat Framework - AI Coding Guidelines

## Architecture Overview
Loquat is a Rust framework for building robot/agent services with a 9-stage pipeline architecture. Core flow: `Engine` → `Router` → `ChannelManager` → `Stream` (9 ordered `Pool`s) → `Worker`s.

- **Data Hierarchy**: `Package` (top-level unit) → `Block` (event array) → `Group` → `Event` (individual message/notice/request)
- **Channel Types**: `group:`, `private:`, `channel:` (extracted from package IDs like `group:123`)
- **Extensible Pools**: `Input`, `PreProcess`, `Process`, `Output` allow third-party `Worker` registration

## Key Patterns
- **Dependency Injection**: Use `Arc<dyn Trait>` for all services (e.g., `Arc<dyn Logger>`, `Arc<dyn Stream>`)
- **Async Traits**: Apply `#[async_trait]` to all async trait methods
- **Error Handling**: Return `Result<T, LoquatError>`; use `?` for propagation
- **Logging**: Create `LogContext` with component/package_id; log at appropriate `LogLevel`
- **AOP Integration**: Implement `Aspect` trait for cross-cutting concerns; use `execute_with_aspects` for weaving

## Common Implementation Examples
- **Engine Processing**: Route via `router.route_package()`, get/create channel via `channel_manager.get_or_create_channel()`, process through stream
- **Pool Registration**: Workers register in specific pools with priority; pools process packages sequentially
- **Event Handling**: Match on `EventEnum` variants (Message, Notice, Request, Meta) for type-specific logic
- **Serialization**: Derive `Serialize/Deserialize` for all data structs; use `serde_json` for JSON handling

## Development Workflow
- **Build**: `cargo build` (edition 2024)
- **Test**: `cargo test` (focus on async integration tests)
- **Run Examples**: `cargo run --example basic_usage` (demonstrates AOP + logging)
- **Debug**: Enable `RUST_LOG=debug` for tracing-subscriber output

## Code Style Conventions
- **Naming**: PascalCase for types, snake_case for fields/methods; use descriptive names (e.g., `StandardEngine`, `process_pipeline`)
- **Modules**: Flat structure with `mod.rs` re-exports; use `prelude` module for common imports
- **Concurrency**: `Arc<RwLock<T>>` for shared mutable state; atomic operations for status flags
- **Traits**: Define behavior traits in `traits.rs`; provide standard implementations in adjacent files

## Integration Points
- **Plugins**: Implement `Plugin` trait; register via `PluginManager`
- **Adapters**: Convert external events to `Package` via `Adapter` trait
- **Web Services**: Use `axum` for HTTP endpoints; integrate with engine processing
- **External Logging**: Bridge to `tracing` ecosystem via custom `Logger` implementations</content>
<parameter name="filePath">c:\Users\gyh20\Desktop\Rust\Loquat\.github\copilot-instructions.md