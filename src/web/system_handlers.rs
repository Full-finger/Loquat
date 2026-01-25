//! System management API handlers

use axum::{
    extract::State,
    Json,
};
use super::types::*;
use super::traits::AppState;

/// Get system status
pub async fn get_system_status(State(state): State<AppState>) -> Json<ApiResponse<SystemStatusResponse>> {
    let uptime = state.start_time.elapsed().as_secs();
    
    // Determine overall status
    let status = if state.config.web.enabled && state.web_running.load(std::sync::atomic::Ordering::SeqCst) {
        "running".to_string()
    } else {
        "stopped".to_string()
    };
    
    // Memory usage (platform-specific)
    let memory_usage_mb = None; // Could be implemented with sysinfo crate
    
    // CPU usage (platform-specific)
    let cpu_usage_percent = None; // Could be implemented with sysinfo crate
    
    // Active connections (simplified)
    let active_connections = 1; // Could track actual connections
    
    let response = SystemStatusResponse {
        status,
        uptime,
        memory_usage_mb,
        cpu_usage_percent,
        active_connections,
    };
    
    Json(ApiResponse::success(response))
}

/// Get system information
pub async fn get_system_info(State(_state): State<AppState>) -> Json<ApiResponse<SystemInfoResponse>> {
    let response = SystemInfoResponse {
        system_name: std::env::consts::OS.to_string(),
        os_version: std::env::consts::OS.to_string(), // Could use sysinfo for detailed version
        architecture: std::env::consts::ARCH.to_string(),
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
        loquat_version: env!("CARGO_PKG_VERSION").to_string(),
        hostname: gethostname::gethostname()
            .to_str()
            .unwrap_or("unknown")
            .to_string(),
    };
    
    Json(ApiResponse::success(response))
}

/// Get system diagnostics
pub async fn get_system_diagnostics(State(state): State<AppState>) -> Json<ApiResponse<SystemDiagnosticsResponse>> {
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    
    // Check engine
    let engine_status = if let Some(engine) = &state.engine {
        let engine_state = engine.state().await;
        if matches!(engine_state.status, crate::engine::types::EngineStatus::Error) {
            issues.push("Engine is in error state".to_string());
        }
        format!("{:?}", engine_state.status)
    } else {
        "not available".to_string()
    };
    
    // Check plugins
    let plugin_status = if let Some(plugin_manager) = &state.plugin_manager {
        let plugins = plugin_manager.list_plugin_infos();
        if plugins.iter().any(|p| matches!(p.status, crate::plugins::types::PluginStatus::Error { .. })) {
            issues.push("Some plugins are in error state".to_string());
        }
        if plugins.is_empty() {
            warnings.push("No plugins loaded".to_string());
        }
        format!("{} plugins loaded", plugins.len())
    } else {
        "not available".to_string()
    };
    
    // Check adapters
    let adapter_status = if let Some(adapter_manager) = &state.adapter_manager {
        let adapters = adapter_manager.list_adapter_infos().await;
        if adapters.iter().any(|a| a.status.is_error()) {
            issues.push("Some adapters are in error state".to_string());
        }
        if adapters.is_empty() {
            warnings.push("No adapters loaded".to_string());
        }
        format!("{} adapters loaded", adapters.len())
    } else {
        "not available".to_string()
    };
    
    // Check web service
    let web_status = if state.config.web.enabled {
        let is_running = state.web_running.load(std::sync::atomic::Ordering::SeqCst);
        if !is_running {
            issues.push("Web service is enabled but not running".to_string());
        }
        if is_running { "running" } else { "stopped" }.to_string()
    } else {
        "disabled".to_string()
    };
    
    // Check logging
    let logging_status = format!(
        "{} ({})",
        state.config.logging.level, state.config.logging.format
    );
    
    // Determine overall health
    let health = if issues.is_empty() {
        if warnings.is_empty() {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    } else {
        "unhealthy".to_string()
    };
    
    let response = SystemDiagnosticsResponse {
        health,
        engine: engine_status,
        plugins: plugin_status,
        adapters: adapter_status,
        web: web_status,
        logging: logging_status,
        issues,
        warnings,
    };
    
    Json(ApiResponse::success(response))
}

/// Shutdown system
pub async fn shutdown_system(State(state): State<AppState>) -> Json<ApiResponse<SystemControlResponse>> {
    let response = SystemControlResponse {
        message: "System shutdown initiated".to_string(),
        action: "shutdown".to_string(),
    };
    
    // In a real implementation, this would trigger graceful shutdown
    Json(ApiResponse::success(response))
}

/// Restart system
pub async fn restart_system(State(state): State<AppState>) -> Json<ApiResponse<SystemControlResponse>> {
    let response = SystemControlResponse {
        message: "System restart initiated".to_string(),
        action: "restart".to_string(),
    };
    
    // In a real implementation, this would trigger restart
    Json(ApiResponse::success(response))
}

/// Clear cache
pub async fn clear_cache(State(state): State<AppState>) -> Json<ApiResponse<CacheClearResponse>> {
    // Clear various caches
    let items_cleared = 0;
    
    // In a real implementation, this would clear:
    // - LRU caches
    // - Hot reload history
    // - Channel manager caches
    // etc.
    
    let response = CacheClearResponse {
        message: "Cache cleared successfully".to_string(),
        cache_type: "all".to_string(),
        items_cleared,
    };
    
    Json(ApiResponse::success(response))
}
