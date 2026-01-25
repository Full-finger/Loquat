//! Path validation utility for security
//! 
//! Prevents directory traversal attacks by validating that paths
//! are within allowed directories

use std::path::{Path, PathBuf};
use crate::errors::{ConfigError, Result};

/// Path validator for security
pub struct PathValidator {
    /// Base directory that is allowed
    base_dir: PathBuf,
}

impl PathValidator {
    /// Create a new path validator with the specified base directory
    pub fn new(base_dir: &str) -> Result<Self> {
        let base_path = PathBuf::from(base_dir);
        
        // Validate that base directory exists
        if !base_path.exists() {
            return Err(ConfigError::ValidationError(
                format!("Base directory '{}' does not exist", base_dir)
            ).into());
        }
        
        // Validate that base directory is actually a directory
        if !base_path.is_dir() {
            return Err(ConfigError::ValidationError(
                format!("Base path '{}' is not a directory", base_dir)
            ).into());
        }
        
        // Canonicalize the base path to resolve any symlinks
        let canonical_base = base_path.canonicalize().map_err(|e| {
            ConfigError::ValidationError(
                format!("Failed to canonicalize base directory '{}': {}", base_dir, e)
            )
        })?;
        
        Ok(Self {
            base_dir: canonical_base,
        })
    }
    
    /// Create a path validator from a PathBuf
    pub fn from_path(base_dir: PathBuf) -> Result<Self> {
        Self::new(base_dir.to_str().ok_or_else(|| {
            ConfigError::ValidationError("Base directory path is not valid UTF-8".to_string())
        })?)
    }
    
    /// Validate that a path is within the allowed base directory
    /// 
    /// This prevents directory traversal attacks like `../../../etc/passwd`
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf> {
        // Check for path traversal components
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return Err(ConfigError::ValidationError(
                format!("Path '{}' contains '..' which is not allowed", path_str)
            ).into());
        }
        
        // Check if path is absolute
        if path.is_absolute() {
            return Err(ConfigError::ValidationError(
                format!("Path '{}' is absolute, only relative paths are allowed", path_str)
            ).into());
        }
        
        // Combine with base directory
        let full_path = self.base_dir.join(path);
        
        // Canonicalize to resolve any symlinks and relative components
        let canonical_path = full_path.canonicalize().map_err(|e| {
            ConfigError::ValidationError(
                format!("Failed to canonicalize path '{}': {}", full_path.display(), e)
            )
        })?;
        
        // Verify that the canonicalized path is still within base directory
        if !canonical_path.starts_with(&self.base_dir) {
            return Err(ConfigError::ValidationError(
                format!(
                    "Path '{}' resolves to '{}' which is outside allowed directory '{}'",
                    path.display(),
                    canonical_path.display(),
                    self.base_dir.display()
                )
            ).into());
        }
        
        Ok(canonical_path)
    }
    
    /// Validate a plugin or adapter file path
    pub fn validate_plugin_path(&self, path: &str) -> Result<PathBuf> {
        let file_name = PathBuf::from(path);
        
        // Check for path traversal in the file name itself
        let file_str = file_name.to_string_lossy();
        if file_str.contains("..") || file_str.contains("/") || file_str.contains("\\") {
            return Err(ConfigError::ValidationError(
                format!("Invalid plugin/adapter file name '{}': contains path separators or '..'", file_str)
            ).into());
        }
        
        // Validate the full path
        self.validate_path(&file_name)
    }
    
    /// Check if a path is valid (returns bool instead of Result)
    pub fn is_valid_path(&self, path: &Path) -> bool {
        self.validate_path(path).is_ok()
    }
    
    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_path_validator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathValidator::new(temp_dir.path().to_str().unwrap()).unwrap();
        
        // On Windows, canonicalize() returns UNC paths (\\?\C:\...)
        // On Unix, it returns: canonicalized path directly
        // Both should point to same directory, so we check if they refer to same file/directory
        let validator_path = validator.base_dir();
        let temp_path = temp_dir.path();
        
        // Check that validator can validate a path in: temp directory
        assert!(validator_path.exists());
        assert!(validator_path.is_dir());
        
        // Compare paths - both should refer to same location
        let canonical_temp = temp_path.canonicalize().unwrap_or_else(|_| temp_path.to_path_buf());
        assert!(validator_path.starts_with(&canonical_temp));
    }
    
    #[test]
    fn test_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathValidator::new(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Create a valid subdirectory
        let test_dir = temp_dir.path().join("test_plugin");
        fs::create_dir(&test_dir).unwrap();
        
        // Validate the path
        let path = Path::new("test_plugin");
        let result = validator.validate_path(&path);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_path_traversal_attack() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathValidator::new(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Try to access parent directory using ".."
        let path = Path::new("../etc/passwd");
        let result = validator.validate_path(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not allowed"));
    }
    
    #[test]
    fn test_absolute_path_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathValidator::new(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Try to use an absolute path
        let path = Path::new("/etc/passwd");
        let result = validator.validate_path(&path);
        
        // Should fail validation
        assert!(result.is_err());
        
        // Check error message (may vary by OS, but should indicate path validation failed)
        let error_msg = result.unwrap_err().to_string();
        // The error should mention that the path is absolute or invalid
        assert!(error_msg.contains("absolute") || error_msg.contains("not allowed") || error_msg.contains("validation"));
    }
    
    #[test]
    fn test_plugin_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let validator = PathValidator::new(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Create a test file
        let test_file = temp_dir.path().join("test.so");
        fs::File::create(&test_file).unwrap();
        
        // Valid plugin path
        let result = validator.validate_plugin_path("test.so");
        assert!(result.is_ok());
        
        // Invalid plugin path with ".."
        let result = validator.validate_plugin_path("../test.so");
        assert!(result.is_err());
        
        // Invalid plugin path with separator
        let result = validator.validate_plugin_path("subdir/test.so");
        assert!(result.is_err());
    }
}
