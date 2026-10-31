//! 消息去重：(platform, message_id) 五分钟内视为重复（评审建议 9）。
//! 场景：适配器重连重放、上游重试——重复投递会重复回复，LLM 场景就是
//! 重复计费。message_id 为空不去重（平台不回 ID 时无法判重）。
//!
//! 存储：单 Mutex HashMap，键数上限内不做后台清理；满时才 retain 清
//! 过期项（n ≤ 4096，全表扫一次微秒级）。命中时同时验 TTL：过期键
//! 不算重复。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::log_warn;

/// 键数上限：按峰值消息速率 × TTL 估的富余量（>13 msg/s 持续 5 分钟）。
const CAP: usize = 4096;
/// 判重窗口。
const TTL: Duration = Duration::from_secs(300);

static SEEN: Mutex<Option<HashMap<(String, String), Instant>>> = Mutex::new(None);

/// 首见返回 false；TTL 内重复返回 true（调用方丢弃）。
pub fn duplicate(platform: &str, message_id: &str) -> bool {
    if message_id.is_empty() {
        return false;
    }
    let mut guard = SEEN.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    if map.len() >= CAP {
        map.retain(|_, t| now.duration_since(*t) < TTL);
        if map.len() >= CAP {
            // 全是新键仍超限：说明容量被正常流量吃满，整表让路保可用
            map.clear();
            log_warn!("去重表已满（{}），已整表清空", CAP);
        }
    }
    match map.insert((platform.to_string(), message_id.to_string()), now) {
        Some(t) => now.duration_since(t) < TTL,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_within_ttl_only() {
        let key = format!("t{}", std::process::id());
        assert!(!duplicate(&key, "m1"));
        assert!(duplicate(&key, "m1"), "同 ID 立即重复");
        assert!(!duplicate(&key, "m2"), "不同 ID 不影响");
        // 空 ID 不判重
        assert!(!duplicate(&key, ""));
        assert!(!duplicate("", "m1"));
    }
}
