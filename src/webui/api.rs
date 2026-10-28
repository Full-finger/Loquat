//! WebUI 的 REST 与 WS handlers。路由表在 mod.rs 的 serve()；
//! 鉴权（authorized）与状态（WebState/STREAM_TX）由 mod.rs 持有。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use super::{authorized, unauthorized, STREAM_TX, WebState};
use crate::stats;

/// spawn_blocking 任务 panic 时兜底，不让请求静默挂死。
fn join_lost() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "后台任务异常" })),
    )
        .into_response()
}

/// 触 FFI 或业务锁的 handler 统一放阻塞池：tokio worker 只跑异步 I/O，
/// 慢请求（LLM 等待、平台超时）不再冻结 WS 推流与其余 API。
macro_rules! blocking {
    ($state:expr, $body:expr) => {
        tokio::task::spawn_blocking(move || $body).await.unwrap_or_else(|_| join_lost())
    };
}

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
        // 插件页面脚本（WebUI 扩展机制）：前端动态 import() 加载
        .route("/ext/{name}/ui.js", get(ext_ui_js))
}

async fn api_status(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
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
    })
}

/// 适配器配置 + 运行状态。
async fn api_adapters(State(state): State<Arc<WebState>>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
        let raw = state.dll.adapters_config();
        let v: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        Json(v).into_response()
    })
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
    blocking!(state, match state.dll.apply_adapters_config(&body.to_string()) {
        Ok(s) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::Value::Null);
            Json(v).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    })
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
    blocking!(state, match crate::providers::save(&body) {
        Ok(()) => {
            state.business.refresh_providers();
            Json(serde_json::json!({ "ok": true, "msg": "已保存，服务配置热生效" }))
                .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    })
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
    blocking!(state, match state.business.plugin_config(&name) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))).into_response(),
    })
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
    blocking!(state, match state.business.save_plugin_config(&name, body) {
        Ok(()) => Json(serde_json::json!({ "ok": true, "msg": "已保存并热重载" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    })
}

/// 插件贡献的导航 tab（WebUI 扩展机制）：聚合已启用插件的 TABS 声明。
/// 内置页面的导航在前端 tabs.ts 声明（组件本来就在前端），不经过宿主。
/// 虽只是导航元数据，也与其他端点统一鉴权（避免「免 token 探针」）。
async fn api_extensions_ui(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
        let tabs = state.business.ui_tabs();
        Json(serde_json::json!({ "tabs": tabs })).into_response()
    })
}

/// 插件页面脚本分发（WebUI 扩展机制）。浏览器动态 import() 拿不到
/// Authorization 头，token 走 ?token= 查询参数兜底（同 /api/ws）。
async fn ext_ui_js(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    Path(name): Path<String>,
) -> Response {
    if !token_query_ok(&state, query.as_deref()) && !authorized(&state, &headers) {
        return unauthorized();
    }
    // 路径解析可能扫插件目录，放阻塞池；文件本体仍走 tokio::fs 异步读
    let path = tokio::task::spawn_blocking(move || state.business.plugin_ui_js(&name))
        .await
        .unwrap_or(None);
    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::read(path).await {
        Ok(bytes) => (
            [(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript"),
            )],
            bytes,
        )
            .into_response(),
        // 读完文件消失（热更窗口）与不存在同样对待，前端 v 指纹会重取
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
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
    // owned 提取：闭包要进 spawn_blocking，不能借用局部 body
    let text = body.get("text").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
    let mode = body.get("mode").and_then(|m| m.as_str()).unwrap_or("private").to_string();
    let at_bot = body.get("atBot").and_then(|b| b.as_bool()).unwrap_or(false);
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "text 为空" }))).into_response();
    }
    if mode != "private" && mode != "group" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "mode 仅支持 private/group" }))).into_response();
    }
    blocking!(state, match state.dll.inject_event("sandbox", &text, at_bot, &mode) {
        Ok(resp) => Json(serde_json::json!({ "ok": true, "detail": resp })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    })
}

async fn api_plugins(
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
    })
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
    blocking!(state, if state.business.set_enabled(&name, true) {
        (StatusCode::OK, Json(serde_json::json!({ "msg": format!("{name} 已启用") }))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "插件不存在" }))).into_response()
    })
}

async fn api_disable(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, if state.business.set_enabled(&name, false) {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "msg": format!("{name} 已禁用（重启后回到文件内声明）") })),
        )
            .into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "插件不存在" }))).into_response()
    })
}

async fn api_install(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, match state.business.install(&name) {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "msg": msg }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    })
}

async fn api_reload_napcat(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    blocking!(state, {
        let msg = if state.dll.reload_plugin("napcat") == 0 {
            "已触发，核心将退避重启适配器".to_string()
        } else {
            "失败（核心未启动）".to_string()
        };
        (StatusCode::OK, Json(serde_json::json!({ "msg": msg }))).into_response()
    })
}

// token 走 ?token= 查询参数兜底的校验：浏览器 WebSocket API 与动态
// import() 都带不了 Authorization 头（token 会进访问日志；服务只绑回环）。
fn token_query_ok(state: &WebState, query: Option<&str>) -> bool {
    match (&state.token, query) {
        (Some(expected), Some(q)) => q.split('&').any(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            k == "token" && v == expected
        }),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

async fn api_ws(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !token_query_ok(&state, query.as_deref()) && !authorized(&state, &headers) {
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
