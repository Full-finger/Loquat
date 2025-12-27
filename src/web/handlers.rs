//! API handlers for Web service

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use crate::engine::traits::Engine;

use super::types::*;
use super::traits::AppState;

/// Health check handler
pub async fn health_check(State(state): State<AppState>) -> Json<ApiResponse<HealthResponse>> {
    let uptime = state.start_time.elapsed().as_secs();
    
    // Determine overall status based on subsystems
    let overall_status = determine_overall_status(&state).await;
    
    // Get engine status
    let engine_status = if let Some(engine) = &state.engine {
        format!("{:?}", engine.state())
    } else {
        "unknown".to_string()
    };
    
    // Get plugin subsystem status
    let plugin_status = if let Some(plugin_manager) = &state.plugin_manager {
        let plugins = plugin_manager.list_plugin_infos();
        let active = plugins.iter().filter(|p| p.status.is_active()).count();
        let inactive = plugins.iter().filter(|p| 
            matches!(p.status, 
                crate::plugins::types::PluginStatus::Unloaded |
                crate::plugins::types::PluginStatus::Disabled)
        ).count();
        let error_count = plugins.iter().filter(|p| matches!(p.status, crate::plugins::types::PluginStatus::Error { .. })).count();
        
        PluginSubsystemStatus {
            enabled: state.config.plugins.enabled,
            total: plugins.len(),
            active,
            inactive,
            error: error_count,
        }
    } else {
        PluginSubsystemStatus {
            enabled: false,
            total: 0,
            active: 0,
            inactive: 0,
            error: 0,
        }
    };
    
    // Get adapter subsystem status
    let adapter_status = if let Some(adapter_manager) = &state.adapter_manager {
        let adapters = adapter_manager.list_adapter_infos().await;
        let active = adapters.iter().filter(|a| a.status.is_active()).count();
        let inactive = adapters.iter().filter(|a| 
            matches!(a.status, 
                crate::adapters::status::AdapterStatus::Uninitialized | 
                crate::adapters::status::AdapterStatus::Initializing |
                crate::adapters::status::AdapterStatus::Stopped)
        ).count();
        let error_count = adapters.iter().filter(|a| a.status.is_error()).count();
        
        AdapterSubsystemStatus {
            enabled: state.config.adapters.enabled,
            total: adapters.len(),
            active,
            inactive,
            error: error_count,
        }
    } else {
        AdapterSubsystemStatus {
            enabled: false,
            total: 0,
            active: 0,
            inactive: 0,
            error: 0,
        }
    };
    
    // Get web subsystem status
    let is_web_running = state.web_running.load(std::sync::atomic::Ordering::SeqCst);
    let web_status = WebSubsystemStatus {
        enabled: state.config.web.enabled,
        running: is_web_running,
        host: state.config.web.host.clone(),
        port: state.config.web.port,
    };
    
    // Get logging subsystem status
    let logging_status = LoggingSubsystemStatus {
        level: state.config.logging.level.clone(),
        format: state.config.logging.format.clone(),
        output: state.config.logging.output.clone(),
    };
    
    // Get error statistics
    let error_stats = state.error_tracker.get_stats();
    
    let response = HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: state.config.general.environment.clone(),
        uptime,
        engine_status,
        subsystems: SubsystemStatus {
            plugins: plugin_status,
            adapters: adapter_status,
            web: web_status,
            logging: logging_status,
        },
        errors: error_stats,
    };

    Json(ApiResponse::success(response))
}

/// Determine overall system health status
async fn determine_overall_status(state: &AppState) -> String {
    let mut issues = 0;
    
    // Check engine
    if let Some(engine) = &state.engine {
        let engine_state = engine.state();
        if matches!(engine_state.status, crate::engine::types::EngineStatus::Error) {
            issues += 1;
        }
    }
    
    // Check plugins
    if let Some(plugin_manager) = &state.plugin_manager {
        let plugins = plugin_manager.list_plugin_infos();
        if plugins.iter().any(|p| matches!(p.status, crate::plugins::types::PluginStatus::Error { .. })) {
            issues += 1;
        }
    }
    
    // Check adapters
    if let Some(adapter_manager) = &state.adapter_manager {
        let adapters = adapter_manager.list_adapter_infos().await;
        if adapters.iter().any(|a| a.status.is_error()) {
            issues += 1;
        }
    }
    
    // Check critical errors
    let error_stats = state.error_tracker.get_stats();
    if error_stats.critical > 0 {
        return "critical".to_string();
    }
    
    match issues {
        0 => "healthy".to_string(),
        1..=2 => "degraded".to_string(),
        _ => "unhealthy".to_string(),
    }
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
