//! WebUI：axum 静态托管 webui/dist + REST + 消息流 WS。
//! 数据与控制台命令同源（stats / business / dll）， REST 契约见
//! webui/src/api/index.ts。独立 std 线程跑 tokio 运行时，不碰主循环。
//! 按职责分文件：REST/WS handlers 在 api.rs。

mod api;

use std::sync::{Arc, OnceLock};

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

// WebUI 扩展机制（插件页面）：导航 tab 由 /api/extensions/ui 聚合插件
// 的 TABS 声明，脚本经 /ext/<名>/ui.js 分发，前端动态挂载（docs/ext-ui.md）。
// 内置页面的导航在前端 webui/src/tabs.ts，不经过宿主。

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
            .worker_threads(4)
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
        .layer(middleware::from_fn(cache_headers))
        .layer(middleware::from_fn(same_site_guard));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    log_info!("WebUI: http://127.0.0.1:{port}（数据与控制台命令同源）");
    axum::serve(listener, app).await?;
    Ok(())
}

// 站点边界（本机服务对浏览器暴露面收口，26102701#2）：
// - Host 必须是回环名——DNS rebinding 把攻击域名解析到 127.0.0.1 时，
//   浏览器发出的 Host 头仍是攻击域名，在此拒掉，GET 数据（含适配器
//   token / 插件 api_key）就带不走；
// - Origin 存在时必须同为回环且端口与 Host 一致——恶意网页的跨站
//   simple request（表单/fetch no-cors）不触发预检也拦不住自定义头，
//   唯有 Origin 骗不了；同源页面的 WS 与 fetch 自带同源 Origin，不受影响。
// 非 POST 的静态资源同受保护：rebound 页面连我们的 JS 也拿不去。
async fn same_site_guard(request: Request, next: Next) -> Response {
    let host_ok = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(loopback_name)
        .unwrap_or(false);
    let origin_ok = request
        .headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|origin| origin_is_same_site(origin, request.headers()))
        .unwrap_or(true); // 无 Origin = 非浏览器客户端（curl/WS 工具），交由 token 鉴权
    if !host_ok || !origin_ok {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": "cross-site" })))
            .into_response();
    }
    next.run(request).await
}

// Host 头的主机部分是否为回环名（127.0.0.1 / localhost / [::1]）。
fn loopback_name(host_port: &str) -> bool {
    let host = host_port
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(host_port);
    host == "127.0.0.1" || host == "localhost" || host == "[::1]"
}

// Origin 与 Host 是否同站：主机回环 + 端口一致（缺省端口按 scheme 补）。
fn origin_is_same_site(origin: &str, headers: &axum::http::HeaderMap) -> bool {
    let (scheme, rest) = match origin.split_once("://") {
        Some((s, r)) => (s, r),
        None => return false,
    };
    let authority = rest.split('/').next().unwrap_or("");
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(0)),
        None => (authority, if scheme == "https" { 443 } else { 80 }),
    };
    if host != "127.0.0.1" && host != "localhost" && host != "[::1]" {
        return false;
    }
    headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.rsplit_once(':'))
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .is_some_and(|host_port| host_port == port)
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

#[cfg(test)]
mod tests {
    use super::{loopback_name, origin_is_same_site};
    use axum::http::HeaderMap;

    fn hostmap(host: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert("host", host.parse().unwrap());
        m
    }

    #[test]
    fn host_must_be_loopback() {
        assert!(loopback_name("127.0.0.1:7900"));
        assert!(loopback_name("localhost:7900"));
        assert!(loopback_name("[::1]:7900"));
        // DNS rebinding：浏览器带的是攻击域名
        assert!(!loopback_name("evil.example.com:7900"));
        assert!(!loopback_name("10.0.0.5:7900"));
    }

    #[test]
    fn origin_must_be_same_site() {
        let h = hostmap("127.0.0.1:7900");
        assert!(origin_is_same_site("http://127.0.0.1:7900", &h));
        assert!(origin_is_same_site("http://localhost:7900", &h));
        // 跨站网页发来的 simple request：Origin 不是回环
        assert!(!origin_is_same_site("https://evil.example.com", &h));
        // 回环但端口不同：本机其他服务发起的跨端口请求，同样拒
        assert!(!origin_is_same_site("http://127.0.0.1:9999", &h));
        // Host 缺端口时无从比对，拒绝（fail closed）
        assert!(!origin_is_same_site("http://127.0.0.1:7900", &hostmap("127.0.0.1")));
    }
}
