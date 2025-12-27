//! Hot reload history management for version tracking and rollback

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Hot reload entry tracking a specific reload event
#[derive(Debug, Clone)]
pub struct HotReloadEntry {
    /// Unique identifier for the entry
    pub id: String,
    /// Path to the plugin/adapter file
    pub path: PathBuf,
    /// Timestamp when the entry was created
    pub timestamp: std::time::SystemTime,
    /// File modification time
    pub modified_time: std::time::SystemTime,
    /// Optional file hash for content verification
    pub hash: Option<String>,
    /// Whether this reload was successful
    pub success: bool,
    /// Error message if reload failed
    pub error: Option<String>,
    /// Previous version data for rollback
    pub previous_data: Option<VersionData>,
}

/// Version data for rollback support
#[derive(Debug, Clone)]
pub struct VersionData {
    /// Version identifier
    pub version: String,
    /// Optional file hash
    pub hash: Option<String>,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// Hot reload history manager
#[derive(Clone)]
pub struct HotReloadHistory {
    /// History entries keyed by plugin/adapter name
    entries: Arc<RwLock<HashMap<String, Vec<HotReloadEntry>>>>,
    /// Maximum history entries per item
    max_entries: usize,
}

impl HotReloadHistory {
    /// Create a new hot reload history manager
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
        }
    }

    /// Create with default max entries (10)
    pub fn with_default_capacity() -> Self {
        Self::new(10)
    }

    /// Record a hot reload attempt
    pub async fn record_reload(
        &self,
        name: &str,
        path: PathBuf,
        success: bool,
        error: Option<String>,
        previous_data: Option<VersionData>,
    ) {
        let modified_time = path.metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(std::time::SystemTime::now);

        let entry = HotReloadEntry {
            id: format!("{}-{:?}", name, std::time::Instant::now()),
            path,
            timestamp: std::time::SystemTime::now(),
            modified_time,
            hash: None, // Can be computed if needed
            success,
            error,
            previous_data,
        };

        let mut entries = self.entries.write().await;
        let history = entries.entry(name.to_string()).or_insert_with(Vec::new);
        history.push(entry);

        // Trim history to max entries
        if history.len() > self.max_entries {
            history.drain(0..history.len() - self.max_entries);
        }
    }

    /// Get the last successful reload entry for a name
    pub async fn get_last_success(&self, name: &str) -> Option<HotReloadEntry> {
        let entries = self.entries.read().await;
        entries.get(name)
            .and_then(|history| history.iter().rev().find(|e| e.success))
            .cloned()
    }

    /// Get the last reload entry (regardless of success)
    pub async fn get_last(&self, name: &str) -> Option<HotReloadEntry> {
        let entries = self.entries.read().await;
        entries.get(name)
            .and_then(|history| history.last())
            .cloned()
    }

    /// Get all history for a name
    pub async fn get_history(&self, name: &str) -> Vec<HotReloadEntry> {
        let entries = self.entries.read().await;
        entries.get(name).cloned().unwrap_or_default()
    }

    /// Check if the last reload was successful
    pub async fn was_last_success(&self, name: &str) -> bool {
        self.get_last(name).await.map(|e| e.success).unwrap_or(true)
    }

    /// Get the last successful version data for rollback
    pub async fn get_rollback_data(&self, name: &str) -> Option<VersionData> {
        self.get_last_success(name).await
            .and_then(|e| e.previous_data)
    }

    /// Clear history for a name
    pub async fn clear(&self, name: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(name);
    }

    /// Clear all history
    pub async fn clear_all(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get statistics about the hot reload history
    pub async fn get_stats(&self) -> HotReloadStats {
        let entries = self.entries.read().await;
        let total_entries: usize = entries.values().map(|v| v.len()).sum();
        let successful_entries: usize = entries.values()
            .flat_map(|v| v.iter())
            .filter(|e| e.success)
            .count();

        let failed_entries = total_entries - successful_entries;

        HotReloadStats {
            total_items: entries.len(),
            total_entries,
            successful_entries,
            failed_entries,
        }
    }
}

/// Statistics about hot reload history
#[derive(Debug, Clone)]
pub struct HotReloadStats {
    /// Number of unique items being tracked
    pub total_items: usize,
    /// Total number of reload entries
    pub total_entries: usize,
    /// Number of successful reloads
    pub successful_entries: usize,
    /// Number of failed reloads
    pub failed_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hot_reload_history_creation() {
        let history = HotReloadHistory::new(10);
        let stats = history.get_stats().await;
        
        assert_eq!(stats.total_items, 0);
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_record_reload() {
        let history = HotReloadHistory::with_default_capacity();
        let path = PathBuf::from("/test/plugin.so");
        
        history.record_reload(
            "test_plugin",
            path.clone(),
            true,
            None,
            None,
        ).await;

        let last = history.get_last("test_plugin").await;
        assert!(last.is_some());
        assert!(last.unwrap().success);
    }

    #[tokio::test]
    async fn test_multiple_reloads() {
        let history = HotReloadHistory::with_default_capacity();
        let path = PathBuf::from("/test/plugin.so");
        
        // Record multiple reloads
        for i in 0..5 {
            history.record_reload(
                "test_plugin",
                path.clone(),
                i % 2 == 0,
                None,
                None,
            ).await;
        }

        let hist = history.get_history("test_plugin").await;
        assert_eq!(hist.len(), 5);
        
        let stats = history.get_stats().await;
        assert_eq!(stats.successful_entries, 3);
        assert_eq!(stats.failed_entries, 2);
    }

    #[tokio::test]
    async fn test_max_entries() {
        let history = HotReloadHistory::new(3);
        let path = PathBuf::from("/test/plugin.so");
        
        // Record more than max entries
        for i in 0..10 {
            history.record_reload(
                "test_plugin",
                path.clone(),
                true,
                None,
                None,
            ).await;
        }

        let hist = history.get_history("test_plugin").await;
        assert_eq!(hist.len(), 3); // Should be trimmed to max_entries
    }
}
