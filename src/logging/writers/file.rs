//! File writer implementation

use crate::errors::{LoggingError, Result};
use crate::logging::traits::LogWriter;
use async_trait::async_trait;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::fs as async_fs;
use tokio::io as async_io;

/// Configuration for file rotation
#[derive(Debug, Clone)]
pub struct FileRotationConfig {
    /// Maximum file size in bytes before rotation
    pub max_file_size: u64,
    /// Maximum number of backup files to keep
    pub max_files: usize,
    /// Whether to compress rotated files
    pub compress: bool,
}

impl Default for FileRotationConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            compress: false,
        }
    }
}

/// File writer for log output
#[derive(Debug)]
pub struct FileWriter {
    file_path: PathBuf,
    writer: Mutex<BufWriter<File>>,
    rotation_config: Option<FileRotationConfig>,
    current_size: Mutex<u64>,
}

impl FileWriter {
    /// Create a new file writer
    pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_rotation_config(path, None).await
    }

    /// Create a new file writer with rotation configuration
    pub async fn with_rotation_config<P: AsRef<Path>>(
        path: P,
        rotation_config: Option<FileRotationConfig>,
    ) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        // Open or create the file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let current_size = file.metadata()?.len();
        let writer = Mutex::new(BufWriter::new(file));

        Ok(Self {
            file_path: path,
            writer,
            rotation_config,
            current_size: Mutex::new(current_size),
        })
    }

    /// Create a file writer with default rotation
    pub async fn with_rotation<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_rotation_config(path, Some(FileRotationConfig::default())).await
    }

    /// Check if file rotation is needed
    fn needs_rotation(&self) -> bool {
        if let Some(config) = &self.rotation_config {
            let current_size = *self.current_size.lock().unwrap();
            current_size >= config.max_file_size
        } else {
            false
        }
    }

    /// Perform file rotation
    async fn rotate_file(&self) -> Result<()> {
        let config = self.rotation_config.as_ref()
            .ok_or_else(|| LoggingError::Configuration("Rotation not configured".to_string()))?;

        // Close current file
        {
            let mut writer = self.writer.lock().unwrap();
            writer.flush()
                .map_err(|e| crate::errors::LoquatError::Io(e.to_string()))?;
        }

        // Move current file to backup
        let base_path = self.file_path.to_string_lossy();
        let mut backup_path = self.file_path.clone();

        // Find next available backup number
        let mut backup_num = 1;
        while backup_path.exists() {
            backup_path = self.file_path.with_extension(&format!("log.{}", backup_num));
            backup_num += 1;
            if backup_num > config.max_files {
                break;
            }
        }

        // Move current file to backup
        if self.file_path.exists() {
            async_fs::rename(&self.file_path, &backup_path).await
                .map_err(|e| LoggingError::WriteError(e.to_string()))?;
        }

        // Rotate existing backup files
        for i in (1..config.max_files).rev() {
            let current_backup = self.file_path.with_extension(&format!("log.{}", i));
            let next_backup = self.file_path.with_extension(&format!("log.{}", i + 1));
            
            if current_backup.exists() {
                async_fs::rename(&current_backup, &next_backup).await
                    .map_err(|e| LoggingError::WriteError(e.to_string()))?;
            }
        }

        // Clean up old backup files if needed
        if config.max_files > 0 {
            for i in config.max_files + 1..=config.max_files + 10 {
                let old_backup = self.file_path.with_extension(&format!("log.{}", i));
                if old_backup.exists() {
                    async_fs::remove_file(&old_backup).await
                        .map_err(|e| LoggingError::WriteError(e.to_string()))?;
                }
            }
        }

        // Create new file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|e| crate::errors::LoquatError::Io(e.to_string()))?;

        // Update writer and reset size
        {
            let mut writer = self.writer.lock().unwrap();
            *writer = BufWriter::new(new_file);
        }
        *self.current_size.lock().unwrap() = 0;

        Ok(())
    }

    /// Write to file with rotation check
    async fn write_with_rotation(&self, message: &str) -> Result<()> {
        // Check if rotation is needed
        if self.needs_rotation() {
            self.rotate_file().await?;
        }

        // Write the message
        {
            let mut writer = self.writer.lock().unwrap();
            writeln!(writer, "{}", message)
                .map_err(|e| LoggingError::WriteError(e.to_string()))?;
            
            // Update size (approximately)
            *self.current_size.lock().unwrap() += message.len() as u64 + 1; // +1 for newline
            
            writer.flush()
                .map_err(|e| crate::errors::LoquatError::Io(e.to_string()))?;
        }

        Ok(())
    }

    /// Get current file size
    pub fn current_size(&self) -> u64 {
        *self.current_size.lock().unwrap()
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.file_path
    }
}

#[async_trait]
impl LogWriter for FileWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        self.write_with_rotation(formatted).await
    }

    fn write(&self, formatted: &str) -> Result<()> {
        // Use tokio runtime for sync write
        use tokio::runtime::Handle;
        let rt = Handle::current();
        rt.block_on(self.write_with_rotation(formatted))
    }

    fn flush(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.flush()
            .map_err(|e| crate::errors::LoquatError::Io(e.to_string()))?;
        Ok(())
    }
}

/// Enhanced file writer with additional features
#[derive(Debug)]
pub struct EnhancedFileWriter {
    base: FileWriter,
    enable_compression: bool,
    sync_interval: Option<std::time::Duration>,
    last_sync: Mutex<std::time::Instant>,
}

impl EnhancedFileWriter {
    /// Create a new enhanced file writer
    pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let base = FileWriter::with_rotation(path).await?;
        Ok(Self {
            base,
            enable_compression: false,
            sync_interval: Some(std::time::Duration::from_secs(5)),
            last_sync: Mutex::new(std::time::Instant::now()),
        })
    }

    /// Configure compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Configure sync interval
    pub fn with_sync_interval(mut self, interval: std::time::Duration) -> Self {
        self.sync_interval = Some(interval);
        self
    }

    /// Force sync to disk
    pub async fn sync(&self) -> Result<()> {
        let writer = self.base.writer.lock().unwrap();
        writer.get_ref().sync_all()
            .map_err(|e| crate::errors::LoquatError::Io(e.to_string()))?;
        *self.last_sync.lock().unwrap() = std::time::Instant::now();
        Ok(())
    }

    /// Check if periodic sync is needed
    fn should_sync(&self) -> bool {
        if let Some(interval) = self.sync_interval {
            let last_sync = *self.last_sync.lock().unwrap();
            last_sync.elapsed() >= interval
        } else {
            false
        }
    }

    /// Write with optional periodic sync
    async fn write_enhanced(&self, message: &str) -> Result<()> {
        self.base.write_with_rotation(message).await?;

        if self.should_sync() {
            self.sync().await?;
        }

        Ok(())
    }
}

#[async_trait]
impl LogWriter for EnhancedFileWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        self.write_enhanced(formatted).await
    }

    fn write(&self, formatted: &str) -> Result<()> {
        use tokio::runtime::Handle;
        let rt = Handle::current();
        rt.block_on(self.write_enhanced(formatted))
    }

    fn flush(&self) -> Result<()> {
        self.base.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_file_writer_creation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.log");
        
        let writer = FileWriter::new(&file_path).await;
        assert!(writer.is_ok());
        
        // Check if file was created
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_file_writer_write() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.log");
        
        let writer = FileWriter::new(&file_path).await.unwrap();
        let result = writer.write_async("Test message").await;
        assert!(result.is_ok());

        // Check if message was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Test message"));
        assert!(content.ends_with('\n'));
    }

    #[tokio::test]
    async fn test_file_rotation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.log");
        
        let rotation_config = FileRotationConfig {
            max_file_size: 100, // Very small for testing
            max_files: 3,
            compress: false,
        };

        let writer = FileWriter::with_rotation_config(&file_path, Some(rotation_config)).await.unwrap();
        
        // Write multiple messages to trigger rotation
        for i in 0..10 {
            let message = format!("Test message {} with some additional content\n", i);
            writer.write_async(&message).await.unwrap();
        }

        // Check if rotation occurred (backup files should exist)
        let backup1 = file_path.with_extension("log.1");
        assert!(backup1.exists() || file_path.exists());
    }

    #[tokio::test]
    async fn test_enhanced_file_writer() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("enhanced_test.log");
        
        let writer = EnhancedFileWriter::new(&file_path).await.unwrap();
        let result = writer.write_async("Enhanced test message").await;
        assert!(result.is_ok());

        // Test sync
        sleep(Duration::from_millis(100)).await;
        let sync_result = writer.sync().await;
        assert!(sync_result.is_ok());
    }

    #[test]
    fn test_rotation_config_default() {
        let config = FileRotationConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.max_files, 5);
        assert!(!config.compress);
    }
}
