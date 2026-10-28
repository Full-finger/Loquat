//! WebUI：axum 静态托管 webui/dist + REST + 消息流 WS。
//! 数据与控制台命令同源（stats / business / dll）， REST 契约见
//! webui/src/api/index.ts。独立 std 线程跑 tokio 运行时，不碰主循环。
//! 按职责分文件：REST/WS handlers 在 api.rs。

mod api;

use std::sync::{Arc, Mutex, OnceLock};

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};

use crate::business::Business;
use crate::ffi::CoreDll;
use crate::{log_error, log_info};

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

/// tab 注册表的 JSON 快照（api.rs 的 /api/extensions/ui 用）。
fn ui_tabs_snapshot() -> Vec<serde_json::Value> {
    let tabs = ui_tabs().lock().expect("tab 注册表锁中毒");
    tabs.iter()
        .map(|t| serde_json::json!({ "id": t.id, "title": t.title, "path": t.path }))
        .collect()
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

    let app = api::routes()
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
fn authorized(state: &WebState, headers: &axum::http::HeaderMap) -> bool {
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
