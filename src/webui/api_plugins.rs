//! 管理面的 API handlers：适配器、Provider（服务配置）、插件列表/启停/
//! 安装、插件配置与资源。路由登记在 api.rs 的 routes()。

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::api::{asset_content_type, blocking, join_lost, query_param, token_query_ok};
use super::{authorized, unauthorized, WebState};

/// 适配器配置 + 运行状态。
pub(super) async fn api_adapters(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
        let raw = state.dll.adapters_config();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        Json(v).into_response()
    })
}

/// 应用适配器配置（写盘 + 热应用）。body 即 config 载荷。
pub(super) async fn api_adapters_apply(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match state.dll.apply_adapters_config(&body.to_string()) {
            Ok(s) => {
                let v: serde_json::Value =
                    serde_json::from_str(&s).unwrap_or(serde_json::Value::Null);
                Json(v).into_response()
            }
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
        }
    )
}

/// 服务配置（provider 层）：api_key 打码，has_api_key 标记是否已配置。
pub(super) async fn api_providers(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    Json(crate::providers::view()).into_response()
}

/// 保存服务配置（api_key 空 = 保持已有，null = 清除），热生效。
pub(super) async fn api_providers_save(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match crate::providers::save(&body) {
            Ok(()) => {
                state.business.refresh_providers();
                Json(serde_json::json!({ "ok": true, "msg": "已保存，服务配置热生效" }))
                    .into_response()
            }
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

/// 插件配置读取（SCHEMA + 当前值）。
pub(super) async fn api_plugin_config(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match state.business.plugin_config(&name) {
            Ok(v) => Json(v).into_response(),
            Err(e) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

/// 插件配置保存（cfglib 校验渲染，热重载即时生效）。
pub(super) async fn api_plugin_config_save(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match state.business.save_plugin_config(&name, body) {
            Ok(()) =>
                Json(serde_json::json!({ "ok": true, "msg": "已保存并热重载" })).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

/// 插件资源清单（asset 配置字段的候选文件）。
pub(super) async fn api_plugin_assets(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match state.business.list_plugin_assets(&name) {
            Ok(v) => Json(serde_json::json!({ "assets": v })).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

/// 上传资源：body 即文件原始字节，文件名走 ?name=（免 multipart 依赖）。
pub(super) async fn api_plugin_asset_upload(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let filename = query_param(query.as_deref(), "name").unwrap_or_default();
    blocking!(
        state,
        match state.business.save_plugin_asset(&name, &filename, &body) {
            Ok(stored) => Json(serde_json::json!({ "name": stored })).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

/// 资源预览/下载：<img> 带不了 Authorization 头，token 查询参数兜底
/// （同 ui.js / /api/ws）。路径解析放阻塞池，文件读取走异步。
pub(super) async fn api_plugin_asset_view(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    Path(name): Path<String>,
) -> Response {
    if !token_query_ok(&state, query.as_deref()) && !authorized(&state, &headers) {
        return unauthorized();
    }
    let rel = query_param(query.as_deref(), "path").unwrap_or_default();
    let path = tokio::task::spawn_blocking(move || state.business.plugin_asset_path(&name, &rel))
        .await
        .unwrap_or(None);
    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            [(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static(asset_content_type(&path.to_string_lossy())),
            )],
            bytes,
        )
            .into_response(),
        // 读完文件消失（刚被删除）与不存在同样对待
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub(super) async fn api_plugin_asset_delete(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let rel = query_param(query.as_deref(), "path").unwrap_or_default();
    blocking!(
        state,
        match state.business.delete_plugin_asset(&name, &rel) {
            Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}

pub(super) async fn api_plugins(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
        let rows: Vec<serde_json::Value> = state
            .business
            .plugin_rows()
            .into_iter()
            .map(|(name, installed, enabled, priority, has_ui)| {
                serde_json::json!({
                    "name": name,
                    "installed": installed,
                    "enabled": enabled,
                    "priority": priority,
                    "has_ui": has_ui,
                })
            })
            .collect();
        Json(serde_json::Value::Array(rows)).into_response()
    })
}

// 启停与安装：set_enabled 返回是否找到插件，install 返回结果文本。
pub(super) async fn api_enable(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        if state.business.set_enabled(&name, true) {
            (
                StatusCode::OK,
                Json(serde_json::json!({ "msg": format!("{name} 已启用") })),
            )
                .into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "插件不存在" })),
            )
                .into_response()
        }
    )
}

pub(super) async fn api_disable(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        if state.business.set_enabled(&name, false) {
            (
                StatusCode::OK,
                Json(
                    serde_json::json!({ "msg": format!("{name} 已禁用（重启后回到文件内声明）") }),
                ),
            )
                .into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "插件不存在" })),
            )
                .into_response()
        }
    )
}

pub(super) async fn api_install(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(
        state,
        match state.business.install(&name) {
            Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "msg": msg }))).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e }))
            )
                .into_response(),
        }
    )
}
