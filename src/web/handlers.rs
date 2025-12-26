//! API handlers for Web service

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::plugins::PluginManager;
use crate::adapters::AdapterManager;
use crate::config::loquat_config::LoquatConfig;

use super::types::*;
use super::traits::AppState;

/// Health check handler
pub async fn health_check(State(state): State<AppState>) -> Json<ApiResponse<HealthResponse>> {
    let uptime = state.start_time.elapsed().as_secs();
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: state.config.general.environment.clone(),
        uptime,
        plugins_enabled: state.config.plugins.enabled,
        adapters_enabled: state.config.adapters.enabled,
    };

    Json(ApiResponse::success(response))
}

/// Welcome page handler
pub async fn welcome(State(state): State<AppState>) -> Json<ApiResponse<WelcomeResponse>> {
    let response = WelcomeResponse {
        name: state.config.general.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: state.config.general.environment.clone(),
        message: "Welcome to Loquat Framework API".to_string(),
        endpoints: vec![
            "GET /health - Health check".to_string(),
            "GET /api/plugins - List all plugins".to_string(),
            "GET /api/plugins/{name} - Get plugin details".to_string(),
            "POST /api/plugins/reload - Reload plugins".to_string(),
            "GET /api/adapters - List all adapters".to_string(),
            "GET /api/adapters/{name} - Get adapter details".to_string(),
            "POST /api/adapters/reload - Reload adapters".to_string(),
            "POST /api/reload - Reload all".to_string(),
            "GET /api/config - Get configuration".to_string(),
        ],
    };

    Json(ApiResponse::success(response))
}

/// List all plugins
pub async fn list_plugins(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<ApiResponse<Vec<PluginInfo>>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        let plugins = plugin_manager.list_plugin_infos();
        
        let plugin_infos: Vec<PluginInfo> = plugins
            .into_iter()
            .map(|info| PluginInfo {
                name: info.metadata.name.clone(),
                plugin_type: info.metadata.plugin_type.to_string(),
                status: format!("{:?}", info.status),
                version: Some(info.metadata.version.clone()),
                author: info.metadata.author.clone(),
                description: info.metadata.description.clone(),
            })
            .filter(|p| {
                if let Some(name_filter) = &params.name {
                    p.name.contains(name_filter)
                } else {
                    true
                }
            })
            .collect();

        Json(ApiResponse::success(plugin_infos))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Get plugin details by name
pub async fn get_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginInfo>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if let Some(info) = plugin_manager.get_plugin_info(&name) {
            let plugin_info = PluginInfo {
                name: info.metadata.name.clone(),
                plugin_type: info.metadata.plugin_type.to_string(),
                status: format!("{:?}", info.status),
                version: Some(info.metadata.version.clone()),
                author: info.metadata.author.clone(),
                description: info.metadata.description.clone(),
            };
            Json(ApiResponse::success(plugin_info))
        } else {
            Json(ApiResponse::error(format!("Plugin '{}' not found", name)))
        }
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Reload plugins
pub async fn reload_plugins(
    State(state): State<AppState>,
    Json(request): Json<ReloadRequest>,
) -> Json<ApiResponse<ReloadResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        let mut reloaded_count = 0;
        
        // Reload specific plugins or all
        if request.plugins.unwrap_or(true) {
            match plugin_manager.auto_load_plugins().await {
                Ok(results) => {
                    reloaded_count = results.iter().filter(|r| r.success).count() as u32;
                    
                    let response = ReloadResponse {
                        message: format!("Reloaded {} plugins", reloaded_count),
                        plugins_reloaded: reloaded_count,
                        adapters_reloaded: 0,
                    };
                    Json(ApiResponse::success(response))
                }
                Err(e) => Json(ApiResponse::error(format!("Failed to reload plugins: {}", e))),
            }
        } else {
            let response = ReloadResponse {
                message: "No plugins reloaded".to_string(),
                plugins_reloaded: 0,
                adapters_reloaded: 0,
            };
            Json(ApiResponse::success(response))
        }
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// List all adapters
pub async fn list_adapters(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<ApiResponse<Vec<AdapterInfo>>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        let adapters = adapter_manager.list_adapter_infos().await;
        
        let adapter_infos: Vec<AdapterInfo> = adapters
            .into_iter()
            .map(|info| AdapterInfo {
                name: info.adapter_id.clone(),
                status: format!("{:?}", info.status),
                version: Some(info.version.clone()),
                description: None,
            })
            .filter(|a| {
                if let Some(name_filter) = &params.name {
                    a.name.contains(name_filter)
                } else {
                    true
                }
            })
            .collect();

        Json(ApiResponse::success(adapter_infos))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Get adapter details by name
pub async fn get_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterInfo>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if let Some(info) = adapter_manager.get_adapter_info(&name).await {
            let adapter_info = AdapterInfo {
                name: info.adapter_id.clone(),
                status: format!("{:?}", info.status),
                version: Some(info.version.clone()),
                description: None,
            };
            Json(ApiResponse::success(adapter_info))
        } else {
            Json(ApiResponse::error(format!("Adapter '{}' not found", name)))
        }
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Reload adapters
pub async fn reload_adapters(
    State(state): State<AppState>,
    Json(request): Json<ReloadRequest>,
) -> Json<ApiResponse<ReloadResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        let mut reloaded_count = 0;
        
        // Reload specific adapters or all
        if request.adapters.unwrap_or(true) {
            match adapter_manager.auto_load_adapters().await {
                Ok(results) => {
                    reloaded_count = results.iter().filter(|r| r.success).count() as u32;
                    
                    let response = ReloadResponse {
                        message: format!("Reloaded {} adapters", reloaded_count),
                        plugins_reloaded: 0,
                        adapters_reloaded: reloaded_count,
                    };
                    Json(ApiResponse::success(response))
                }
                Err(e) => Json(ApiResponse::error(format!("Failed to reload adapters: {}", e))),
            }
        } else {
            let response = ReloadResponse {
                message: "No adapters reloaded".to_string(),
                plugins_reloaded: 0,
                adapters_reloaded: 0,
            };
            Json(ApiResponse::success(response))
        }
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Reload all (plugins and adapters)
pub async fn reload_all(
    State(state): State<AppState>,
    Json(request): Json<ReloadRequest>,
) -> Json<ApiResponse<ReloadResponse>> {
    let mut plugins_reloaded = 0u32;
    let mut adapters_reloaded = 0u32;
    
    // Reload plugins
    if request.plugins.unwrap_or(true) {
        if let Some(plugin_manager) = &state.plugin_manager {
            if let Ok(results) = plugin_manager.auto_load_plugins().await {
                plugins_reloaded = results.iter().filter(|r| r.success).count() as u32;
            }
        }
    }
    
    // Reload adapters
    if request.adapters.unwrap_or(true) {
        if let Some(adapter_manager) = &state.adapter_manager {
            if let Ok(results) = adapter_manager.auto_load_adapters().await {
                adapters_reloaded = results.iter().filter(|r| r.success).count() as u32;
            }
        }
    }
    
    let response = ReloadResponse {
        message: format!(
            "Reloaded {} plugins and {} adapters",
            plugins_reloaded, adapters_reloaded
        ),
        plugins_reloaded,
        adapters_reloaded,
    };
    
    Json(ApiResponse::success(response))
}

/// Get configuration (sanitized)
pub async fn get_config(State(state): State<AppState>) -> Json<ApiResponse<ConfigResponse>> {
    let response = ConfigResponse {
        environment: state.config.general.environment.clone(),
        name: state.config.general.name.clone(),
        log_level: state.config.logging.level.clone(),
        log_format: state.config.logging.format.clone(),
        log_output: state.config.logging.output.clone(),
        plugins_enabled: state.config.plugins.enabled,
        adapters_enabled: state.config.adapters.enabled,
        web_enabled: state.config.web.enabled,
        web_host: state.config.web.host.clone(),
        web_port: state.config.web.port,
    };
    
    Json(ApiResponse::success(response))
}

/// Query parameters for list endpoints
#[derive(Debug, Deserialize)]
pub struct ListParams {
    /// Filter by name (contains)
    pub name: Option<String>,
}

/// Welcome response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WelcomeResponse {
    /// Service name
    pub name: String,
    /// Version
    pub version: String,
    /// Environment
    pub environment: String,
    /// Welcome message
    pub message: String,
    /// Available endpoints
    pub endpoints: Vec<String>,
}
