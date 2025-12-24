//! Log writer implementations

pub mod console;
pub mod file;

pub use console::*;
pub use file::*;

use crate::logging::traits::{LogOutput, LogWriter};
use std::sync::Arc;

/// Create a writer based on configuration
pub async fn create_writer(output: &LogOutput) -> Result<Arc<dyn LogWriter>, crate::errors::LoggingError> {
    match output {
        LogOutput::Console => Ok(Arc::new(ConsoleWriter::new())),
        LogOutput::File { path } => {
            let writer = FileWriter::new(path).await
                .map_err(|e| crate::errors::LoggingError::Configuration(e.to_string()))?;
            Ok(Arc::new(writer))
        }
        LogOutput::Both { path } => {
            let file_writer = FileWriter::new(path).await
                .map_err(|e| crate::errors::LoggingError::Configuration(e.to_string()))?;
            let console_writer = ConsoleWriter::new();
            let combined_writer = CombinedWriter::new(vec![
                Arc::new(console_writer),
                Arc::new(file_writer),
            ]);
            Ok(Arc::new(combined_writer))
        }
        LogOutput::Custom(_) => {
            Err(crate::errors::LoggingError::Configuration(
                "Custom writers not yet implemented".to_string()
            ))
        }
    }
}

/// Combined writer that writes to multiple destinations
pub struct CombinedWriter {
    writers: Vec<Arc<dyn LogWriter>>,
}

impl std::fmt::Debug for CombinedWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CombinedWriter")
            .field("writer_count", &self.writers.len())
            .finish()
    }
}

impl CombinedWriter {
    /// Create a new combined writer
    pub fn new(writers: Vec<Arc<dyn LogWriter>>) -> Self {
        Self { writers }
    }

    /// Add a writer to the combination
    pub fn add_writer(&mut self, writer: Arc<dyn LogWriter>) {
        self.writers.push(writer);
    }
}

#[async_trait::async_trait]
impl LogWriter for CombinedWriter {
    async fn write_async(&self, formatted: &str) -> crate::errors::Result<()> {
        // Write to all writers, collecting errors
        let mut errors = Vec::new();
        
        for writer in &self.writers {
            if let Err(e) = writer.write_async(formatted).await {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(crate::errors::LoggingError::WriteError(
                format!("Multiple write errors: {:?}", errors)
            ).into())
        }
    }

    fn write(&self, formatted: &str) -> crate::errors::Result<()> {
        // Write to all writers, collecting errors
        let mut errors = Vec::new();
        
        for writer in &self.writers {
            if let Err(e) = writer.write(formatted) {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(crate::errors::LoggingError::WriteError(
                format!("Multiple write errors: {:?}", errors)
            ).into())
        }
    }

    fn flush(&self) -> crate::errors::Result<()> {
        let mut errors = Vec::new();
        
        for writer in &self.writers {
            if let Err(e) = writer.flush() {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(crate::errors::LoggingError::WriteError(
                format!("Multiple flush errors: {:?}", errors)
            ).into())
        }
    }

    async fn flush_async(&self) -> crate::errors::Result<()> {
        let mut errors = Vec::new();
        
        for writer in &self.writers {
            if let Err(e) = writer.flush_async().await {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(crate::errors::LoggingError::WriteError(
                format!("Multiple flush errors: {:?}", errors)
            ).into())
        }
    }

    async fn close_async(&self) -> crate::errors::Result<()> {
        let mut errors = Vec::new();
        
        for writer in &self.writers {
            if let Err(e) = writer.close_async().await {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(crate::errors::LoggingError::WriteError(
                format!("Multiple close errors: {:?}", errors)
            ).into())
        }
    }
}
