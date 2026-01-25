//! File operations

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Create a directory if it doesn't exist
pub fn create_directory(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path))?;
    }
    Ok(())
}

/// Create a directory recursively
pub fn create_directory_recursive(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {:?}", path))?;
    }
    Ok(())
}

/// Write content to a file
pub fn write_file(path: &str, content: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        create_directory_recursive(parent)?;
    }
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path))?;
    
    Ok(())
}

/// Read content from a file
pub fn read_file(path: &str) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Check if a directory exists
pub fn directory_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Remove a file
pub fn remove_file(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        fs::remove_file(path)
            .with_context(|| format!("Failed to remove file: {}", path))?;
    }
    Ok(())
}

/// Remove a directory recursively
pub fn remove_directory(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory: {}", path))?;
    }
    Ok(())
}

/// Copy a file
pub fn copy_file(source: &str, destination: &str) -> Result<()> {
    fs::copy(source, destination)
        .with_context(|| format!("Failed to copy {} to {}", source, destination))?;
    Ok(())
}

/// List files in a directory
pub fn list_directory(path: &str) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path))?;
    
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        files.push(entry.path());
    }
    
    Ok(files)
}

/// Get project root directory
pub fn get_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    
    // Look for Cargo.toml or adapters directory
    for dir in current_dir.ancestors() {
        if dir.join("Cargo.toml").exists() || dir.join("adapters").exists() {
            return Ok(dir.to_path_buf());
        }
    }
    
    Err(anyhow::anyhow!("Could not find Loquat project root. Please run this command from a Loquat project directory."))
}

/// Check if we're in a Loquat project
pub fn is_loquat_project() -> bool {
    if let Ok(root) = get_project_root() {
        root.join("Cargo.toml").exists() && root.join("src").exists()
    } else {
        false
    }
}
