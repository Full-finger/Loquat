//! 出站发送队列（评审建议 2）：worker 只产出回复，发送交给 sender 池。
//!
//! - worker 不再被 15s 发送超时占住，慢发送不拖累其他会话的处理
//! - 同 chat 恒在同 sender（与 worker 同款 chat_id 哈希），会话内顺序保持
//! - 每平台最小间隔限流（防触发平台风控）；频控（retcode=1200）与
//!   网络错误有限重试，指数退避加抖动
//! - 逃生舱（pybridge send_action 的 send_* 类动作）共享限流闸：
//!   旁路受控，同步应答语义保留
//!
//! 出站幂等键（评审里的 client_msg_id）：OneBot 11 不支持自定义消息
//! ID，协议级做不了；重复源头已由入站去重消除，此处不再建模。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::business::Message;
use crate::console;
use crate::event::Chat;
use crate::ffi;
use crate::ffi::CoreDll;
use crate::{log_error, log_info, stats, webui};

/// 每 sender 的出站队列上限：满即丢（与全链路丢弃语义一致），有计数。
const QUEUE_CAP: usize = 256;
/// sender 线程数：按 chat 哈希分流，同 chat 恒在同一线程保序。
pub(crate) const SENDERS: usize = 2;
/// 每平台发送最小间隔：防触发 NapCat/QQ 风控。
const MIN_INTERVAL: Duration = Duration::from_millis(300);
/// 发送超时：对齐原 worker 内同步发送的口径。
const SEND_TIMEOUT_MS: i32 = 15_000;
/// 可重试失败的最大重试次数与退避基值（<< attempt 指数 + 亚秒抖动）。
const RETRY_MAX: u32 = 2;
const RETRY_BASE_MS: u64 = 500;

pub(crate) struct OutboundMsg {
    platform: String,
    chat_kind: &'static str,
    chat_id: String,
    /// 一条消息的扁平段列表（建议 5）：原样序列化进 SendMessageRequest.segments
    segments: Message,
}

static QUEUE_TX: OnceLock<Vec<mpsc::SyncSender<OutboundMsg>>> = OnceLock::new();
static LAST_SEND: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn last_send() -> &'static Mutex<HashMap<String, Instant>> {
    LAST_SEND.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn dropped() -> u64 {
    OUTBOUND_DROPPED.load(Ordering::Relaxed)
}

pub(crate) fn retries() -> u64 {
    SEND_RETRIES.load(Ordering::Relaxed)
}

/// 启动时灌入上次落盘的基线（stats 持久化）：计数从历史值续走。
pub(crate) fn init_baseline(dropped: u64, retries: u64) {
    OUTBOUND_DROPPED.store(dropped, Ordering::Relaxed);
    SEND_RETRIES.store(retries, Ordering::Relaxed);
}

static OUTBOUND_DROPPED: AtomicU64 = AtomicU64::new(0);
static SEND_RETRIES: AtomicU64 = AtomicU64::new(0);

// 与 worker 选路同款：chat_id 哈希。同 chat 的回复由同 worker 依序产出、
// 恒落入同 sender 队列，两段都是 FIFO，会话内顺序成立。
fn sender_of(chat_id: &str) -> usize {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    chat_id.hash(&mut h);
    (h.finish() % SENDERS as u64) as usize
}

/// worker 产出回复后入队；满则丢弃并计数（不阻塞 worker）。
pub(crate) fn enqueue(platform: &str, chat: &Chat, segments: Message) {
    let (chat_kind, chat_id) = match chat {
        Chat::Private { id } => ("private", id.as_str()),
        Chat::Group { id } => ("group", id.as_str()),
    };
    let msg = OutboundMsg {
        platform: platform.to_string(),
        chat_kind,
        chat_id: chat_id.to_string(),
        segments,
    };
    let sent = QUEUE_TX
        .get()
        .and_then(|txs| txs.get(sender_of(&msg.chat_id)))
        .map(|tx| tx.try_send(msg).is_ok());
    match sent {
        Some(true) => {}
        Some(false) => {
            OUTBOUND_DROPPED.fetch_add(1, Ordering::Relaxed);
            log_error!("出站队列已满，丢弃回复（chat={chat_id}）");
        }
        None => log_error!("出站队列未初始化，丢弃回复（chat={chat_id}）"),
    }
}

/// 起 sender 池。main 在 worker 池之前调用一次。
pub(crate) fn spawn_senders(dll: Arc<CoreDll>) {
    let mut txs = Vec::with_capacity(SENDERS);
    let mut rxs = Vec::with_capacity(SENDERS);
    for _ in 0..SENDERS {
        let (tx, rx) = mpsc::sync_channel::<OutboundMsg>(QUEUE_CAP);
        txs.push(tx);
        rxs.push(rx);
    }
    let _ = QUEUE_TX.set(txs);
    for (idx, rx) in rxs.into_iter().enumerate() {
        let dll = Arc::clone(&dll);
        std::thread::spawn(move || {
            let _ = idx; // 线程与队列一一对应，recv 只见自己 chat 桶的消息
            for msg in rx {
                send_with_retry(&dll, msg);
            }
        });
    }
}

/// 每平台发送间隔闸：距上次发送不足 MIN_INTERVAL 则睡到到期。
/// 锁内只算时间，睡在锁外；醒来重查（可能被同平台调用者插队）。
/// 逃生舱的 send_* 类动作也走这里（pybridge 在放掉 GIL 后调用）。
pub(crate) fn throttle(platform: &str) {
    loop {
        let wait = {
            let mut m = last_send().lock().unwrap_or_else(|p| p.into_inner());
            let now = Instant::now();
            let next = m.get(platform).map_or(now, |&t| t + MIN_INTERVAL);
            if next <= now {
                m.insert(platform.to_string(), now);
                return;
            }
            next - now
        };
        std::thread::sleep(wait);
    }
}

fn backoff_sleep(attempt: u32) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    std::thread::sleep(Duration::from_millis(
        (RETRY_BASE_MS << attempt) + nanos % 200,
    ));
}

/// 日志与沙箱记账用的文本投影：有文本取文本（与入站 derive_text 同一投影），
/// 纯媒体/纯引用消息给占位，避免图片消息在日志与沙箱里留空。
pub(crate) fn summary_of(segments: &[serde_json::Value]) -> String {
    let text = crate::event::derive_text(segments);
    if !text.is_empty() {
        return text;
    }
    let kind = segments
        .iter()
        .find_map(|s| s.as_object().and_then(|o| o.keys().next()))
        .map(String::as_str)
        .unwrap_or("");
    match kind {
        "" => "",
        "image" => "[图片]",
        "voice" => "[语音]",
        "video" => "[视频]",
        "file" => "[文件]",
        _ => "[消息]",
    }
    .to_string()
}

fn send_with_retry(dll: &CoreDll, msg: OutboundMsg) {
    let OutboundMsg {
        platform,
        chat_kind,
        chat_id,
        segments,
    } = msg;
    let summary = summary_of(&segments);
    let payload = serde_json::Value::Array(segments).to_string();
    for attempt in 0..=RETRY_MAX {
        throttle(&platform);
        let started = Instant::now();
        let result = dll.send_message(&platform, chat_kind, &chat_id, &payload, SEND_TIMEOUT_MS);
        stats::record_send(started.elapsed());
        match result {
            Ok(resp) => {
                // Go 侧约定：失败装在 {"error": ...} 里（26102701#5）
                if !ffi::resp_failed(&resp) {
                    stats::bump_reply();
                    webui::publish_reply(&platform, chat_kind, &chat_id, &summary);
                    log_info!("已回复: {} resp={resp}", console::brief(&summary, 60));
                    return;
                }
                if attempt < RETRY_MAX && resp.contains("retcode=1200") {
                    SEND_RETRIES.fetch_add(1, Ordering::Relaxed);
                    backoff_sleep(attempt);
                    continue;
                }
                log_error!("发送失败: {}", console::brief(&resp, 200));
                return;
            }
            Err(e) => {
                // 网络错误可重试；其他 FFI 错误重试无益，直接失败
                if attempt < RETRY_MAX {
                    SEND_RETRIES.fetch_add(1, Ordering::Relaxed);
                    backoff_sleep(attempt);
                    continue;
                }
                log_error!("发送失败: {e}");
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_enforces_min_interval_per_platform() {
        let platform = format!("t{}", std::process::id());
        let t0 = Instant::now();
        throttle(&platform);
        throttle(&platform); // 同平台连续两次：第二次应被压到间隔之后
        assert!(t0.elapsed() >= MIN_INTERVAL, "实际 {:?}", t0.elapsed());

        // 不同平台互不影响
        let other = format!("o{}", std::process::id());
        let t1 = Instant::now();
        throttle(&other);
        assert!(t1.elapsed() < MIN_INTERVAL);
    }

    #[test]
    fn sender_of_is_stable() {
        assert_eq!(sender_of("chat-a"), sender_of("chat-a"));
        assert!(sender_of("chat-a") < SENDERS);
    }

    #[test]
    fn summary_projects_text_or_placeholder() {
        use serde_json::json;
        // 有文本：拼全部 text 段，忽略夹在中间的媒体
        let mixed = [
            json!({ "reply": { "messageId": "m1" } }),
            json!({ "text": { "text": "看" } }),
            json!({ "image": { "url": "https://x" } }),
            json!({ "text": { "text": "这个" } }),
        ];
        assert_eq!(summary_of(&mixed), "看这个");
        // 纯媒体：按首段类型给占位，日志与沙箱不留空
        assert_eq!(
            summary_of(&[json!({ "image": { "url": "https://x" } })]),
            "[图片]"
        );
        assert_eq!(
            summary_of(&[json!({ "voice": { "url": "https://x" } })]),
            "[语音]"
        );
        assert_eq!(summary_of(&[json!({ "at": { "target": "1" } })]), "[消息]");
        assert_eq!(summary_of(&[]), "");
    }
}
