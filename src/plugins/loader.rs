//! Plugin loader for loading different plugin types

use crate::errors::{PluginError, Result};
use crate::plugins::types::PluginType;
use crate::plugins::traits::Plugin;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// Plugin loader trait for loading different plugin types
#[async_trait]
pub trait PluginLoader: Send + Sync {
    /// Load a plugin from a file path
    async fn load_plugin(&self, path: &PathBuf) -> Result<Arc<dyn Plugin>>;

    /// Check if this loader can handle the given plugin type
    fn can_handle(&self, plugin_type: &PluginType) -> bool;

    /// Get the plugin type this loader handles
    fn plugin_type(&self) -> PluginType;
}

/// Native Rust plugin loader (dylib)
pub struct NativePluginLoader;

#[async_trait]
impl PluginLoader for NativePluginLoader {
    async fn load_plugin(&self, path: &PathBuf) -> Result<Arc<dyn Plugin>> {
        if !path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "Plugin file not found: {:?}",
                path
            ))
            .into());
        }

        // Check file extension
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::LoadFailed("Invalid plugin file".to_string()))?;

        if ext != "dll" && ext != "so" && ext != "dylib" {
            return Err(PluginError::UnsupportedType(format!(
                "Invalid native plugin extension: {}",
                ext
            ))
            .into());
        }

        // In a real implementation, this would:
        // 1. Load the dynamic library
        // 2. Find the plugin constructor/export
        // 3. Create an instance of the plugin
        // For now, we'll return a placeholder
        Err(PluginError::LoadFailed(
            "Native plugin loading not yet implemented - requires dynamic library infrastructure"
                .to_string(),
        )
        .into())
    }

    fn can_handle(&self, plugin_type: &PluginType) -> bool {
        matches!(plugin_type, PluginType::Native)
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Native
    }
}

/// Python plugin loader
pub struct PythonPluginLoader {
    python_path: Option<PathBuf>,
}

impl PythonPluginLoader {
    /// Create a new Python plugin loader
    pub fn new() -> Self {
        Self { python_path: None }
    }

    /// Set the Python interpreter path
    pub fn with_python_path(mut self, path: PathBuf) -> Self {
        self.python_path = Some(path);
        self
    }
}

impl Default for PythonPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginLoader for PythonPluginLoader {
    async fn load_plugin(&self, path: &PathBuf) -> Result<Arc<dyn Plugin>> {
        if !path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "Plugin file not found: {:?}",
                path
            ))
            .into());
        }

        // Check file extension
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::LoadFailed("Invalid plugin file".to_string()))?;

        if ext != "py" {
            return Err(PluginError::UnsupportedType(format!(
                "Invalid Python plugin extension: {}",
                ext
            ))
            .into());
        }

        // In a real implementation, this would:
        // 1. Use pyo3 to embed Python
        // 2. Load and execute the Python module
        // 3. Create a wrapper that implements the Plugin trait
        Err(PluginError::LoadFailed(
            "Python plugin loading not yet implemented - requires pyo3 infrastructure".to_string(),
        )
        .into())
    }

    fn can_handle(&self, plugin_type: &PluginType) -> bool {
        matches!(plugin_type, PluginType::Python)
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Python
    }
}

/// JavaScript plugin loader
pub struct JavaScriptPluginLoader {
    node_path: Option<PathBuf>,
}

impl JavaScriptPluginLoader {
    /// Create a new JavaScript plugin loader
    pub fn new() -> Self {
        Self { node_path: None }
    }

    /// Set the Node.js interpreter path
    pub fn with_node_path(mut self, path: PathBuf) -> Self {
        self.node_path = Some(path);
        self
    }
}

impl Default for JavaScriptPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginLoader for JavaScriptPluginLoader {
    async fn load_plugin(&self, path: &PathBuf) -> Result<Arc<dyn Plugin>> {
        if !path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "Plugin file not found: {:?}",
                path
            ))
            .into());
        }

        // Check file extension
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::LoadFailed("Invalid plugin file".to_string()))?;

        if ext != "js" && ext != "mjs" && ext != "ts" {
            return Err(PluginError::UnsupportedType(format!(
                "Invalid JavaScript plugin extension: {}",
                ext
            ))
            .into());
        }

        // In a real implementation, this would:
        // 1. Use a JS runtime (deno_core, quickjs, etc.)
        // 2. Load and execute the JS module
        // 3. Create a wrapper that implements the Plugin trait
        Err(PluginError::LoadFailed(
            "JavaScript plugin loading not yet implemented - requires JS runtime infrastructure"
                .to_string(),
        )
        .into())
    }

    fn can_handle(&self, plugin_type: &PluginType) -> bool {
        matches!(plugin_type, PluginType::JavaScript)
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::JavaScript
    }
}

/// Composite plugin loader that delegates to specific loaders
pub struct CompositePluginLoader {
    loaders: Vec<Box<dyn PluginLoader>>,
}

impl CompositePluginLoader {
    /// Create a new composite loader with default loaders
    pub fn new() -> Self {
        Self {
            loaders: vec![
                Box::new(NativePluginLoader),
                Box::new(PythonPluginLoader::default()),
                Box::new(JavaScriptPluginLoader::default()),
            ],
        }
    }

    /// Add a custom plugin loader
    pub fn add_loader(mut self, loader: Box<dyn PluginLoader>) -> Self {
        self.loaders.push(loader);
        self
    }

    /// Get a loader that can handle the given plugin type
    pub fn get_loader_for_type(&self, plugin_type: &PluginType) -> Option<&dyn PluginLoader> {
        self.loaders
            .iter()
            .find(|l| l.can_handle(plugin_type))
            .map(|l| l.as_ref())
    }
}

impl Default for CompositePluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginLoader for CompositePluginLoader {
    async fn load_plugin(&self, path: &PathBuf) -> Result<Arc<dyn Plugin>> {
        // Try to determine plugin type from metadata
        // For now, we'll use file extension
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::LoadFailed("Invalid plugin file".to_string()))?;

        let plugin_type = match ext {
            "dll" | "so" | "dylib" => PluginType::Native,
            "py" => PluginType::Python,
            "js" | "mjs" | "ts" => PluginType::JavaScript,
            _ => {
                return Err(PluginError::UnsupportedType(format!(
                    "Unknown plugin extension: {}",
                    ext
                ))
                .into())
            }
        };

        if let Some(loader) = self.get_loader_for_type(&plugin_type) {
            loader.load_plugin(path).await
        } else {
            Err(PluginError::UnsupportedType(format!(
                "No loader available for plugin type: {:?}",
                plugin_type
            ))
            .into())
        }
    }

    fn can_handle(&self, plugin_type: &PluginType) -> bool {
        self.get_loader_for_type(plugin_type).is_some()
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Native // Composite can handle all types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_loader_handles_native_type() {
        let loader = NativePluginLoader;
        assert!(loader.can_handle(&PluginType::Native));
        assert!(!loader.can_handle(&PluginType::Python));
        assert!(!loader.can_handle(&PluginType::JavaScript));
    }

    #[test]
    fn test_python_loader_handles_python_type() {
        let loader = PythonPluginLoader::default();
        assert!(!loader.can_handle(&PluginType::Native));
        assert!(loader.can_handle(&PluginType::Python));
        assert!(!loader.can_handle(&PluginType::JavaScript));
    }

    #[test]
    fn test_js_loader_handles_js_type() {
        let loader = JavaScriptPluginLoader::default();
        assert!(!loader.can_handle(&PluginType::Native));
        assert!(!loader.can_handle(&PluginType::Python));
        assert!(loader.can_handle(&PluginType::JavaScript));
    }

    #[test]
    fn test_composite_loader_handles_all_types() {
        let loader = CompositePluginLoader::new();
        assert!(loader.can_handle(&PluginType::Native));
        assert!(loader.can_handle(&PluginType::Python));
        assert!(loader.can_handle(&PluginType::JavaScript));
    }
}
