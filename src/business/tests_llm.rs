//! loquat_llm 的一次重试（建议 4）：429/5xx/连接错误重试一次，4xx 不重试。
//! 用本机 TcpListener 当假 LLM 服务，按连接顺序给预设应答。
//! helpers 与 tests.rs 各持一份（模块解耦惯例）。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use super::tests::ctx;
use super::*;
use crate::pluginload::SIGNATURE;

fn plugin_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loquat_biz_llm_test_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["log.py", "llm.py"] {
        let lib = dir.join("_lib");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::copy(Path::new("extensions/_lib").join(f), lib.join(f)).unwrap();
    }
    dir
}

/// 假 LLM 服务：对第 i 个连接回 statuses[i]（200 带最小 JSON 体），
/// 每服务完一个连接就往通道报一次。statuses 用完线程即退、监听口关闭。
fn fake_llm(statuses: Vec<u16>) -> (u16, mpsc::Receiver<u16>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for status in statuses {
            let Ok((mut s, _)) = listener.accept() else {
                break;
            };
            // 读完整个请求（头 + Content-Length 体）再应答，避免对端收到 RST
            let mut buf = Vec::new();
            let mut tmp = [0u8; 4096];
            loop {
                let n = s.read(&mut tmp).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if let Some(end) = header_end(&buf) {
                    if buf.len() >= end + content_length(&buf[..end]) {
                        break;
                    }
                }
            }
            let body = if status == 200 {
                r#"{"choices":[{"message":{"content":"ok"}}]}"#
            } else {
                ""
            };
            let resp = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(resp.as_bytes());
            let _ = s.flush();
            let _ = tx.send(status);
        }
    });
    (port, rx)
}

/// 头部结束位置（"\r\n\r\n" 之后的偏移）。
fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn content_length(head: &[u8]) -> usize {
    String::from_utf8_lossy(head)
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

/// 调 loquat_llm.chat 的插件：装一个无代理 opener，确保 127.0.0.1 不被系统代理劫走。
fn write_llm_plugin(dir: &Path, port: u16) {
    let code = format!(
        "{SIGNATURE}\n\
         import urllib.request\n\
         import loquat_llm\n\
         urllib.request.install_opener(urllib.request.build_opener(urllib.request.ProxyHandler({{}})))\n\
         CFG = {{'api_base': 'http://127.0.0.1:{port}', 'api_key': 'k', 'model': 'm',\n\
         \x20      'temperature': 0, 'max_tokens': 8, 'system_prompt': 's', 'timeout_seconds': 5}}\n\
         def handle(ctx):\n\
         \x20   r = loquat_llm.chat(CFG, [{{'role': 'user', 'content': ctx['text']}}])\n\
         \x20   return [r.text if r else 'NONE']\n"
    );
    let d = dir.join("p");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(ENTRY_FILE), code).unwrap();
}

#[test]
fn llm_retries_once_on_server_error() {
    let dir = plugin_dir("retry");
    let (port, served) = fake_llm(vec![500, 200]);
    write_llm_plugin(&dir, port);
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.texts(&ctx("hi")).unwrap(),
        vec!["ok".to_string()],
        "首连 500 后重试一次拿到 200"
    );
    assert_eq!(served.recv_timeout(Duration::from_secs(2)).unwrap(), 500);
    assert_eq!(served.recv_timeout(Duration::from_secs(2)).unwrap(), 200);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn llm_does_not_retry_client_error() {
    let dir = plugin_dir("noretry");
    let (port, served) = fake_llm(vec![400]);
    write_llm_plugin(&dir, port);
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.texts(&ctx("hi")).unwrap(),
        vec!["NONE".to_string()],
        "4xx 语义错误不重试，直接降级为 None"
    );
    assert_eq!(served.recv_timeout(Duration::from_secs(2)).unwrap(), 400);
    // statuses 用完监听线程退出、发送端析构：没有第二个连接
    assert!(served.recv_timeout(Duration::from_millis(500)).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
