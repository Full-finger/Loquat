//! 沙箱选项卡的 API handlers：注入消息、会话管理、历史与媒体回读。
//! 记录本体在 sandbox_store；路由登记在 api.rs 的 routes()。

use std::sync::Arc;

use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::api::{asset_content_type, join_lost, query_param, token_query_ok};
use super::{authorized, sandbox_store, unauthorized, WebState};

/// 沙箱选项卡的输入通路：注入一条消息进沙箱适配器的事件链路。
/// body: {"segments": [...], "mode": "private"|"group", "atBot": bool, "session": "<chat_id>"}
/// segments 是 protojson Segment 数组（与 SendMessage 同构，可带图片/回复段）；
/// session 是会话标识，缺省（老前端）落到该 mode 的缺省会话。
/// 应答带 messageId，前端绑到本地气泡供引用回复。
pub(super) async fn api_sandbox_send(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let segments = body
        .get("segments")
        .cloned()
        .unwrap_or(serde_json::json!([]));
    let mode = body
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("private")
        .to_string();
    let at_bot = body.get("atBot").and_then(|b| b.as_bool()).unwrap_or(false);
    if !segments.is_array() || segments.as_array().is_some_and(|a| a.is_empty()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "segments 为空" })),
        )
            .into_response();
    }
    if mode != "private" && mode != "group" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "mode 仅支持 private/group" })),
        )
            .into_response();
    }
    // 会话：空 = 缺省会话（兼容老前端）；给了就必须是登记过的，避免写出野文件
    let chat_id = body
        .get("session")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string();
    if !chat_id.is_empty() && !sandbox_store::exists(&chat_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "会话不存在，请刷新会话列表" })),
        )
            .into_response();
    }

    let segments_str = segments.to_string();
    // 注入要在阻塞池跑；结果拿出来后再记账（失败不落记录）
    let call = {
        let dll = state.dll.clone();
        let segs = segments_str;
        let mode = mode.clone();
        let chat = chat_id.clone();
        // Box<dyn Error> 不是 Send，闭包里先转字符串
        tokio::task::spawn_blocking(move || {
            dll.inject_event("sandbox", &segs, at_bot, &mode, &chat)
                .map_err(|e| e.to_string())
        })
        .await
    };
    let raw = match call {
        Ok(Ok(raw)) => raw,
        Ok(Err(e)) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
        Err(_) => return join_lost(),
    };
    // 记账用实际会话：空 session 时按 mode 落缺省会话
    let recorded_chat = if chat_id.is_empty() {
        if mode == "group" {
            sandbox_store::DEFAULT_GROUP
        } else {
            sandbox_store::DEFAULT_PRIVATE
        }
    } else {
        chat_id.as_str()
    };
    let msg_id = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| {
            v.get("messageId")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
        .unwrap_or_default();
    // 有登记才记；缺省会话在种子数据里，正常情况下都在
    if sandbox_store::exists(recorded_chat) {
        sandbox_store::record_in(recorded_chat, &msg_id, &segments);
    }
    Json(serde_json::json!({ "ok": true, "detail": raw })).into_response()
}

/// 会话列表（按最近使用倒序）。
pub(super) async fn api_sandbox_sessions(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    Json(serde_json::json!({ "sessions": sandbox_store::list() })).into_response()
}

/// 新建会话：body {"mode": "private"|"group"}。名字先占位，首条消息自动取前几个字。
pub(super) async fn api_sandbox_session_create(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let mode = body
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("private");
    match sandbox_store::create(mode) {
        Ok(s) => Json(serde_json::json!({ "session": s })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// 改名：body {"name": "..."}。改名后不再被首条消息覆盖。
pub(super) async fn api_sandbox_session_rename(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(chat_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("");
    match sandbox_store::rename(&chat_id, name) {
        Ok(s) => Json(serde_json::json!({ "session": s })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// 删除会话：记录与机器人上下文一起删（否则「新建会话」会继承旧记忆）。
pub(super) async fn api_sandbox_session_delete(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    Path(chat_id): Path<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    match sandbox_store::remove(&chat_id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// 会话历史：?session=<chat_id>&limit=200，返回 [{dir, id?, segments?|text?, ts}]。
pub(super) async fn api_sandbox_history(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    if !authorized(&state, &headers) {
        return unauthorized();
    }
    let chat_id = query_param(query.as_deref(), "session").unwrap_or_default();
    if chat_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "缺少 session" })),
        )
            .into_response();
    }
    let limit = query_param(query.as_deref(), "limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200);
    Json(serde_json::json!({ "messages": sandbox_store::history(&chat_id, limit) })).into_response()
}

/// 会话里的图片：dataURL 落盘后的回读。<img> 带不了 Authorization 头，
/// token 查询参数兜底（与插件资源预览同一策略）。
pub(super) async fn api_sandbox_media(
    State(state): State<Arc<WebState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    Path(name): Path<String>,
) -> Response {
    if !token_query_ok(&state, query.as_deref()) && !authorized(&state, &headers) {
        return unauthorized();
    }
    let Some(path) = sandbox_store::media_path(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            [(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(asset_content_type(&name)),
            )],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
