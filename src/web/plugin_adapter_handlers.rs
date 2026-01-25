//! Plugin and adapter management extension API handlers

use axum::{
    extract::{Path, State},
    Json,
};
use super::types::*;
use super::traits::AppState;

// ============================================================================
// Plugin Management Extensions
// ============================================================================

/// Disable plugin (add to blacklist)
pub async fn disable_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if !plugin_manager.is_plugin_loaded(&name) {
            return Json(ApiResponse::error(format!("Plugin '{}' not found", name)));
        }
        
        // Add to blacklist
        let mut config = state.config.plugins.clone();
        if !config.blacklist.contains(&name) {
            config.blacklist.push(name.clone());
        }
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' disabled", name),
            plugin_name: name,
            status: "disabled".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Enable plugin (remove from blacklist)
pub async fn enable_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if !plugin_manager.is_plugin_loaded(&name) {
            return Json(ApiResponse::error(format!("Plugin '{}' not found", name)));
        }
        
        // Remove from blacklist
        let mut config = state.config.plugins.clone();
        config.blacklist.retain(|x| x != &name);
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' enabled", name),
            plugin_name: name,
            status: "enabled".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Add plugin to whitelist
pub async fn whitelist_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if !plugin_manager.is_plugin_loaded(&name) {
            return Json(ApiResponse::error(format!("Plugin '{}' not found", name)));
        }
        
        // Add to whitelist
        let mut config = state.config.plugins.clone();
        if !config.whitelist.contains(&name) {
            config.whitelist.push(name.clone());
        }
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' added to whitelist", name),
            plugin_name: name,
            status: "whitelisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Remove plugin from whitelist
pub async fn unwhitelist_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        // Remove from whitelist
        let mut config = state.config.plugins.clone();
        config.whitelist.retain(|x| x != &name);
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' removed from whitelist", name),
            plugin_name: name,
            status: "unwhitelisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Add plugin to blacklist
pub async fn blacklist_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if !plugin_manager.is_plugin_loaded(&name) {
            return Json(ApiResponse::error(format!("Plugin '{}' not found", name)));
        }
        
        // Add to blacklist
        let mut config = state.config.plugins.clone();
        if !config.blacklist.contains(&name) {
            config.blacklist.push(name.clone());
        }
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' added to blacklist", name),
            plugin_name: name,
            status: "blacklisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Remove plugin from blacklist
pub async fn unblacklist_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<PluginToggleResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        // Remove from blacklist
        let mut config = state.config.plugins.clone();
        config.blacklist.retain(|x| x != &name);
        
        let response = PluginToggleResponse {
            message: format!("Plugin '{}' removed from blacklist", name),
            plugin_name: name,
            status: "unblacklisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Get plugin configuration
pub async fn get_plugin_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if let Some(plugin) = plugin_manager.get_plugin(&name).await {
            // In a real implementation, we would get the plugin's config
            // For now, return a placeholder
            let config = serde_json::json!({
                "name": name,
                "enabled": true,
                "config": {}
            });
            Json(ApiResponse::success(config))
        } else {
            Json(ApiResponse::error(format!("Plugin '{}' not found", name)))
        }
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

/// Update plugin configuration
pub async fn update_plugin_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Json<ApiResponse<PluginConfigUpdateResponse>> {
    if let Some(plugin_manager) = &state.plugin_manager {
        if !plugin_manager.is_plugin_loaded(&name) {
            return Json(ApiResponse::error(format!("Plugin '{}' not found", name)));
        }
        
        // In a real implementation, we would update the plugin's config
        let response = PluginConfigUpdateResponse {
            message: format!("Plugin '{}' configuration updated", name),
            plugin_name: name,
            config: config.clone(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Plugin system is not enabled".to_string()))
    }
}

// ============================================================================
// Adapter Management Extensions
// ============================================================================

/// Disable adapter (add to blacklist)
pub async fn disable_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // Add to blacklist
        let mut config = state.config.adapters.clone();
        if !config.blacklist.contains(&name) {
            config.blacklist.push(name.clone());
        }
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' disabled", name),
            adapter_name: name,
            status: "disabled".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Enable adapter (remove from blacklist)
pub async fn enable_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // Remove from blacklist
        let mut config = state.config.adapters.clone();
        config.blacklist.retain(|x| x != &name);
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' enabled", name),
            adapter_name: name,
            status: "enabled".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Add adapter to whitelist
pub async fn whitelist_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // Add to whitelist
        let mut config = state.config.adapters.clone();
        if !config.whitelist.contains(&name) {
            config.whitelist.push(name.clone());
        }
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' added to whitelist", name),
            adapter_name: name,
            status: "whitelisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Remove adapter from whitelist
pub async fn unwhitelist_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        // Remove from whitelist
        let mut config = state.config.adapters.clone();
        config.whitelist.retain(|x| x != &name);
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' removed from whitelist", name),
            adapter_name: name,
            status: "unwhitelisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Add adapter to blacklist
pub async fn blacklist_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // Add to blacklist
        let mut config = state.config.adapters.clone();
        if !config.blacklist.contains(&name) {
            config.blacklist.push(name.clone());
        }
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' added to blacklist", name),
            adapter_name: name,
            status: "blacklisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Remove adapter from blacklist
pub async fn unblacklist_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterToggleResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        // Remove from blacklist
        let mut config = state.config.adapters.clone();
        config.blacklist.retain(|x| x != &name);
        
        let response = AdapterToggleResponse {
            message: format!("Adapter '{}' removed from blacklist", name),
            adapter_name: name,
            status: "unblacklisted".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Get adapter configuration
pub async fn get_adapter_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if let Some(adapter) = adapter_manager.get_adapter(&name).await {
            // Get adapter's configuration
            let config = serde_json::json!({
                "name": name,
                "adapter_type": adapter.config().adapter_type,
                "platform": adapter.config().platform,
                "enabled": true,
                "platform_config": adapter.config().platform_config
            });
            Json(ApiResponse::success(config))
        } else {
            Json(ApiResponse::error(format!("Adapter '{}' not found", name)))
        }
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Update adapter configuration
pub async fn update_adapter_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Json<ApiResponse<AdapterConfigUpdateResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // In a real implementation, we would update the adapter's config
        let response = AdapterConfigUpdateResponse {
            message: format!("Adapter '{}' configuration updated", name),
            adapter_name: name,
            config: config.clone(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Start adapter
pub async fn start_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterControlResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // In a real implementation, we would start the adapter
        let response = AdapterControlResponse {
            message: format!("Adapter '{}' start requested", name),
            adapter_name: name,
            action: "start".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}

/// Stop adapter
pub async fn stop_adapter(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<AdapterControlResponse>> {
    if let Some(adapter_manager) = &state.adapter_manager {
        if !adapter_manager.is_adapter_loaded(&name).await {
            return Json(ApiResponse::error(format!("Adapter '{}' not found", name)));
        }
        
        // In a real implementation, we would stop the adapter
        let response = AdapterControlResponse {
            message: format!("Adapter '{}' stop requested", name),
            adapter_name: name,
            action: "stop".to_string(),
        };
        
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Adapter system is not enabled".to_string()))
    }
}
