# Loquat Database Integration Guide

## Overview

The Loquat framework now includes SQLite database support for persistent storage of events, plugins, adapters, and logs. This integration provides:

- Event persistence and tracking
- Plugin lifecycle management
- Adapter state monitoring
- Centralized logging storage
- Transaction support for data integrity

## Architecture

### Components

1. **Database Connection** (`src/database/connection.rs`)
   - Manages SQLite connections
   - Thread-safe access using Mutex
   - WAL mode support for better concurrency
   - Automatic schema initialization

2. **Data Models** (`src/database/models.rs`)
   - EventRecord: Stores event data and processing status
   - PluginRecord: Tracks plugin lifecycle
   - AdapterRecord: Monitors adapter state
   - LogRecord: Stores structured logs
   - StatsRecord: Holds statistics data

3. **Repositories** (`src/database/repository.rs`)
   - EventRepository: CRUD operations for events
   - PluginRepository: Plugin management operations
   - AdapterRepository: Adapter state management
   - LogRepository: Log storage and retrieval

4. **Database Schema** (`src/database/schema.sql`)
   - Complete table definitions
   - Indexes for performance
   - Triggers for automatic updates

## Database Schema

### Events Table

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    source TEXT NOT NULL,
    target_site TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    raw_data TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Status Values:**
- 0: Pending
- 1: Processing
- 2: Success
- 3: Failed

### Plugins Table

```sql
CREATE TABLE plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    version TEXT NOT NULL,
    plugin_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    last_loaded_at TEXT,
    load_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Status Values:**
- 0: Unloaded
- 1: Loading
- 2: Loaded
- 3: Failed
- 4: Disabled

### Adapters Table

```sql
CREATE TABLE adapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    adapter_id TEXT NOT NULL UNIQUE,
    adapter_type TEXT NOT NULL,
    config TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    connected INTEGER NOT NULL DEFAULT 0,
    last_started_at TEXT,
    last_stopped_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Status Values:**
- 0: Uninitialized
- 1: Initializing
- 2: Ready
- 3: Running
- 4: Stopping
- 5: Stopped
- 6: Error

### Logs Table

```sql
CREATE TABLE logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    component TEXT,
    timestamp TEXT NOT NULL,
    context TEXT
);
```

## Usage Examples

### 1. Initialize Database

```rust
use loquat::database::{DatabaseConnection, DatabaseConfig};

// Create database configuration
let config = DatabaseConfig::new("data/loquat.db");

// Create connection
let conn = DatabaseConnection::new(config)?;

// Initialize schema
conn.initialize_schema()?;
```

### 2. Use Event Repository

```rust
use loquat::database::{DatabaseConnection, EventRepository};
use loquat::database::models::{EventRecord, EventStatus};
use chrono::Utc;

let conn = DatabaseConnection::new(DatabaseConfig::default())?;
let event_repo = EventRepository::new(conn);

// Create event record
let record = EventRecord {
    id: 0, // Auto-generated
    package_id: "pkg-123".to_string(),
    event_type: "message".to_string(),
    source: "console".to_string(),
    target_site: "user-456".to_string(),
    timestamp: Utc::now(),
    raw_data: serde_json::to_string(&event_data)?,
    status: EventStatus::Pending,
    error_message: None,
    created_at: Utc::now(),
};

// Insert event
let id = event_repo.insert(&record)?;

// Update event status
event_repo.update_status("pkg-123", EventStatus::Success, None)?;

// Query events by status
let failed_events = event_repo.get_by_status(EventStatus::Failed, Some(100))?;

// Query events by time range
let start = Utc::now() - chrono::Duration::hours(24);
let end = Utc::now();
let recent_events = event_repo.get_by_time_range(start, end, Some(50))?;
```

### 3. Use Plugin Repository

```rust
use loquat::database::{DatabaseConnection, PluginRepository};
use loquat::database::models::{PluginRecord, PluginStatus};
use chrono::Utc;

let conn = DatabaseConnection::new(DatabaseConfig::default())?;
let plugin_repo = PluginRepository::new(conn);

// Create plugin record
let record = PluginRecord {
    id: 0,
    name: "my_plugin".to_string(),
    version: "1.0.0".to_string(),
    plugin_type: "Rust".to_string(),
    file_path: "plugins/my_plugin".to_string(),
    status: PluginStatus::Unloaded,
    last_loaded_at: None,
    load_count: 0,
    error_message: None,
    metadata: Some(serde_json::to_string(&metadata)?),
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

// Upsert plugin
let id = plugin_repo.upsert(&record)?;

// Update plugin status
plugin_repo.update_status("my_plugin", PluginStatus::Loaded, None)?;

// Increment load count
plugin_repo.increment_load_count("my_plugin")?;

// Get all plugins
let all_plugins = plugin_repo.get_all()?;
```

### 4. Use Adapter Repository

```rust
use loquat::database::{DatabaseConnection, AdapterRepository};
use loquat::database::models::{AdapterRecord, AdapterStatus};
use chrono::Utc;

let conn = DatabaseConnection::new(DatabaseConfig::default())?;
let adapter_repo = AdapterRepository::new(conn);

// Create adapter record
let record = AdapterRecord {
    id: 0,
    adapter_id: "console-adapter".to_string(),
    adapter_type: "console".to_string(),
    config: serde_json::to_string(&config)?,
    status: AdapterStatus::Ready,
    connected: false,
    last_started_at: None,
    last_stopped_at: None,
    error_message: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

// Upsert adapter
let id = adapter_repo.upsert(&record)?;

// Update adapter status
adapter_repo.update_status("console-adapter", AdapterStatus::Running, None)?;

// Update connection status
adapter_repo.update_connected("console-adapter", true)?;

// Get all adapters
let all_adapters = adapter_repo.get_all()?;
```

### 5. Use Log Repository

```rust
use loquat::database::{DatabaseConnection, LogRepository};
use loquat::database::models::LogRecord;
use chrono::Utc;

let conn = DatabaseConnection::new(DatabaseConfig::default())?;
let log_repo = LogRepository::new(conn);

// Create log record
let record = LogRecord {
    id: 0,
    level: "INFO".to_string(),
    message: "Application started".to_string(),
    component: Some("main".to_string()),
    timestamp: Utc::now(),
    context: Some(serde_json::to_string(&context)?),
};

// Insert log
let id = log_repo.insert(&record)?;

// Query logs by level
let error_logs = log_repo.get_by_level("ERROR", Some(100))?;

// Query logs by component
let component_logs = log_repo.get_by_component("main", Some(50))?;
```

### 6. Transaction Support

```rust
use loquat::database::{DatabaseConnection, EventRepository, PluginRepository};

let conn = DatabaseConnection::new(DatabaseConfig::default())?;

// Begin transaction
let tx = conn.begin_transaction()?;

try {
    // Perform multiple operations
    event_repo.insert(&event_record)?;
    plugin_repo.upsert(&plugin_record)?;
    
    // Commit if all operations succeed
    tx.commit()?;
} catch (e) {
    // Rollback on error
    tx.rollback()?;
    return Err(e.into());
}
```

### 7. Database Maintenance

```rust
use loquat::database::DatabaseConnection;
use chrono::{Utc, Duration};

let conn = DatabaseConnection::new(DatabaseConfig::default())?;

// Get database statistics
let stats = conn.get_stats()?;
println!("Database size: {}", stats.size_human());

// Delete old data (older than 30 days)
let cutoff = Utc::now() - Duration::days(30);
event_repo.delete_old_events(cutoff)?;
log_repo.delete_old_logs(cutoff)?;
```

## Configuration

### Database Configuration Options

```rust
pub struct DatabaseConfig {
    pub path: PathBuf,           // Database file path
    pub enable_wal: bool,         // Enable WAL mode (recommended)
    pub timeout: u64,             // Connection timeout in seconds
    pub enable_foreign_keys: bool,  // Enable foreign key constraints
}
```

### Example Configurations

```rust
// Default configuration
let config = DatabaseConfig::default();
// Path: data/loquat.db
// WAL mode: enabled
// Timeout: 30 seconds
// Foreign keys: enabled

// Custom configuration
let config = DatabaseConfig {
    path: PathBuf::from("/custom/path/database.db"),
    enable_wal: true,
    timeout: 60,
    enable_foreign_keys: true,
};

// In-memory database (for testing)
let config = DatabaseConfig::in_memory();
```

## Best Practices

1. **Always Initialize Schema**
   ```rust
   let conn = DatabaseConnection::new(config)?;
   conn.initialize_schema()?; // Don't forget this!
   ```

2. **Use Transactions for Multiple Operations**
   - Ensures atomicity
   - Improves performance
   - Maintains data consistency

3. **Handle Errors Properly**
   - Database operations can fail
   - Always check Result types
   - Log errors appropriately

4. **Clean Up Old Data**
   - Implement periodic cleanup
   - Prevent database bloat
   - Maintain performance

5. **Use Indexes Wisely**
   - Schema includes necessary indexes
   - Add custom indexes if needed
   - Monitor query performance

6. **Connection Pooling**
   - Repository uses shared connection
   - Thread-safe by design
   - No manual pooling needed

## Performance Considerations

### WAL Mode
- Enabled by default
- Improves read/write concurrency
- Allows simultaneous reads and writes

### Indexes
- Schema includes indexes on common query fields
- Improves query performance
- Minimal overhead on inserts/updates

### Connection Management
- Single connection with mutex lock
- Suitable for most use cases
- Consider connection pooling for high-concurrency scenarios

## Troubleshooting

### Database Locked Error
```
Error: Database is locked
```
**Solution:**
- Ensure WAL mode is enabled
- Check for long-running transactions
- Verify no external processes are accessing the database

### Connection Timeout
```
Error: Database connection timeout
```
**Solution:**
- Increase timeout in DatabaseConfig
- Check database file permissions
- Verify sufficient disk space

### Schema Initialization Failed
```
Error: Failed to initialize database schema
```
**Solution:**
- Check write permissions on database directory
- Verify database file isn't corrupted
- Delete and recreate database (with backup)

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use loquat::database::DatabaseConfig;

    #[test]
    fn test_event_repository() {
        let config = DatabaseConfig::in_memory();
        let conn = DatabaseConnection::new(config).unwrap();
        conn.initialize_schema().unwrap();
        
        let repo = EventRepository::new(conn);
        // Test operations...
    }
}
```

## Future Enhancements

- Connection pooling for high-concurrency scenarios
- Query builder for complex queries
- Database migration system
- Backup and restore utilities
- Performance monitoring
- Query caching
- Async repository methods

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/Full-finger/Loquat/issues
- Documentation: https://github.com/Full-finger/Loquat/docs
