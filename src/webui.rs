//! WebUI：axum 静态托管 webui/dist + REST + 消息流 WS。
//! 数据与控制台命令同源（stats / business / dll）， REST 契约见
//! webui/src/api/index.ts。独立 std 线程跑 tokio 运行时，不碰主循环。

use std::sync::{Arc, Mutex, OnceLock};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};

use crate::business::Business;
use crate::ffi::CoreDll;
use crate::{log_error, log_info, stats};

const DEFAULT_PORT: u16 = 7900;
const CONFIG_FILE: &str = "config/webui.yaml";

struct WebState {
    dll: Arc<CoreDll>,
    business: Arc<Business>,
    token: Option<String>,
}

// 消息流广播：监控用途，容量满或无订阅者都直接丢。
static STREAM_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();

// ── WebUI 扩展机制（tab 注册）──
// 内置页面与扩展页面统一经此登记，前端导航从 /api/extensions/ui 拉取。
// tab 的界面在 webui/src/views（Vue 组件按 path 映射）；注册只声明
// 导航入口，不含 HTML——插件级自定义页面等有真实需求再演进。
pub struct UiTab {
    pub id: String,
    pub title: String,
    pub path: String,
}

fn ui_tabs() -> &'static Mutex<Vec<UiTab>> {
    static TABS: OnceLock<Mutex<Vec<UiTab>>> = OnceLock::new();
    TABS.get_or_init(|| {
        Mutex::new(vec![
            UiTab { id: "dashboard".into(), title: "仪表盘".into(), path: "/dashboard".into() },
            UiTab { id: "messages".into(), title: "消息".into(), path: "/messages".into() },
            UiTab { id: "plugins".into(), title: "插件".into(), path: "/plugins".into() },
        ])
    })
}

/// 注册一个导航 tab（通用扩展点：内置页面与后续扩展同一路径）。
pub fn register_tab(id: &str, title: &str, path: &str) {
    let mut tabs = ui_tabs().lock().expect("tab 注册表锁中毒");
    if !tabs.iter().any(|t| t.id == id) {
        tabs.push(UiTab { id: id.into(), title: title.into(), path: path.into() });
    }
}

/// 主循环与 worker 线程调用：往消息流广播一帧 JSON 文本。
pub fn publish(frame: String) {
    if let Some(tx) = STREAM_TX.get() {
        let _ = tx.send(frame);
    }
}

fn now_hms() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// 加载完成后调用：起独立线程跑 HTTP 服务。失败只记日志，不拖垮宿主。
pub fn spawn(dll: Arc<CoreDll>, business: Arc<Business>) {
    let (tx, _) = broadcast::channel(256);
    if STREAM_TX.set(tx.clone()).is_err() {
        return; // 已初始化（测试场景重复调用）
    }
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build();
        let Ok(rt) = rt else {
            log_error!("WebUI: tokio 运行时创建失败");
            return;
        };
        if let Err(e) = rt.block_on(serve(dll, business)) {
            log_error!("WebUI 服务退出: {e}");
        }
    });
}

async fn serve(
    dll: Arc<CoreDll>,
    business: Arc<Business>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (port, token) = read_config();
    let state = Arc::new(WebState { dll, business, token });

    // SPA：未命中的路径回 index.html（/dashboard 等前端路由直接访问）
    let assets = ServeDir::new("webui/dist")
        .fallback(ServeFile::new("webui/dist/index.html"));

    let app = Router::new()
        .route("/api/status", get(api_status))
        .route("/api/extensions/ui", get(api_extensions_ui))
        .route("/api/sandbox/send", post(api_sandbox_send))
        .route("/api/adapters", get(api_adapters))
        .route("/api/adapters/apply", post(api_adapters_apply))
        .route("/api/plugins/{name}/config", get(api_plugin_config).put(api_plugin_config_save))
        .route("/api/plugins", get(api_plugins))
        .route("/api/plugins/{name}/enable", post(api_enable))
        .route("/api/plugins/{name}/disable", post(api_disable))
        .route("/api/plugins/{name}/install", post(api_install))
        .route("/api/core/reload-napcat", post(api_reload_napcat))
        .route("/api/ws", get(api_ws))
        .fallback_service(assets)
        .with_state(state)
        .layer(middleware::from_fn(cache_headers));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    log_info!("WebUI: http://127.0.0.1:{port}（数据与控制台命令同源）");
    axum::serve(listener, app).await?;
    Ok(())
}

// 可选配置 config/webui.yaml（port / token），缺失用缺省值、不鉴权。
fn read_config() -> (u16, Option<String>) {
    let Ok(text) = std::fs::read_to_string(CONFIG_FILE) else {
        return (DEFAULT_PORT, None);
    };
    let Ok(docs) = yaml_rust2::YamlLoader::load_from_str(&text) else {
        log_error!("WebUI 配置解析失败，用缺省值");
        return (DEFAULT_PORT, None);
    };
    let Some(cfg) = docs.into_iter().next() else {
        return (DEFAULT_PORT, None);
    };
    let port = cfg["port"].as_i64().map(|p| p as u16).unwrap_or(DEFAULT_PORT);
    let token = cfg["token"].as_str().filter(|t| !t.is_empty()).map(String::from);
    (port, token)
}

// 缓存策略：assets 文件名带内容 hash，可长缓存；HTML 与 API 禁缓存，
// 避免 build 后浏览器拿旧 index.html 引旧样式（“改了没生效”的元凶）。
async fn cache_headers(uri: Uri, request: Request, next: Next) -> Response {
    let mut res = next.run(request).await;
    let cache = if uri.path().starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    if let Ok(v) = HeaderValue::from_str(cache) {
        res.headers_mut().insert("cache-control", v);
    }
    res
}

// token 配置了就校验 Bearer；未配置放行（默认只绑 127.0.0.1）。
fn authorized(state: &WebState, headers: &HeaderMap) -> bool {
    let Some(expected) = &state.token else {
        return true;
    };
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|got| got == expected)
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "unauthorized" }))).into_response()
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
async fn api_extensions_ui() -> Response {
    let tabs = ui_tabs().lock().expect("tab 注册表锁中毒");
    let list: Vec<serde_json::Value> = tabs
        .iter()
        .map(|t| serde_json::json!({ "id": t.id, "title": t.title, "path": t.path }))
        .collect();
    Json(serde_json::json!({ "tabs": list })).into_response()
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
    upgrade: WebSocketUpgrade,
) -> Response {
    if !authorized(&state, &headers) {
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

async fn ws_loop(mut rx: broadcast::Receiver<String>, mut socket: WebSocket) {
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

// ---- publish 的调用方（main.rs 主循环与 worker）在此拼帧，保证消息流格式单一出处 ----

/// 收到消息帧（主循环 Event 分支调用）。
pub fn publish_incoming(platform: &str, message_type: &str, chat_id: &str, text: &str) {
    let where_ = if message_type == "group" { "群" } else { "好友" };
    publish(
        serde_json::json!({
            "dir": "in",
            "platform": platform,
            "chat": format!("{where_} {chat_id}"),
            "text": text,
            "time": now_hms(),
        })
        .to_string(),
    );
}

/// 回复帧（worker 发送成功后调用）。
pub fn publish_reply(platform: &str, message_type: &str, chat_id: &str, text: &str) {
    let where_ = if message_type == "group" { "群" } else { "好友" };
    publish(
        serde_json::json!({
            "dir": "out",
            "platform": platform,
            "chat": format!("{where_} {chat_id}"),
            "text": text,
            "time": now_hms(),
        })
        .to_string(),
    );
}
