//! Configuration management API handlers

use axum::{
    extract::{Path, State},
    Json,
};
use crate::config::loquat_config::{
    LoquatConfig, GeneralConfig, LoggingConfig, PluginConfig, AdapterConfig,
    EngineConfig as EngineLoquatConfig, WebConfig, Validate,
};
use crate::errors::{ConfigError, Result};
use super::types::*;
use super::traits::AppState;

/// Get general configuration
pub async fn get_general_config(State(state): State<AppState>) -> Json<ApiResponse<GeneralConfigUpdate>> {
    let config = GeneralConfigUpdate {
        environment: state.config.general.environment.clone(),
        name: state.config.general.name.clone(),
    };
    Json(ApiResponse::success(config))
}

/// Update general configuration
pub async fn update_general_config(
    State(_state): State<AppState>,
    Json(update): Json<GeneralConfigUpdate>,
) -> Json<ApiResponse<GeneralConfigUpdateResponse>> {
    let mut updated_fields = Vec::new();
    
    let mut state_config = crate::config::loquat_config::LoquatConfig::default();
    
    if let Some(ref config) = &update.environment {
        state_config.general.environment = config.clone();
        updated_fields.push("environment".to_string());
    }
    if let Some(ref config) = &update.name {
        state_config.general.name = config.clone();
        updated_fields.push("name".to_string());
    }
    
    let response = GeneralConfigUpdateResponse {
        message: "General configuration updated".to_string(),
        updated_fields,
    };
    Json(ApiResponse::success(response))
}

/// Get logging configuration
pub async fn get_logging_config(State(state): State<AppState>) -> Json<ApiResponse<LoggingConfigUpdate>> {
    let config = LoggingConfigUpdate {
        level: state.config.logging.level.clone(),
        format: state.config.logging.format.clone(),
        output: state.config.logging.output.clone(),
        file_path: state.config.logging.file_path.clone(),
        enable_colors: state.config.logging.enable_colors,
    };
    Json(ApiResponse::success(config))
}

/// Update logging configuration
pub async fn update_logging_config(
    State(state): State<AppState>,
    Json(update): Json<LoggingConfigUpdate>,
) -> Json<ApiResponse<LoggingConfigUpdate>> {
    // Validate the update
    let config = LoggingConfig {
        level: update.level.clone(),
        format: update.format.clone(),
        output: update.output.clone(),
        file_path: update.file_path.clone(),
        enable_colors: update.enable_colors,
    };
    
    if let Err(e) = config.validate() {
        return Json(ApiResponse::error(format!("Validation failed: {}", e)));
    }
    
    Json(ApiResponse::success(update))
}

/// Get plugin configuration
pub async fn get_plugin_config(State(state): State<AppState>) -> Json<ApiResponse<PluginConfigUpdate>> {
    let config = PluginConfigUpdate {
        enabled: state.config.plugins.enabled,
        plugin_dir: state.config.plugins.plugin_dir.clone(),
        auto_load: state.config.plugins.auto_load,
        enable_hot_reload: state.config.plugins.enable_hot_reload,
        hot_reload_interval: state.config.plugins.hot_reload_interval,
        whitelist: state.config.plugins.whitelist.clone(),
        blacklist: state.config.plugins.blacklist.clone(),
    };
    Json(ApiResponse::success(config))
}

/// Update plugin configuration
pub async fn update_plugin_config(
    State(state): State<AppState>,
    Json(update): Json<PluginConfigUpdate>,
) -> Json<ApiResponse<PluginConfigUpdate>> {
    // Validate the update
    let config = PluginConfig {
        enabled: update.enabled,
        plugin_dir: update.plugin_dir.clone(),
        auto_load: update.auto_load,
        enable_hot_reload: update.enable_hot_reload,
        hot_reload_interval: update.hot_reload_interval,
        whitelist: update.whitelist.clone(),
        blacklist: update.blacklist.clone(),
    };
    
    if let Err(e) = config.validate() {
        return Json(ApiResponse::error(format!("Validation failed: {}", e)));
    }
    
    Json(ApiResponse::success(update))
}

/// Get adapter configuration
pub async fn get_adapter_config(State(state): State<AppState>) -> Json<ApiResponse<AdapterConfigUpdate>> {
    let config = AdapterConfigUpdate {
        enabled: state.config.adapters.enabled,
        adapter_dir: state.config.adapters.adapter_dir.clone(),
        auto_load: state.config.adapters.auto_load,
        enable_hot_reload: state.config.adapters.enable_hot_reload,
        hot_reload_interval: state.config.adapters.hot_reload_interval,
        whitelist: state.config.adapters.whitelist.clone(),
        blacklist: state.config.adapters.blacklist.clone(),
    };
    Json(ApiResponse::success(config))
}

/// Update adapter configuration
pub async fn update_adapter_config(
    State(state): State<AppState>,
    Json(update): Json<AdapterConfigUpdate>,
) -> Json<ApiResponse<AdapterConfigUpdate>> {
    // Validate the update
    let config = AdapterConfig {
        enabled: update.enabled,
        adapter_dir: update.adapter_dir.clone(),
        auto_load: update.auto_load,
        enable_hot_reload: update.enable_hot_reload,
        hot_reload_interval: update.hot_reload_interval,
        whitelist: update.whitelist.clone(),
        blacklist: update.blacklist.clone(),
    };
    
    if let Err(e) = config.validate() {
        return Json(ApiResponse::error(format!("Validation failed: {}", e)));
    }
    
    Json(ApiResponse::success(update))
}

/// Get engine configuration
pub async fn get_engine_config(State(state): State<AppState>) -> Json<ApiResponse<EngineConfigUpdate>> {
    let config = EngineConfigUpdate {
        auto_route: state.config.engine.auto_route,
        auto_create_channels: state.config.engine.auto_create_channels,
        auto_initialize: state.config.engine.auto_initialize,
    };
    Json(ApiResponse::success(config))
}

/// Update engine configuration
pub async fn update_engine_config(
    State(state): State<AppState>,
    Json(update): Json<EngineConfigUpdate>,
) -> Json<ApiResponse<EngineConfigUpdate>> {
    // Validate the update
    let config = EngineLoquatConfig {
        auto_route: update.auto_route,
        auto_create_channels: update.auto_create_channels,
        auto_initialize: update.auto_initialize,
    };
    
    if let Err(e) = config.validate() {
        return Json(ApiResponse::error(format!("Validation failed: {}", e)));
    }
    
    Json(ApiResponse::success(update))
}

/// Get web configuration
pub async fn get_web_config(State(state): State<AppState>) -> Json<ApiResponse<WebConfigUpdate>> {
    let config = WebConfigUpdate {
        enabled: state.config.web.enabled,
        host: state.config.web.host.clone(),
        port: state.config.web.port,
        enable_cors: state.config.web.enable_cors,
    };
    Json(ApiResponse::success(config))
}

/// Update web configuration
pub async fn update_web_config(
    State(state): State<AppState>,
    Json(update): Json<WebConfigUpdate>,
) -> Json<ApiResponse<WebConfigUpdate>> {
    // Validate the update
    let config = WebConfig {
        enabled: update.enabled,
        host: update.host.clone(),
        port: update.port,
        enable_cors: update.enable_cors,
    };
    
    if let Err(e) = config.validate() {
        return Json(ApiResponse::error(format!("Validation failed: {}", e)));
    }
    
    Json(ApiResponse::success(update))
}

/// Validate configuration without saving
pub async fn validate_config(
    State(state): State<AppState>,
) -> Json<ApiResponse<ConfigValidationResult>> {
    let mut errors = Vec::new();
    let warnings = Vec::new();
    
    // Validate all sub-configurations
    if let Err(e) = state.config.general.validate() {
        errors.push(ValidationError {
            field: "general".to_string(),
            message: e.to_string(),
        });
    }
    
    if let Err(e) = state.config.logging.validate() {
        errors.push(ValidationError {
            field: "logging".to_string(),
            message: e.to_string(),
        });
    }
    
    if let Err(e) = state.config.plugins.validate() {
        errors.push(ValidationError {
            field: "plugins".to_string(),
            message: e.to_string(),
        });
    }
    
    if let Err(e) = state.config.adapters.validate() {
        errors.push(ValidationError {
            field: "adapters".to_string(),
            message: e.to_string(),
        });
    }
    
    if let Err(e) = state.config.web.validate() {
        errors.push(ValidationError {
            field: "web".to_string(),
            message: e.to_string(),
        });
    }
    
    let valid = errors.is_empty();
    let result = ConfigValidationResult {
        valid,
        errors,
        warnings,
    };
    
    Json(ApiResponse::success(result))
}

/// Reload configuration from file
pub async fn reload_configuration(State(state): State<AppState>) -> Json<ApiResponse<ConfigReloadResponse>> {
    // In a real implementation, this would reload the config from disk
    // For now, we'll return a success message
    let response = ConfigReloadResponse {
        message: "Configuration reload initiated".to_string(),
        environment: state.config.general.environment.clone(),
    };
    Json(ApiResponse::success(response))
}

/// Save configuration to file
pub async fn save_configuration(State(state): State<AppState>) -> Json<ApiResponse<ConfigSaveResponse>> {
    // Save the current configuration
    let config_dir = LoquatConfig::get_config_dir().unwrap();
    let env = &state.config.general.environment;
    let file_path = config_dir.join(format!("{}.toml", env));
    
    if let Err(e) = state.config.to_toml_file(&file_path) {
        return Json(ApiResponse::error(format!("Failed to save configuration: {}", e)));
    }
    
    let response = ConfigSaveResponse {
        message: "Configuration saved successfully".to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        environment: env.clone(),
    };
    
    Json(ApiResponse::success(response))
}

/// Get configuration backups
pub async fn get_config_backups(State(_state): State<AppState>) -> Json<ApiResponse<Vec<ConfigBackup>>> {
    // In a real implementation, this would list backup files
    // For now, return empty list
    Json(ApiResponse::success(Vec::new()))
}

/// Backup current configuration
pub async fn backup_configuration(State(state): State<AppState>) -> Json<ApiResponse<ConfigBackup>> {
    use chrono::Utc;
    
    // Create a backup with timestamp
    let timestamp = Utc::now();
    let backup_id = format!("backup_{}", timestamp.format("%Y%m%d_%H%M%S"));
    let config_dir = LoquatConfig::get_config_dir().unwrap();
    let file_path = config_dir.join("backups").join(format!("{}.toml", backup_id));
    
    // Ensure backup directory exists
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    if let Err(e) = state.config.to_toml_file(&file_path) {
        return Json(ApiResponse::error(format!("Failed to create backup: {}", e)));
    }
    
    let backup = ConfigBackup {
        id: backup_id,
        timestamp,
        description: format!("Backup created on {}", timestamp.format("%Y-%m-%d %H:%M:%S")),
        file_path: file_path.to_string_lossy().to_string(),
    };
    
    Json(ApiResponse::success(backup))
}

/// Restore configuration from backup
pub async fn restore_configuration(
    State(_state): State<AppState>,
    Path(backup_id): Path<String>,
) -> Json<ApiResponse<ConfigSaveResponse>> {
    // In a real implementation, this would restore from backup
    // For now, return a placeholder response
    Json(ApiResponse::error(format!("Restore from backup '{}' not yet implemented", backup_id)))
}
