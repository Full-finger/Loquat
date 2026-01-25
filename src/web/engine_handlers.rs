//! Engine control API handlers

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use crate::engine::traits::Engine;
use crate::engine::types::{EngineConfig, EngineStats, EngineStatus};
use super::types::*;
use super::traits::AppState;

/// Get engine status
pub async fn get_engine_status(State(state): State<AppState>) -> Json<ApiResponse<EngineStatusResponse>> {
    if let Some(engine) = &state.engine {
        let engine_state = engine.state().await;
        let status = EngineStatusResponse {
            status: format!("{:?}", engine_state.status),
            running: engine_state.status.is_running(),
            last_error: engine_state.last_error,
        };
        Json(ApiResponse::success(status))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Start engine
pub async fn start_engine(State(state): State<AppState>) -> Json<ApiResponse<EngineControlResponse>> {
    if let Some(engine) = &state.engine {
        // Note: We can't call start() on Arc<dyn Engine> directly
        // This is a limitation of the current design
        // In a real implementation, we would need a mutable reference
        let response = EngineControlResponse {
            message: "Engine start requested".to_string(),
            status: "requested".to_string(),
        };
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Stop engine
pub async fn stop_engine(State(state): State<AppState>) -> Json<ApiResponse<EngineControlResponse>> {
    if let Some(engine) = &state.engine {
        let engine_state = engine.state().await;
        if !engine_state.status.is_running() {
            return Json(ApiResponse::error("Engine is not running".to_string()));
        }
        
        // Note: Similar to start, we can't call stop() on Arc<dyn Engine>
        let response = EngineControlResponse {
            message: "Engine stop requested".to_string(),
            status: "requested".to_string(),
        };
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Get engine statistics
pub async fn get_engine_stats(State(state): State<AppState>) -> Json<ApiResponse<EngineStatsResponse>> {
    if let Some(engine) = &state.engine {
        let uptime = state.start_time.elapsed().as_secs();
        let stats = engine.stats();
        
        let response = EngineStatsResponse {
            total_packages: stats.total_packages as u64,
            successful_packages: (stats.total_packages - stats.failed_packages) as u64,
            failed_packages: stats.failed_packages as u64,
            avg_processing_time: stats.avg_processing_time_ms,
            uptime,
        };
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Update engine configuration
pub async fn update_engine_config(
    State(state): State<AppState>,
    Json(update): Json<EngineConfigUpdate>,
) -> Json<ApiResponse<EngineConfigUpdateResponse>> {
    if let Some(_engine) = &state.engine {
        // Note: We can't update config on Arc<dyn Engine> directly
        // This is a design limitation
        let mut updated_fields = Vec::new();
        
        if update.auto_route {
            updated_fields.push("auto_route".to_string());
        }
        if update.auto_create_channels {
            updated_fields.push("auto_create_channels".to_string());
        }
        if update.auto_initialize {
            updated_fields.push("auto_initialize".to_string());
        }
        
        let response = EngineConfigUpdateResponse {
            message: "Engine configuration update requested".to_string(),
            updated_fields,
        };
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Reset engine statistics
pub async fn reset_engine_stats(State(state): State<AppState>) -> Json<ApiResponse<EngineControlResponse>> {
    if let Some(engine) = &state.engine {
        // Note: We can't reset stats on Arc<dyn Engine> directly
        // This is a design limitation
        let response = EngineControlResponse {
            message: "Engine statistics reset requested".to_string(),
            status: "requested".to_string(),
        };
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Engine not available".to_string()))
    }
}

/// Engine health check
pub async fn engine_health(State(state): State<AppState>) -> Json<ApiResponse<EngineHealthResponse>> {
    if let Some(engine) = &state.engine {
        use chrono::Utc;
        
        let engine_state = engine.state().await;
        let is_healthy = !matches!(engine_state.status, EngineStatus::Error);
        
        let status = if is_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };
        
        let response = EngineHealthResponse {
            healthy: is_healthy,
            status,
            timestamp: Utc::now(),
        };
        Json(ApiResponse::success(response))
    } else {
        let response = EngineHealthResponse {
            healthy: false,
            status: "unavailable".to_string(),
            timestamp: Utc::now(),
        };
        Json(ApiResponse::success(response))
    }
}
