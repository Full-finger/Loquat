//! 统一事件信封（IR）解析：把 Go 核心投递的 EventEnvelope JSON
//! （protojson 形态，见 internal/pluginapi/adapter.proto）转成宿主
//! 用的 Incoming。OneBot 的平台知识已下沉到适配器，这里只认 IR。

use crate::console;
use crate::log_error;

/// 会话指向：私聊为对方用户 ID，群聊为群号。ID 全 String，
/// 跨平台不可假设为数字。
#[derive(Debug, Clone)]
pub enum Chat {
    Private { id: String },
    Group { id: String },
}

impl Chat {
    /// Python ctx 的 message_type 兼容值："private" / "group"。
    pub fn message_type(&self) -> &'static str {
        match self {
            Chat::Private { .. } => "private",
            Chat::Group { .. } => "group",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Chat::Private { id } | Chat::Group { id } => id,
        }
    }
}

/// 解析结果：传给插件 handle(ctx) 的字段来源。
/// text/images 是从 segments 派生的便捷字段；结构化段在 segments 里，
/// protojson 形态（如 {"text":{"text":"早"}}），插件可按需取用。
pub struct Incoming {
    pub platform: String,
    pub chat: Chat,
    pub sender_id: String,
    pub self_id: String,
    /// 触发消息的 ID（撤回/表情回应类工具用）。平台不回时为空串，
    /// 工具调用时再报错，不影响消息本体处理
    pub message_id: String,
    /// 是否 at 了机器人。适配器已判定（只有它知道自己的 bot ID）
    pub mentions_bot: bool,
    /// 结构化消息段（protojson 形态）
    pub segments: Vec<serde_json::Value>,
    /// 全部 text 段拼接（不含 CQ 码等标记）
    pub text: String,
    /// 消息里的图片 URL（image 段 url 优先，file_id 是 http 时兜底）
    pub images: Vec<String>,
}

// 图片提取上限：一次消息几十张图会把视觉模型的 token 打爆
const MAX_IMAGES: usize = 4;

/// 解析事件信封 JSON。非 MESSAGE 事件（notice/request/meta）返回 None，
/// 静默忽略；message 事件缺关键字段走日志后丢弃（信封有 raw 兜底）。
pub fn parse_incoming(payload: &[u8]) -> Option<Incoming> {
    let raw = || console::brief(&String::from_utf8_lossy(payload), 200);
    let v: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(e) => {
            log_error!("事件信封 JSON 解析失败，已丢弃: {e}; 原文: {}", raw());
            return None;
        }
    };
    // protojson 的 enum 输出名字（adapter.proto EventKind）
    if v.get("kind").and_then(|k| k.as_str()) != Some("EVENT_KIND_MESSAGE") {
        return None;
    }
    let msg = match v.get("message") {
        Some(m) if m.is_object() => m,
        _ => {
            log_error!("MESSAGE 信封缺 message，已丢弃; 原文: {}", raw());
            return None;
        }
    };

    let platform = v
        .get("platform")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let chat = match parse_chat(msg) {
        Some(c) => c,
        None => {
            log_error!("事件缺 chat 或 chat.kind，已丢弃; 原文: {}", raw());
            return None;
        }
    };
    let sender_id = field_str(msg, &["sender", "id"]);
    if sender_id.is_empty() {
        log_error!("事件缺 sender.id，已丢弃; 原文: {}", raw());
        return None;
    }
    // 机器人自己的消息不上报（适配器已拦一层，这里兜双保险防自问自答）
    let self_id = field_str(&v, &["selfId"]);
    if sender_id == self_id && !self_id.is_empty() {
        return None;
    }

    let segments: Vec<serde_json::Value> = msg
        .get("segments")
        .and_then(|s| s.as_array())
        .map(|a| a.clone())
        .unwrap_or_default();

    Some(Incoming {
        platform,
        text: derive_text(&segments),
        images: derive_images(&segments),
        chat,
        sender_id,
        self_id,
        message_id: msg
            .get("id")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string(),
        mentions_bot: msg
            .get("mentionsBot")
            .and_then(|b| b.as_bool())
            .unwrap_or(false),
        segments,
    })
}

// chat：protojson oneof 形态 {"privateId": "..."} / {"groupId": "..."}
fn parse_chat(msg: &serde_json::Value) -> Option<Chat> {
    let chat = msg.get("chat")?;
    if let Some(id) = chat.get("privateId").and_then(|s| s.as_str()) {
        return Some(Chat::Private { id: id.to_string() });
    }
    if let Some(id) = chat.get("groupId").and_then(|s| s.as_str()) {
        return Some(Chat::Group { id: id.to_string() });
    }
    None
}

// 按路径取字符串字段（只认对象链）
fn field_str(v: &serde_json::Value, path: &[&str]) -> String {
    let mut cur = v;
    for (i, key) in path.iter().enumerate() {
        match cur.get(key) {
            Some(next) if i == path.len() - 1 => {
                return next.as_str().unwrap_or("").to_string();
            }
            Some(next) => cur = next,
            None => return String::new(),
        }
    }
    String::new()
}

// 派生文本：拼全部 text 段
fn derive_text(segments: &[serde_json::Value]) -> String {
    let mut out = String::new();
    for seg in segments {
        if let Some(t) = seg.get("text").and_then(|t| t.get("text")).and_then(|t| t.as_str()) {
            out.push_str(t);
        }
    }
    out
}

// 派生图片 URL：image 段 url 优先；fileId 以 http 开头时兜底；去重截断
fn derive_images(segments: &[serde_json::Value]) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for seg in segments {
        let Some(img) = seg.get("image") else { continue };
        let url = img
            .get("url")
            .and_then(|u| u.as_str())
            .or_else(|| {
                img.get("fileId")
                    .and_then(|f| f.as_str())
                    .filter(|f| f.starts_with("http"))
            });
        if let Some(u) = url {
            if u.starts_with("http") && !urls.iter().any(|x| x == u) {
                urls.push(u.to_string());
            }
        }
        if urls.len() >= MAX_IMAGES {
            break;
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 与 Go 侧 consume 的 protojson 形态一致的信封样例
    fn envelope(message: serde_json::Value) -> Vec<u8> {
        serde_json::json!({
            "platform": "napcat",
            "kind": "EVENT_KIND_MESSAGE",
            "selfId": "7",
            "message": message,
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn private_message_parses() {
        let payload = envelope(json!({
            "id": "100",
            "chat": {"privateId": "42"},
            "sender": {"id": "42", "nickname": "nm"},
            "segments": [{"text": {"text": "早"}}],
            "mentionsBot": false,
        }));
        let inc = parse_incoming(&payload).expect("应解析成功");
        assert!(matches!(inc.chat, Chat::Private { ref id } if id == "42"));
        assert_eq!(inc.chat.message_type(), "private");
        assert_eq!(inc.chat.id(), "42");
        assert_eq!(inc.sender_id, "42");
        assert_eq!(inc.message_id, "100");
        assert_eq!(inc.text, "早");
        assert!(!inc.mentions_bot);
    }

    #[test]
    fn group_at_bot_number_and_string_qq() {
        // at 的 qq 数字与字符串两种（适配器已归一成字符串，这里验透传）
        for target in ["7", "7"] {
            let payload = envelope(json!({
                "id": "1",
                "chat": {"groupId": "99"},
                "sender": {"id": "42"},
                "segments": [
                    {"text": {"text": "早"}},
                    {"at": {"target": target}},
                ],
                "mentionsBot": true,
            }));
            let inc = parse_incoming(&payload).expect("应解析成功");
            assert!(matches!(inc.chat, Chat::Group { ref id } if id == "99"));
            assert!(inc.mentions_bot, "target={target}");
        }
    }

    #[test]
    fn non_message_events_skipped() {
        for kind in ["EVENT_KIND_NOTICE", "EVENT_KIND_REQUEST", "EVENT_KIND_META"] {
            let payload = serde_json::json!({"platform": "napcat", "kind": kind}).to_string();
            assert!(parse_incoming(payload.as_bytes()).is_none(), "{kind}");
        }
    }

    #[test]
    fn missing_chat_or_sender_dropped() {
        let no_chat = envelope(json!({"id": "1", "sender": {"id": "42"}, "segments": []}));
        assert!(parse_incoming(&no_chat).is_none());
        let no_sender = envelope(json!({"id": "1", "chat": {"groupId": "99"}, "segments": []}));
        assert!(parse_incoming(&no_sender).is_none());
    }

    #[test]
    fn self_message_dropped() {
        let payload = envelope(json!({
            "id": "1",
            "chat": {"privateId": "7"},
            "sender": {"id": "7"},
            "segments": [],
        }));
        assert!(parse_incoming(&payload).is_none());
    }

    #[test]
    fn derived_text_joins_text_segments_only() {
        let payload = envelope(json!({
            "id": "1",
            "chat": {"groupId": "99"},
            "sender": {"id": "42"},
            "segments": [
                {"text": {"text": "看"}},
                {"image": {"url": "http://x/a.png"}},
                {"text": {"text": "这个"}},
            ],
        }));
        let inc = parse_incoming(&payload).expect("应解析成功");
        assert_eq!(inc.text, "看这个"); // CQ 码等标记不再混进 text
        assert_eq!(inc.images, vec!["http://x/a.png".to_string()]);
    }

    #[test]
    fn derived_images_cap_and_dedup() {
        let payload = envelope(json!({
            "id": "1",
            "chat": {"groupId": "99"},
            "sender": {"id": "42"},
            "segments": [
                {"image": {"url": "http://x/1.png"}},
                {"image": {"url": "http://x/1.png"}},
                {"image": {"fileId": "http://x/2.png"}},
                {"image": {"fileId": "local_only"}},
                {"image": {"url": "http://x/3.png"}},
                {"image": {"url": "http://x/4.png"}},
                {"image": {"url": "http://x/5.png"}},
            ],
        }));
        let inc = parse_incoming(&payload).expect("应解析成功");
        assert_eq!(
            inc.images,
            vec![
                "http://x/1.png".to_string(),
                "http://x/2.png".to_string(),
                "http://x/3.png".to_string(),
                "http://x/4.png".to_string(),
            ]
        );
    }

    #[test]
    fn segments_survive_for_plugins() {
        // 结构化段透传：插件可按 protojson 形态自取
        let payload = envelope(json!({
            "id": "1",
            "chat": {"groupId": "99"},
            "sender": {"id": "42"},
            "segments": [{"reply": {"messageId": "100"}}],
        }));
        let inc = parse_incoming(&payload).expect("应解析成功");
        assert_eq!(inc.segments[0]["reply"]["messageId"], "100");
    }
}
