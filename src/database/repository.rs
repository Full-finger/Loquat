//! Database repository implementations for Loquat framework

use crate::database::connection::DatabaseConnection;
use crate::database::models::*;
use crate::errors::{LoquatError, Result};
use chrono::{DateTime, Utc, TimeZone};
use rusqlite::params;
use tracing::{debug, error, info};

/// Event repository for managing event records
pub struct EventRepository {
    conn: DatabaseConnection,
}

impl EventRepository {
    /// Create a new event repository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert a new event record
    pub fn insert(&self, record: &EventRecord) -> Result<i64> {
        debug!("Inserting event record: package_id={}", record.package_id);

        let sql = r#"
            INSERT INTO events (
                package_id, event_type, source, target_site, timestamp,
                raw_data, status, error_message, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#;

        self.conn.execute_insert(
            sql,
            params![
                &record.package_id,
                &record.event_type,
                &record.source,
                &record.target_site,
                &record.timestamp.to_rfc3339(),
                &record.raw_data,
                record.status.as_i32(),
                &record.error_message,
                &record.created_at.to_rfc3339(),
            ],
        )
    }

    /// Get an event record by package ID
    pub fn get_by_package_id(&self, package_id: &str) -> Result<Option<EventRecord>> {
        let sql = "SELECT * FROM events WHERE package_id = ?1";

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let result = conn.query_row(sql, params![package_id], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                package_id: row.get(1)?,
                event_type: row.get(2)?,
                source: row.get(3)?,
                target_site: row.get(4)?,
                timestamp: parse_datetime(&row.get::<_, String>(5)?),
                raw_data: row.get(6)?,
                status: EventStatus::from_i32(row.get(7)?),
                error_message: row.get(8)?,
                created_at: parse_datetime(&row.get::<_, String>(9)?),
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            )),
        }
    }

    /// Update event status
    pub fn update_status(&self, package_id: &str, status: EventStatus, error_message: Option<String>) -> Result<()> {
        debug!("Updating event status: package_id={}, status={:?}", package_id, status);

        let sql = r#"
            UPDATE events SET status = ?1, error_message = ?2 WHERE package_id = ?3
        "#;

        let rows = self.conn.execute(
            sql,
            params![status.as_i32(), error_message, package_id],
        )?;

        if rows == 0 {
            error!("No event found with package_id: {}", package_id);
            return Err(LoquatError::Internal(format!(
                "Event not found: {}", package_id
            )));
        }

        Ok(())
    }

    /// Get events by status
    pub fn get_by_status(&self, status: EventStatus, limit: Option<usize>) -> Result<Vec<EventRecord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let sql = format!("SELECT * FROM events WHERE status = ?1 ORDER BY created_at DESC{}", limit_clause);

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map(params![status.as_i32()], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    package_id: row.get(1)?,
                    event_type: row.get(2)?,
                    source: row.get(3)?,
                    target_site: row.get(4)?,
                    timestamp: parse_datetime(&row.get::<_, String>(5)?),
                    raw_data: row.get(6)?,
                    status: EventStatus::from_i32(row.get(7)?),
                    error_message: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Get events by time range
    pub fn get_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<EventRecord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let sql = format!(
            "SELECT * FROM events WHERE timestamp BETWEEN ?1 AND ?2 ORDER BY timestamp DESC{}",
            limit_clause
        );

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    package_id: row.get(1)?,
                    event_type: row.get(2)?,
                    source: row.get(3)?,
                    target_site: row.get(4)?,
                    timestamp: parse_datetime(&row.get::<_, String>(5)?),
                    raw_data: row.get(6)?,
                    status: EventStatus::from_i32(row.get(7)?),
                    error_message: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Delete old events
    pub fn delete_old_events(&self, before: DateTime<Utc>) -> Result<usize> {
        let sql = "DELETE FROM events WHERE created_at < ?1";

        let rows = self.conn.execute(sql, params![before.to_rfc3339()])?;
        info!("Deleted {} old events", rows);

        Ok(rows)
    }
}

/// Plugin repository for managing plugin records
pub struct PluginRepository {
    conn: DatabaseConnection,
}

impl PluginRepository {
    /// Create a new plugin repository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert or update a plugin record
    pub fn upsert(&self, record: &PluginRecord) -> Result<i64> {
        debug!("Upserting plugin record: name={}", record.name);

        let sql = r#"
            INSERT INTO plugins (
                name, version, plugin_type, file_path, status,
                last_loaded_at, load_count, error_message, metadata,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(name) DO UPDATE SET
                version = excluded.version,
                plugin_type = excluded.plugin_type,
                file_path = excluded.file_path,
                status = excluded.status,
                last_loaded_at = excluded.last_loaded_at,
                load_count = excluded.load_count,
                error_message = excluded.error_message,
                metadata = excluded.metadata
        "#;

        self.conn.execute_insert(
            sql,
            params![
                &record.name,
                &record.version,
                &record.plugin_type,
                &record.file_path,
                record.status.as_i32(),
                record.last_loaded_at.map(|d| d.to_rfc3339()),
                record.load_count,
                &record.error_message,
                &record.metadata,
                &record.created_at.to_rfc3339(),
                &record.updated_at.to_rfc3339(),
            ],
        )
    }

    /// Get a plugin record by name
    pub fn get_by_name(&self, name: &str) -> Result<Option<PluginRecord>> {
        let sql = "SELECT * FROM plugins WHERE name = ?1";

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let result = conn.query_row(sql, params![name], |row| {
            Ok(PluginRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                plugin_type: row.get(3)?,
                file_path: row.get(4)?,
                status: PluginStatus::from_i32(row.get(5)?),
                last_loaded_at: row.get::<_, Option<String>>(6)?.and_then(|s| parse_datetime_opt(&s)),
                load_count: row.get(7)?,
                error_message: row.get(8)?,
                metadata: row.get(9)?,
                created_at: parse_datetime(&row.get::<_, String>(10)?),
                updated_at: parse_datetime(&row.get::<_, String>(11)?),
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            )),
        }
    }

    /// Get all plugins
    pub fn get_all(&self) -> Result<Vec<PluginRecord>> {
        let sql = "SELECT * FROM plugins ORDER BY name";

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map([], |row| {
                Ok(PluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    plugin_type: row.get(3)?,
                    file_path: row.get(4)?,
                    status: PluginStatus::from_i32(row.get(5)?),
                    last_loaded_at: row.get::<_, Option<String>>(6)?.and_then(|s| parse_datetime_opt(&s)),
                    load_count: row.get(7)?,
                    error_message: row.get(8)?,
                    metadata: row.get(9)?,
                    created_at: parse_datetime(&row.get::<_, String>(10)?),
                    updated_at: parse_datetime(&row.get::<_, String>(11)?),
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Update plugin status
    pub fn update_status(&self, name: &str, status: PluginStatus, error_message: Option<String>) -> Result<()> {
        debug!("Updating plugin status: name={}, status={:?}", name, status);

        let sql = r#"
            UPDATE plugins SET status = ?1, error_message = ?2 WHERE name = ?3
        "#;

        let rows = self.conn.execute(sql, params![status.as_i32(), error_message, name])?;

        if rows == 0 {
            return Err(LoquatError::Internal(format!("Plugin not found: {}", name)));
        }

        Ok(())
    }

    /// Increment plugin load count
    pub fn increment_load_count(&self, name: &str) -> Result<()> {
        let sql = "UPDATE plugins SET load_count = load_count + 1, last_loaded_at = ?1 WHERE name = ?2";

        let rows = self.conn.execute(sql, params![Utc::now().to_rfc3339(), name])?;

        if rows == 0 {
            return Err(LoquatError::Internal(format!("Plugin not found: {}", name)));
        }

        Ok(())
    }
}

/// Adapter repository for managing adapter records
pub struct AdapterRepository {
    conn: DatabaseConnection,
}

impl AdapterRepository {
    /// Create a new adapter repository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert or update an adapter record
    pub fn upsert(&self, record: &AdapterRecord) -> Result<i64> {
        debug!("Upserting adapter record: adapter_id={}", record.adapter_id);

        let sql = r#"
            INSERT INTO adapters (
                adapter_id, adapter_type, config, status, connected,
                last_started_at, last_stopped_at, error_message,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(adapter_id) DO UPDATE SET
                adapter_type = excluded.adapter_type,
                config = excluded.config,
                status = excluded.status,
                connected = excluded.connected,
                last_started_at = excluded.last_started_at,
                last_stopped_at = excluded.last_stopped_at,
                error_message = excluded.error_message
        "#;

        self.conn.execute_insert(
            sql,
            params![
                &record.adapter_id,
                &record.adapter_type,
                &record.config,
                record.status.as_i32(),
                record.connected as i32,
                record.last_started_at.map(|d| d.to_rfc3339()),
                record.last_stopped_at.map(|d| d.to_rfc3339()),
                &record.error_message,
                &record.created_at.to_rfc3339(),
                &record.updated_at.to_rfc3339(),
            ],
        )
    }

    /// Get an adapter record by ID
    pub fn get_by_id(&self, adapter_id: &str) -> Result<Option<AdapterRecord>> {
        let sql = "SELECT * FROM adapters WHERE adapter_id = ?1";

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let result = conn.query_row(sql, params![adapter_id], |row| {
            Ok(AdapterRecord {
                id: row.get(0)?,
                adapter_id: row.get(1)?,
                adapter_type: row.get(2)?,
                config: row.get(3)?,
                status: AdapterStatus::from_i32(row.get(4)?),
                connected: row.get::<_, i32>(5)? != 0,
                last_started_at: row.get::<_, Option<String>>(6)?.and_then(|s| parse_datetime_opt(&s)),
                last_stopped_at: row.get::<_, Option<String>>(7)?.and_then(|s| parse_datetime_opt(&s)),
                error_message: row.get(8)?,
                created_at: parse_datetime(&row.get::<_, String>(9)?),
                updated_at: parse_datetime(&row.get::<_, String>(10)?),
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            )),
        }
    }

    /// Get all adapters
    pub fn get_all(&self) -> Result<Vec<AdapterRecord>> {
        let sql = "SELECT * FROM adapters ORDER BY adapter_id";

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map([], |row| {
                Ok(AdapterRecord {
                    id: row.get(0)?,
                    adapter_id: row.get(1)?,
                    adapter_type: row.get(2)?,
                    config: row.get(3)?,
                    status: AdapterStatus::from_i32(row.get(4)?),
                    connected: row.get::<_, i32>(5)? != 0,
                    last_started_at: row.get::<_, Option<String>>(6)?.and_then(|s| parse_datetime_opt(&s)),
                    last_stopped_at: row.get::<_, Option<String>>(7)?.and_then(|s| parse_datetime_opt(&s)),
                    error_message: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                    updated_at: parse_datetime(&row.get::<_, String>(10)?),
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Update adapter status
    pub fn update_status(&self, adapter_id: &str, status: AdapterStatus, error_message: Option<String>) -> Result<()> {
        debug!("Updating adapter status: adapter_id={}, status={:?}", adapter_id, status);

        let sql = r#"
            UPDATE adapters SET status = ?1, error_message = ?2 WHERE adapter_id = ?3
        "#;

        let rows = self.conn.execute(sql, params![status.as_i32(), error_message, adapter_id])?;

        if rows == 0 {
            return Err(LoquatError::Internal(format!("Adapter not found: {}", adapter_id)));
        }

        Ok(())
    }

    /// Update adapter connection status
    pub fn update_connected(&self, adapter_id: &str, connected: bool) -> Result<()> {
        let sql = "UPDATE adapters SET connected = ?1 WHERE adapter_id = ?2";

        let rows = self.conn.execute(sql, params![connected as i32, adapter_id])?;

        if rows == 0 {
            return Err(LoquatError::Internal(format!("Adapter not found: {}", adapter_id)));
        }

        Ok(())
    }
}

/// Log repository for managing log records
pub struct LogRepository {
    conn: DatabaseConnection,
}

impl LogRepository {
    /// Create a new log repository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert a new log record
    pub fn insert(&self, record: &LogRecord) -> Result<i64> {
        let sql = r#"
            INSERT INTO logs (level, message, component, timestamp, context)
            VALUES (?1, ?2, ?3, ?4, ?5)
        "#;

        self.conn.execute_insert(
            sql,
            params![
                &record.level,
                &record.message,
                &record.component,
                &record.timestamp.to_rfc3339(),
                &record.context,
            ],
        )
    }

    /// Get logs by level
    pub fn get_by_level(&self, level: &str, limit: Option<usize>) -> Result<Vec<LogRecord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let sql = format!("SELECT * FROM logs WHERE level = ?1 ORDER BY timestamp DESC{}", limit_clause);

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map(params![level], |row| {
                Ok(LogRecord {
                    id: row.get(0)?,
                    level: row.get(1)?,
                    message: row.get(2)?,
                    component: row.get(3)?,
                    timestamp: parse_datetime(&row.get::<_, String>(4)?),
                    context: row.get(5)?,
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Get logs by component
    pub fn get_by_component(&self, component: &str, limit: Option<usize>) -> Result<Vec<LogRecord>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let sql = format!("SELECT * FROM logs WHERE component = ?1 ORDER BY timestamp DESC{}", limit_clause);

        let conn = self.conn.inner.lock().map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::ConnectionFailed(
                format!("Failed to acquire connection lock: {}", e)
            ))
        })?;

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            crate::errors::Error::Database(crate::errors::DatabaseError::QueryFailed(
                format!("Failed to prepare statement: {}", e)
            ))
        })?;

        let records = stmt
            .query_map(params![component], |row| {
                Ok(LogRecord {
                    id: row.get(0)?,
                    level: row.get(1)?,
                    message: row.get(2)?,
                    component: row.get(3)?,
                    timestamp: parse_datetime(&row.get::<_, String>(4)?),
                    context: row.get(5)?,
                })
            })
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Query failed: {}", e))
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| crate::errors::Error::Database(
                crate::errors::DatabaseError::QueryFailed(format!("Row extraction failed: {}", e))
            ))?;

        Ok(records)
    }

    /// Delete old logs
    pub fn delete_old_logs(&self, before: DateTime<Utc>) -> Result<usize> {
        let sql = "DELETE FROM logs WHERE timestamp < ?1";

        let rows = self.conn.execute(sql, params![before.to_rfc3339()])?;
        debug!("Deleted {} old logs", rows);

        Ok(rows)
    }
}

/// Parse datetime from RFC3339 string
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Parse optional datetime from RFC3339 string
fn parse_datetime_opt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        let dt_str = "2024-01-01T12:00:00Z";
        let dt = parse_datetime(dt_str);
        
        assert_eq!(dt.timestamp(), 1704110400);
    }

    #[test]
    fn test_parse_datetime_opt() {
        let dt_str = "2024-01-01T12:00:00Z";
        let dt = parse_datetime_opt(dt_str);
        
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().timestamp(), 1704110400);

        let invalid_dt = parse_datetime_opt("invalid");
        assert!(invalid_dt.is_none());
    }
}
