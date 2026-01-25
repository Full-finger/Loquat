# SQLite Database Integration for Loquat

## Summary

Successfully integrated SQLite database support into the Loquat framework for persistent data storage.

## What's New

### Database Module (`src/database/`)

1. **Connection Management** (`connection.rs`)
   - Thread-safe database connections
   - WAL mode support for better concurrency
   - Automatic schema initialization
   - Transaction support with automatic rollback

2. **Data Models** (`models.rs`)
   - `EventRecord` - Event persistence and tracking
   - `PluginRecord` - Plugin lifecycle management
   - `AdapterRecord` - Adapter state monitoring
   - `LogRecord` - Structured log storage
   - `StatsRecord` - Statistics data

3. **Repositories** (`repository.rs`)
   - `EventRepository` - Event CRUD operations
   - `PluginRepository` - Plugin management
   - `AdapterRepository` - Adapter state management
   - `LogRepository` - Log storage and retrieval

4. **Database Schema** (`schema.sql`)
   - Complete table definitions
   - Performance indexes
   - Automatic timestamp triggers

### Features

- ✅ SQLite database support
- ✅ Thread-safe operations with Mutex
- ✅ WAL mode for better concurrency
- ✅ Transaction support
- ✅ Comprehensive error handling
- ✅ Automatic schema initialization
- ✅ In-memory database support for testing
- ✅ Database statistics
- ✅ Old data cleanup utilities

### Database Schema

#### Events Table
Stores event data with processing status tracking
- Package ID, event type, source, target site
- Timestamp and raw JSON data
- Processing status (Pending, Processing, Success, Failed)

#### Plugins Table
Tracks plugin lifecycle and metadata
- Plugin name, version, type
- File path and configuration
- Status (Unloaded, Loading, Loaded, Failed, Disabled)
- Load count and last loaded timestamp

#### Adapters Table
Monitors adapter state and connections
- Adapter ID and type
- Configuration JSON
- Status (Uninitialized, Initializing, Ready, Running, Stopping, Stopped, Error)
- Connection status and timestamps

#### Logs Table
Centralized structured log storage
- Log level, message, component
- Timestamp and optional context
- Queryable by level or component

## Quick Start

```rust
use loquat::database::{DatabaseConnection, EventRepository};
use loquat::database::models::{EventRecord, EventStatus};

// Create database connection
let config = DatabaseConfig::default();
let conn = DatabaseConnection::new(config)?;
conn.initialize_schema()?;

// Use event repository
let event_repo = EventRepository::new(conn.clone());
let record = EventRecord {
    package_id: "pkg-123".to_string(),
    event_type: "message".to_string(),
    // ... other fields
};
event_repo.insert(&record)?;
```

## Files Added

```
src/database/
├── mod.rs           # Module exports
├── connection.rs    # Connection management
├── models.rs        # Data models
└── repository.rs    # CRUD operations

docs/
├── DATABASE_INTEGRATION.md  # Comprehensive usage guide
└── DATABASE_README.md       # This file
```

## Dependencies Added

```toml
rusqlite = { version = "0.30", features = ["bundled"] }
```

## Error Handling

Added `DatabaseError` enum to error types:
- `ConnectionFailed` - Database connection errors
- `QueryFailed` - SQL query execution errors
- `TransactionFailed` - Transaction management errors
- `MigrationFailed` - Schema migration errors
- `Database` - General database errors

## Testing

All tests pass successfully:
```
test result: ok. 340 passed; 0 failed; 0 ignored; 0 measured
```

### Running Database Tests

```bash
cargo test --lib database
```

## Documentation

- **Integration Guide**: `docs/DATABASE_INTEGRATION.md`
  - Detailed usage examples
  - API reference
  - Best practices
  - Troubleshooting guide

## Usage Patterns

### 1. Event Tracking

```rust
// Insert event
event_repo.insert(&record)?;

// Update status
event_repo.update_status("pkg-123", EventStatus::Success, None)?;

// Query by status
let failed = event_repo.get_by_status(EventStatus::Failed, Some(100))?;

// Time range queries
let recent = event_repo.get_by_time_range(start, end, Some(50))?;
```

### 2. Plugin Management

```rust
// Register plugin
plugin_repo.upsert(&record)?;

// Update status
plugin_repo.update_status("my_plugin", PluginStatus::Loaded, None)?;

// Track loads
plugin_repo.increment_load_count("my_plugin")?;

// Get all plugins
let all = plugin_repo.get_all()?;
```

### 3. Adapter Monitoring

```rust
// Register adapter
adapter_repo.upsert(&record)?;

// Update status
adapter_repo.update_status("console", AdapterStatus::Running, None)?;

// Update connection
adapter_repo.update_connected("console", true)?;

// Get all adapters
let all = adapter_repo.get_all()?;
```

### 4. Log Storage

```rust
// Store log
log_repo.insert(&record)?;

// Query by level
let errors = log_repo.get_by_level("ERROR", Some(100))?;

// Query by component
let logs = log_repo.get_by_component("main", Some(50))?;
```

### 5. Transactions

```rust
let tx = conn.begin_transaction()?;

try {
    event_repo.insert(&event_record)?;
    plugin_repo.upsert(&plugin_record)?;
    tx.commit()?;
} catch (e) {
    tx.rollback()?;
    return Err(e.into());
}
```

## Configuration

```rust
// Default: data/loquat.db
let config = DatabaseConfig::default();

// Custom path
let config = DatabaseConfig::new("custom/path.db");

// In-memory (for testing)
let config = DatabaseConfig::in_memory();

// Full configuration
let config = DatabaseConfig {
    path: PathBuf::from("data/loquat.db"),
    enable_wal: true,
    timeout: 30,
    enable_foreign_keys: true,
};
```

## Performance Considerations

- **WAL Mode**: Enabled by default for better concurrency
- **Indexes**: Pre-configured on commonly queried fields
- **Connection Pooling**: Shared connection with Mutex (suitable for most use cases)
- **Transactions**: Use for multiple related operations

## Maintenance

```rust
// Get database statistics
let stats = conn.get_stats()?;

// Clean up old data (older than 30 days)
let cutoff = Utc::now() - Duration::days(30);
event_repo.delete_old_events(cutoff)?;
log_repo.delete_old_logs(cutoff)?;
```

## Future Enhancements

Potential improvements for future versions:

1. **Connection Pooling** - For high-concurrency scenarios
2. **Query Builder** - For complex queries
3. **Migration System** - For schema versioning
4. **Backup/Restore** - Database management utilities
5. **Performance Monitoring** - Query performance tracking
6. **Query Caching** - For frequently accessed data
7. **Async Operations** - Async repository methods

## Integration with Existing Systems

The database module can be integrated with existing Loquat components:

### Engine Integration
- Store event processing results
- Track engine statistics
- Persist engine configuration

### Plugin Manager Integration
- Track plugin lifecycle
- Store plugin metadata
- Monitor plugin performance

### Adapter Manager Integration
- Monitor adapter connections
- Track adapter statistics
- Store adapter configurations

### Logging Integration
- Centralized log storage
- Queryable log history
- Log analysis and reporting

## Troubleshooting

### Database Locked
- Ensure WAL mode is enabled
- Check for long-running transactions
- Verify no external processes accessing database

### Connection Timeout
- Increase timeout in DatabaseConfig
- Check file permissions
- Verify disk space

### Schema Initialization Failed
- Check write permissions
- Verify database isn't corrupted
- Delete and recreate database (with backup)

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/Full-finger/Loquat/issues
- Documentation: `docs/DATABASE_INTEGRATION.md`

## License

This database integration follows the same license as the Loquat framework.
