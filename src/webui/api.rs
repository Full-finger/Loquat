//! WebUI 的 REST 与 WS handlers。路由表在 mod.rs 的 serve()；
//! 鉴权（authorized）与状态（WebState/STREAM_TX）由 mod.rs 持有。
//!
//! 文件划分：api.rs 是核心与扩展页分发，api_sandbox.rs 是沙箱选项卡，
//! api_plugins.rs 是适配器/Provider/插件管理面。R4 规则限单文件 400 行。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};

use super::api_plugins::{
    api_adapters, api_adapters_apply, api_disable, api_enable, api_install, api_plugin_asset_delete,
    api_plugin_asset_upload, api_plugin_asset_view, api_plugin_assets, api_plugin_config,
    api_plugin_config_save, api_plugins, api_providers, api_providers_save,
};
use super::api_sandbox::{
    api_sandbox_history, api_sandbox_media, api_sandbox_send, api_sandbox_session_create,
    api_sandbox_session_delete, api_sandbox_session_rename, api_sandbox_sessions,
};
use super::{authorized, unauthorized, STREAM_TX, WebState};
use crate::stats;

/// spawn_blocking 任务 panic 时兜底，不让请求静默挂死。
pub(super) fn join_lost() -> Response {
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
pub(crate) use blocking;

/// 全部 API 路由（宿主能力面）。新增端点在此登记。
pub fn routes() -> Router<Arc<WebState>> {
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/extensions/ui", get(api_extensions_ui))
        .route("/api/sandbox/send", post(api_sandbox_send))
        // 沙箱会话：列表 / 新建 / 改名+删除 / 历史 / 图片（记录见 sandbox_store）
        .route(
            "/api/sandbox/sessions",
            get(api_sandbox_sessions).post(api_sandbox_session_create),
        )
        .route(
            "/api/sandbox/sessions/{chat_id}",
            patch(api_sandbox_session_rename).delete(api_sandbox_session_delete),
        )
        .route("/api/sandbox/history", get(api_sandbox_history))
        .route("/api/sandbox/media/{name}", get(api_sandbox_media))
        .route("/api/adapters", get(api_adapters))
        .route("/api/adapters/apply", post(api_adapters_apply))
        .route("/api/providers", get(api_providers).post(api_providers_save))
        .route("/api/plugins/{name}/config", get(api_plugin_config).put(api_plugin_config_save))
        .route(
            "/api/plugins/{name}/assets",
            get(api_plugin_assets)
                .post(api_plugin_asset_upload)
                .delete(api_plugin_asset_delete),
        )
        .route("/api/plugins/{name}/assets/view", get(api_plugin_asset_view))
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
        let mut core: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        // 多适配器：任意平台存活即视为运行中
        let alive = core
            .get("adapters")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().any(|x| x.get("alive").and_then(|b| b.as_bool()).unwrap_or(false)))
            .unwrap_or(false);
        // 适配器子进程占用：按 pid 探测 CPU/内存，进程已退出的留空
        if let Some(arr) = core.get_mut("adapters").and_then(|v| v.as_array_mut()) {
            for a in arr {
                if let Some(pid) = a.get("pid").and_then(|v| v.as_u64()) {
                    if let Some((cpu, mem)) = super::proc::probe(pid as u32) {
                        a["cpu"] = serde_json::json!(cpu);
                        a["mem"] = serde_json::json!(mem);
                    }
                }
            }
        }
        Json(serde_json::json!({
            "core": if alive { "运行中" } else { "未就绪" },
            "core_detail": core,
            "events": stats::event_count(),
            "replies": stats::reply_count(),
            "workers": stats::worker_backlogs(),
            "workers_done": stats::worker_done(),
            "active_chats": stats::active_chats(),
            "uptime_sec": super::uptime_sec(),
            "sys": super::proc::host(),
            "business": state.business.info(),
        }))
        .into_response()
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
pub(super) fn token_query_ok(state: &WebState, query: Option<&str>) -> bool {
    match (&state.token, query) {
        (Some(expected), Some(q)) => q.split('&').any(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            k == "token" && v == expected
        }),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

// ?a=1&b=2 里取指定参数的值并做百分号解码（文件名可能是中文）。
pub(super) fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    query?.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == key).then(|| percent_decode(v))
    })
}

// 手写百分号解码：文件名来自 fetch 的 encodeURIComponent，多字节
// 序列按字节还原；非法转义原样保留。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// 按扩展名给 Content-Type；浏览器预览只认图片，其余当下载。
pub(super) fn asset_content_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "txt" | "log" | "json" | "yaml" | "yml" => "text/plain",
        _ => "application/octet-stream",
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
