//! OneBot 11 事件解析：从上报 JSON 提取 MVP 关心的字段，at 判定走
//! message 段正规解析（CQ 码字符串与数组两种格式，清单 26102201 #4）。

use crate::console;
use crate::log_error;

/// 解析结果：传给插件 handle(ctx) 的字段来源。
pub struct Incoming {
    pub message_type: String,
    pub chat_id: i64,
    pub user_id: i64,
    pub self_id: i64,
    /// 触发消息的 ID（OneBot 11 标准 message 事件必有）；撤回/表情回应类
    /// 工具用。非标实现缺失时为 0，工具调用时再报错，不影响消息本体处理
    pub message_id: i64,
    /// 是否 at 了机器人（校验 qq == self_id），由 message 段正规解析得出
    pub is_at_bot: bool,
    /// 消息里的图片 URL（message 段正规解析，上限 MAX_IMAGES 张）
    pub images: Vec<String>,
    pub text: String,
}

// 图片提取上限：一次消息几十张图会把视觉模型的 token 打爆
const MAX_IMAGES: usize = 4;

// 图片 URL 收集（数组与 CQ 码两种格式都认，顺序去重）。
// 数组格式取 data.url 优先，缺失退化 data.file 里的 http 链接
// （部分实现只给 file）；CQ 码格式解析 url=/file= 参数并反转义。
fn extract_images(v: &serde_json::Value, text: &str) -> Vec<String> {
    let mut urls = match v.get("message") {
        Some(m) if m.is_array() => seg_image_urls(m),
        _ => Vec::new(),
    };
    for u in cq_image_urls(text) {
        push_image_url(&mut urls, &u);
    }
    urls.truncate(MAX_IMAGES);
    urls
}

fn seg_image_urls(segs: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(items) = segs.as_array() else {
        return out;
    };
    for seg in items {
        if seg.get("type").and_then(|t| t.as_str()) != Some("image") {
            continue;
        }
        let data = seg.get("data");
        let direct = data.and_then(|d| d.get("url")).and_then(|u| u.as_str());
        let in_file = data
            .and_then(|d| d.get("file"))
            .and_then(|f| f.as_str())
            .filter(|f| f.starts_with("http"));
        if let Some(u) = direct.or(in_file) {
            push_image_url(&mut out, u);
        }
    }
    out
}

fn cq_image_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("[CQ:image,") {
        let after = &rest[pos + "[CQ:image,".len()..];
        let Some(end) = after.find(']') else {
            break;
        };
        for part in after[..end].split(',') {
            let value = part
                .strip_prefix("url=")
                .map(str::to_string)
                .or_else(|| {
                    part.strip_prefix("file=")
                        .filter(|f| f.starts_with("http"))
                        .map(str::to_string)
                });
            if let Some(v) = value {
                push_image_url(&mut out, &cq_unescape(&v));
            }
        }
        rest = &after[end + 1..];
    }
    out
}

fn push_image_url(out: &mut Vec<String>, url: &str) {
    if url.starts_with("http") && !out.iter().any(|u| u == url) {
        out.push(url.to_string());
    }
}

// CQ 码反转义（OneBot 11 约定）：QQ 图床 URL 常带逗号签名参数，
// CQ 码里会被转义成 &#44;。&amp; 放最后（转义顺序的逆）。
fn cq_unescape(s: &str) -> String {
    s.replace("&#44;", ",")
        .replace("&#91;", "[")
        .replace("&#93;", "]")
        .replace("&amp;", "&")
}

// CQ 码字符串里是否 at 了指定 QQ：找 [CQ:at,qq= 前缀，取到 ',' 或 ']' 前的
// 号段比对。at 全体（qq=all）不算 at 机器人。
fn cq_contains_at(text: &str, qq: i64) -> bool {
    let needle = "[CQ:at,qq=";
    let mut rest = text;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        if let Some(end) = after.find(']') {
            let token = after[..end].split(',').next().unwrap_or("");
            if token == qq.to_string() {
                return true;
            }
        }
        rest = after;
    }
    false
}

// message 段的数组形式（消息段 list）里是否 at 了指定 QQ。
// qq 字段兼容数字与字符串两种（不同实现不一致）。
fn segs_contain_at(v: &serde_json::Value, qq: i64) -> bool {
    let Some(segs) = v.as_array() else {
        return false;
    };
    segs.iter().any(|seg| {
        seg.get("type").and_then(|t| t.as_str()) == Some("at")
            && seg
                .get("data")
                .and_then(|d| d.get("qq"))
                .and_then(|q| {
                    q.as_i64()
                        .or_else(|| q.as_str().and_then(|s| s.parse().ok()))
                })
                == Some(qq)
    })
}

// 从事件 JSON 里取出 MVP 关心的字段。意外缺失走日志（清单 26101801 #2）——
// OneBot 出新类型或字段变化时消息不再无声消失；meta/notice/request 与
// 机器人自己的消息按约定静默忽略，不打日志。
pub fn parse_incoming(payload: &[u8]) -> Option<Incoming> {
    let raw = || console::brief(&String::from_utf8_lossy(payload), 200);
    let v: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(e) => {
            log_error!("事件 JSON 解析失败，已丢弃: {e}; 原文: {}", raw());
            return None;
        }
    };
    if v.get("post_type")?.as_str()? != "message" {
        return None;
    }
    let message_type = match v.get("message_type").and_then(|s| s.as_str()) {
        Some(m) => m.to_string(),
        None => {
            log_error!("事件缺 message_type，已丢弃; 原文: {}", raw());
            return None;
        }
    };
    let self_id = v.get("self_id").and_then(|s| s.as_i64()).unwrap_or(0);
    let (user_id, chat_id) = match message_type.as_str() {
        "private" => match v.get("user_id").and_then(|u| u.as_i64()) {
            Some(u) => (u, u),
            None => {
                log_error!("私聊事件缺 user_id，已丢弃; 原文: {}", raw());
                return None;
            }
        },
        "group" => {
            let user = v
                .get("sender")
                .and_then(|s| s.get("user_id"))
                .and_then(|u| u.as_i64());
            let group = v.get("group_id").and_then(|g| g.as_i64());
            match (user, group) {
                (Some(u), Some(g)) => (u, g),
                (None, _) => {
                    log_error!("群事件缺 sender.user_id，已丢弃; 原文: {}", raw());
                    return None;
                }
                (_, None) => {
                    log_error!("群事件缺 group_id，已丢弃; 原文: {}", raw());
                    return None;
                }
            }
        }
        other => {
            log_error!("未知的 message_type={other:?}，已丢弃; 原文: {}", raw());
            return None;
        }
    };
    if user_id == self_id {
        return None; // 机器人自己的消息，丢弃防自问自答
    }
    let text = match v.get("raw_message").and_then(|t| t.as_str()) {
        Some(t) => t.to_string(),
        None => {
            log_error!("事件缺 raw_message，已丢弃; 原文: {}", raw());
            return None;
        }
    };
    // at 判定走 message 段正规解析：数组（消息段 list）与 CQ 码字符串两种
    // 格式都认，字段缺失退化用 raw_message。校验 qq == self_id，
    // at 别人/全体不误触发（清单 26102201 #4）
    let is_at_bot = match v.get("message") {
        Some(m) if m.is_array() => segs_contain_at(m, self_id),
        Some(m) if m.is_string() => cq_contains_at(m.as_str().unwrap_or(""), self_id),
        _ => cq_contains_at(&text, self_id),
    };
    Some(Incoming {
        message_type,
        chat_id,
        user_id,
        self_id,
        message_id: v.get("message_id").and_then(|m| m.as_i64()).unwrap_or(0),
        is_at_bot,
        images: extract_images(&v, &text),
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cq_at_matches_exact_qq() {
        assert!(cq_contains_at("[CQ:at,qq=100] 早", 100));
        assert!(cq_contains_at("看这个[CQ:at,qq=100,id=2]哦", 100));
        assert!(!cq_contains_at("[CQ:at,qq=100] 早", 7));
        // at 全体不算 at 机器人
        assert!(!cq_contains_at("[CQ:at,qq=all] 早", 100));
        // 号段前后缀相似不误匹配（如 qq=1001 不算 100）
        assert!(!cq_contains_at("[CQ:at,qq=1001] 早", 100));
        assert!(!cq_contains_at("无码纯文本", 100));
    }

    #[test]
    fn segs_at_matches_number_and_string_qq() {
        let at_num = serde_json::json!([
            {"type": "text", "data": {"text": "早"}},
            {"type": "at", "data": {"qq": 100}}
        ]);
        let at_str = serde_json::json!([
            {"type": "at", "data": {"qq": "100"}}
        ]);
        let at_other = serde_json::json!([
            {"type": "at", "data": {"qq": 7}}
        ]);
        let at_all = serde_json::json!([
            {"type": "at", "data": {"qq": "all"}}
        ]);
        assert!(segs_contain_at(&at_num, 100));
        assert!(segs_contain_at(&at_str, 100));
        assert!(!segs_contain_at(&at_other, 100));
        assert!(!segs_contain_at(&at_all, 100));
    }

    fn group_event(message: serde_json::Value) -> Vec<u8> {
        serde_json::json!({
            "post_type": "message",
            "message_type": "group",
            "self_id": 100,
            "user_id": 7,
            "sender": {"user_id": 7},
            "group_id": 9,
            "message_id": 123,
            "message": message,
            "raw_message": "[CQ:at,qq=100] hi"
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn parse_incoming_detects_at_bot_both_formats() {
        // CQ 码字符串格式
        let inc =
            parse_incoming(&group_event(serde_json::json!("[CQ:at,qq=100] hi")))
                .expect("合法事件应解析成功");
        assert!(inc.is_at_bot);
        assert_eq!(inc.message_id, 123);

        // 数组格式
        let inc = parse_incoming(&group_event(serde_json::json!([
            {"type": "at", "data": {"qq": 100}},
            {"type": "text", "data": {"text": "hi"}}
        ])))
        .expect("合法事件应解析成功");
        assert!(inc.is_at_bot);

        // at 别人：不误触发，但事件本体照常下发
        let inc = parse_incoming(&group_event(serde_json::json!("[CQ:at,qq=7] hi")))
            .expect("合法事件应解析成功");
        assert!(!inc.is_at_bot);
    }

    #[test]
    fn parse_incoming_drops_self_messages() {
        let mut payload = group_event(serde_json::json!("[CQ:at,qq=100] hi"));
        let mut v: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        v["user_id"] = serde_json::json!(100);
        v["sender"]["user_id"] = serde_json::json!(100);
        payload = v.to_string().into_bytes();
        assert!(parse_incoming(&payload).is_none(), "机器人自己的消息应丢弃");
    }

    #[test]
    fn images_extracted_from_array_format() {
        // 数组格式：data.url 优先，file 为 http 链接时兜底；非 image 段忽略
        let inc = parse_incoming(&group_event(serde_json::json!([
            {"type": "text", "data": {"text": "看图"}},
            {"type": "image", "data": {"file": "abc.image", "url": "https://qq.com/a.jpg"}},
            {"type": "image", "data": {"file": "https://qq.com/b.jpg"}},
            {"type": "face", "data": {"id": "1"}}
        ])))
        .expect("合法事件应解析成功");
        assert_eq!(
            inc.images,
            vec!["https://qq.com/a.jpg".to_string(), "https://qq.com/b.jpg".to_string()]
        );
    }

    #[test]
    fn images_extracted_from_cq_and_unescaped() {
        // CQ 码格式（raw_message 携带）：url= 的 CQ 转义（&#44;）要还原
        let cq = "[CQ:image,subType=0,url=https://g.qq.com/a?term=2&#44;x&amp;y]看";
        let payload = serde_json::json!({
            "post_type": "message",
            "message_type": "group",
            "self_id": 100,
            "user_id": 7,
            "sender": {"user_id": 7},
            "group_id": 9,
            "message_id": 123,
            "message": cq,
            "raw_message": cq
        })
        .to_string()
        .into_bytes();
        let inc = parse_incoming(&payload).expect("合法事件应解析成功");
        assert_eq!(inc.images, vec!["https://g.qq.com/a?term=2,x&y".to_string()]);
    }

    #[test]
    fn images_deduped_and_capped() {
        // 6 张去重后 5 张，截到上限 4；纯文本消息为空列表
        let segs: Vec<_> = (1..=6u8)
            .map(|i| {
                let url = if i == 5 { "https://q/1.jpg" } else { "" };
                let url = if url.is_empty() {
                    format!("https://q/{i}.jpg")
                } else {
                    url.to_string()
                };
                serde_json::json!({"type": "image", "data": {"url": url}})
            })
            .collect();
        let inc = parse_incoming(&group_event(serde_json::json!(segs)))
            .expect("合法事件应解析成功");
        assert_eq!(inc.images.len(), MAX_IMAGES);
        assert_eq!(inc.images[0], "https://q/1.jpg");
        assert_eq!(inc.images[1], "https://q/2.jpg");

        let inc = parse_incoming(&group_event(serde_json::json!("[CQ:at,qq=100] hi")))
            .expect("合法事件应解析成功");
        assert!(inc.images.is_empty(), "纯文本消息不应有图片");
    }
}
