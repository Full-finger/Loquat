//! 小工具：时间、短 ID、名字清理、行读取、dataURL 与 base64 编解码。

use std::path::Path;

use serde_json::Value;

use super::NAME_MAX_CHARS;

pub(super) fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

/// 6 位十六进制短 ID：纳秒时间戳混淆 + 进程号 + 重试序号，单机足够唯一。
pub(super) fn short_id(attempt: u64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mixed = nano_mix(nanos)
        ^ (std::process::id() as u64).rotate_left(17)
        ^ attempt.wrapping_mul(0x9E37);
    format!("{:06x}", mixed & 0xFF_FFFF)
}

fn nano_mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

/// 会话名：压掉换行与多余空白，按字符截断（中文不能按字节切）。
pub(super) fn clamp_name(raw: &str) -> String {
    let flat = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = flat.chars().take(NAME_MAX_CHARS).collect();
    if out.chars().count() == NAME_MAX_CHARS && flat.chars().count() > NAME_MAX_CHARS {
        out.push('…');
    }
    out.trim().to_string()
}

/// 取第一条文本段做自动命名；没有文本就返回空（等有文本的消息再说）。
pub(super) fn text_of(segments: &Value) -> &str {
    segments
        .as_array()
        .and_then(|arr| {
            arr.iter().find_map(|s| {
                s.get("text")
                    .and_then(|t| t.get("text"))
                    .and_then(Value::as_str)
            })
        })
        .unwrap_or("")
}

pub(super) fn read_lines(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect())
}

pub(super) fn split_data_url(url: &str) -> Option<(String, &str)> {
    let rest = url.strip_prefix("data:")?;
    let (mime, payload) = rest.split_once(';')?;
    let payload = payload.strip_prefix("base64,")?;
    Some((mime.to_string(), payload))
}

pub(super) fn media_ext(mime: &str) -> &'static str {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "bin",
    }
}

pub(super) fn hash_hex(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// 标准 base64 解码（含填充）：不引额外依赖，几十行够用。
pub(super) fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for b in s.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
            buf &= if bits == 0 { 0 } else { (1u32 << bits) - 1 };
        }
    }
    Some(out)
}
