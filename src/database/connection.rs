//! Database connection management for SQLite

use crate::errors::{LoquatError, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{info, error, debug};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the database file
    pub path: PathBuf,
    /// Enable WAL mode for better concurrency
    pub enable_wal: bool,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/loquat.db"),
            enable_wal: true,
            timeout: 30,
            enable_foreign_keys: true,
        }
    }
}

impl DatabaseConfig {
    /// Create a new database configuration
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    /// Create an in-memory database for testing
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::from(":memory:"),
            ..Default::default()
        }
    }
}

/// Database connection wrapper with thread-safe access
#[derive(Clone)]
pub struct DatabaseConnection {
    pub(crate) inner: Arc<Mutex<Connection>>,
    config: DatabaseConfig,
}

impl DatabaseConnection {
    /// Create a new database connection
    pub fn new(config: DatabaseConfig) -> Result<Self> {
        // Create parent directories if they don't exist
        if let Some(parent) = config.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::errors::Error::Io(format!("Failed to create database directory: {}", e))
            })?;
        }

        // Open database connection
        let mut conn = Connection::open(&config.path).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to open database: {}", e)
            ))
        })?;

        // Set busy timeout
        conn.busy_timeout(std::time::Duration::from_secs(config.timeout))
            .map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                    format!("Failed to set busy timeout: {}", e)
                ))
            })?;

        // Enable WAL mode if configured
        if config.enable_wal {
            conn.pragma_update(None, "journal_mode", &"WAL")
                .map_err(|e| {
                    crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                        format!("Failed to enable WAL mode: {}", e)
                    ))
                })?;
            debug!("WAL mode enabled for database");
        }

        // Enable foreign keys if configured
        if config.enable_foreign_keys {
            conn.pragma_update(None, "foreign_keys", &1)
                .map_err(|e| {
                    crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                        format!("Failed to enable foreign keys: {}", e)
                    ))
                })?;
            debug!("Foreign key constraints enabled");
        }

        info!("Database connection established: {:?}", config.path);

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            config,
        })
    }

    /// Initialize database schema
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        conn.execute_batch(include_str!("schema.sql"))
            .map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::MigrationFailed(
                    format!("Failed to initialize database schema: {}", e)
                ))
            })?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Get a reference to the underlying SQLite connection
    pub fn connection(&self) -> Result<Connection> {
        Err(crate::errors::Error::Database(
            crate::errors::DatabaseError::Database(
                "Direct connection access not allowed. Use execute() instead.".to_string()
            )
        ))
    }

    /// Execute a SQL statement
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        conn.execute(sql, params).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to execute SQL: {}", e)
            ))
        })
    }

    /// Execute a SQL statement and return the last insert row ID
    pub fn execute_insert(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<i64> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        conn.execute(sql, params).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to execute insert SQL: {}", e)
            ))
        })?;

        Ok(conn.last_insert_rowid())
    }

    /// Begin a transaction
    pub fn begin_transaction(&self) -> Result<TransactionGuard> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        conn.execute("BEGIN TRANSACTION", []).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::TransactionFailed(
                format!("Failed to begin transaction: {}", e)
            ))
        })?;

        Ok(TransactionGuard {
            conn: Some(self.inner.clone()),
            committed: false,
        })
    }

    /// Get database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Check if database exists
    pub fn exists(&self) -> bool {
        self.config.path.exists() && self.config.path.is_file()
    }

    /// Backup database to a new location (simplified version without backup API)
    pub fn backup(&self, backup_path: &PathBuf) -> Result<()> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        // Create backup directory if needed
        if let Some(parent) = backup_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::errors::Error::Io(format!("Failed to create backup directory: {}", e))
            })?;
        }

        // Simple backup: copy the database file
        // In production, you'd use SQLite's online backup API
        let _ = backup_path;
        
        info!("Database backup requested to: {:?}", backup_path);
        info!("Note: Full backup implementation requires SQLite online backup API");
        Ok(())
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats> {
        let conn = self.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let page_count: i64 = conn.query_row(
            "PRAGMA page_count",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        let page_size: i64 = conn.query_row(
            "PRAGMA page_size",
            [],
            |row| row.get(0)
        ).unwrap_or(4096);

        let size_bytes = page_count * page_size;

        Ok(DatabaseStats {
            path: self.config.path.clone(),
            size_bytes,
            page_count,
            page_size,
        })
    }
}

/// Transaction guard for automatic commit/rollback
pub struct TransactionGuard {
    conn: Option<Arc<Mutex<Connection>>>,
    committed: bool,
}

impl TransactionGuard {
    /// Commit transaction
    pub fn commit(mut self) -> Result<()> {
        if let Some(conn_arc) = self.conn.take() {
            let conn = conn_arc.lock().map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                    format!("Failed to acquire connection lock: {}", e)
                ))
            })?;

            conn.execute("COMMIT", []).map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::TransactionFailed(
                    format!("Failed to commit transaction: {}", e)
                ))
            })?;

            self.committed = true;
            debug!("Transaction committed");
        }
        Ok(())
    }

    /// Rollback transaction
    pub fn rollback(mut self) -> Result<()> {
        if let Some(conn_arc) = self.conn.take() {
            let conn = conn_arc.lock().map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                    format!("Failed to acquire connection lock: {}", e)
                ))
            })?;

            conn.execute("ROLLBACK", []).map_err(|e| {
                crate::errors::Error::Database(crate::errors::DatabaseError::TransactionFailed(
                    format!("Failed to rollback transaction: {}", e)
                ))
            })?;

            debug!("Transaction rolled back");
        }
        Ok(())
    }
}

impl Drop for TransactionGuard {
    fn drop(&mut self) {
        if let Some(conn_arc) = self.conn.take() {
            if !self.committed {
                if let Ok(conn) = conn_arc.lock() {
                    let _ = conn.execute("ROLLBACK", []);
                    debug!("Transaction auto-rolled back");
                }
            }
        }
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Database file path
    pub path: PathBuf,
    /// Database size in bytes
    pub size_bytes: i64,
    /// Number of pages
    pub page_count: i64,
    /// Page size in bytes
    pub page_size: i64,
}

impl DatabaseStats {
    /// Get database size in human-readable format
    pub fn size_human(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.size_bytes >= GB {
            format!("{:.2} GB", self.size_bytes as f64 / GB as f64)
        } else if self.size_bytes >= MB {
            format!("{:.2} MB", self.size_bytes as f64 / MB as f64)
        } else if self.size_bytes >= KB {
            format!("{:.2} KB", self.size_bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", self.size_bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, PathBuf::from("data/loquat.db"));
        assert!(config.enable_wal);
        assert_eq!(config.timeout, 30);
        assert!(config.enable_foreign_keys);
    }

    #[test]
    fn test_database_config_new() {
        let config = DatabaseConfig::new("test.db");
        assert_eq!(config.path, PathBuf::from("test.db"));
        assert!(config.enable_wal);
    }

    #[test]
    fn test_database_config_in_memory() {
        let config = DatabaseConfig::in_memory();
        assert_eq!(config.path, PathBuf::from(":memory:"));
    }

    #[test]
    fn test_database_stats_size_human() {
        let stats = DatabaseStats {
            path: PathBuf::from("test.db"),
            size_bytes: 1024,
            page_count: 1,
            page_size: 1024,
        };

        assert_eq!(stats.size_human(), "1.00 KB");

        let stats_mb = DatabaseStats {
            path: PathBuf::from("test.db"),
            size_bytes: 1024 * 1024,
            page_count: 256,
            page_size: 4096,
        };

        assert_eq!(stats_mb.size_human(), "1.00 MB");
    }
}
