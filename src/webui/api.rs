//! WebUI 的 REST 与 WS handlers。路由表在 mod.rs 的 serve()；
//! 鉴权（authorized）与状态（WebState/STREAM_TX）由 mod.rs 持有。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use super::{authorized, unauthorized, STREAM_TX, WebState};
use crate::stats;

/// 全部 API 路由（宿主能力面）。新增端点在此登记。
pub fn routes() -> Router<Arc<WebState>> {
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/extensions/ui", get(api_extensions_ui))
        .route("/api/sandbox/send", post(api_sandbox_send))
        .route("/api/adapters", get(api_adapters))
        .route("/api/adapters/apply", post(api_adapters_apply))
        .route("/api/providers", get(api_providers).post(api_providers_save))
        .route("/api/plugins/{name}/config", get(api_plugin_config).put(api_plugin_config_save))
        .route("/api/plugins", get(api_plugins))
        .route("/api/plugins/{name}/enable", post(api_enable))
        .route("/api/plugins/{name}/disable", post(api_disable))
        .route("/api/plugins/{name}/install", post(api_install))
        .route("/api/core/reload-napcat", post(api_reload_napcat))
        .route("/api/ws", get(api_ws))
}

async fn api_status(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let raw = state.dll.status();
    let core: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    // 多适配器：任意平台存活即视为运行中
    let alive = core
        .get("adapters")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().any(|x| x.get("alive").and_then(|b| b.as_bool()).unwrap_or(false)))
        .unwrap_or(false);
    Json(serde_json::json!({
        "core": if alive { "运行中" } else { "未就绪" },
        "core_detail": core,
        "events": stats::event_count(),
        "replies": stats::reply_count(),
        "workers": stats::worker_backlogs(),
        "business": state.business.info(),
    }))
    .into_response()
}

/// 适配器配置 + 运行状态。
async fn api_adapters(State(state): State<Arc<WebState>>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let raw = state.dll.adapters_config();
    let v: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    Json(v).into_response()
}

/// 应用适配器配置（写盘 + 热应用）。body 即 config 载荷。
async fn api_adapters_apply(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match state.dll.apply_adapters_config(&body.to_string()) {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::Value::Null);
            Json(v).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// 服务配置（provider 层）：api_key 打码，has_api_key 标记是否已配置。
async fn api_providers(State(state): State<Arc<WebState>>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    Json(crate::providers::view()).into_response()
}

/// 保存服务配置（api_key 空 = 保持已有，null = 清除），热生效。
async fn api_providers_save(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match crate::providers::save(&body) {
        Ok(()) => {
            state.business.refresh_providers();
            Json(serde_json::json!({ "ok": true, "msg": "已保存，服务配置热生效" }))
                .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

/// 插件配置读取（SCHEMA + 当前值）。
async fn api_plugin_config(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match state.business.plugin_config(&name) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

/// 插件配置保存（cfglib 校验渲染，热重载即时生效）。
async fn api_plugin_config_save(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match state.business.save_plugin_config(&name, body) {
        Ok(()) => Json(serde_json::json!({ "ok": true, "msg": "已保存并热重载" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

/// 导航 tab 列表（通用扩展机制的宿主侧声明）。
/// 虽只是导航元数据，也与其他端点统一鉴权（避免「免 token 探针」）。
async fn api_extensions_ui(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let tabs = super::ui_tabs_snapshot();
    Json(serde_json::json!({ "tabs": tabs })).into_response()
}

/// 沙箱选项卡的输入通路：注入一条消息进沙箱适配器的事件链路。
/// body: {"text": "...", "mode": "private"|"group", "atBot": bool}
async fn api_sandbox_send(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let text = body.get("text").and_then(|t| t.as_str()).unwrap_or("").trim();
    let mode = body.get("mode").and_then(|m| m.as_str()).unwrap_or("private");
    let at_bot = body.get("atBot").and_then(|b| b.as_bool()).unwrap_or(false);
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "text 为空" }))).into_response();
    }
    if mode != "private" && mode != "group" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "mode 仅支持 private/group" }))).into_response();
    }
    match state.dll.inject_event("sandbox", text, at_bot, mode) {
        Ok(resp) => Json(serde_json::json!({ "ok": true, "detail": resp })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn api_plugins(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let rows: Vec<serde_json::Value> = state
        .business
        .plugin_rows()
        .into_iter()
        .map(|(name, installed, enabled, priority)| {
            serde_json::json!({
                "name": name,
                "installed": installed,
                "enabled": enabled,
                "priority": priority,
            })
        })
        .collect();
    Json(serde_json::Value::Array(rows)).into_response()
}

// 启停与安装：set_enabled 返回是否找到插件，install 返回结果文本。
async fn api_enable(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    if state.business.set_enabled(&name, true) {
        (StatusCode::OK, Json(serde_json::json!({ "msg": format!("{name} 已启用") }))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "插件不存在" }))).into_response()
    }
}

async fn api_disable(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    if state.business.set_enabled(&name, false) {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "msg": format!("{name} 已禁用（重启后回到文件内声明）") })),
        )
            .into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "插件不存在" }))).into_response()
    }
}

async fn api_install(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match state.business.install(&name) {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "msg": msg }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn api_reload_napcat(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let msg = if state.dll.reload_plugin("napcat") == 0 {
        "已触发，核心将退避重启适配器".to_string()
    } else {
        "失败（核心未启动）".to_string()
    };
    (StatusCode::OK, Json(serde_json::json!({ "msg": msg }))).into_response()
}

async fn api_ws(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    upgrade: WebSocketUpgrade,
) -> Response {
    // 浏览器 WebSocket API 不能带 Authorization 头，token 允许走 ?token=
    // 查询参数兜底（会进访问日志；本服务只绑回环，日志仅落本机）。
    let query_ok = match (&state.token, query.as_deref()) {
        (Some(expected), Some(q)) => q.split('&').any(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            k == "token" && v == expected
        }),
        (Some(_), None) => false,
        (None, _) => true,
    };
    if !query_ok && !authorized(&state, &headers) {
        return unauthorized();
    }
    let rx = STREAM_TX.get().map(|tx| tx.subscribe());
    upgrade
        .on_upgrade(move |socket| async move {
            if let Some(rx) = rx {
                ws_loop(rx, socket).await;
            }
        })
        .into_response()
}

async fn ws_loop(mut rx: tokio::sync::broadcast::Receiver<String>, mut socket: WebSocket) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        tokio::select! {
            frame = rx.recv() => match frame {
                Ok(f) => {
                    if socket.send(Message::text(f)).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue, // 监控流，丢帧可接受
                Err(RecvError::Closed) => break,
            },
            msg = socket.recv() => match msg {
                // 客户端关连接或出错就退订；忽略 ping/pong（自动应答）
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}
