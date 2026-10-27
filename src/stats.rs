//! 管线统计：事件/回复计数、worker 积压与累计、per-chat 积压。
//! agents / status 控制台命令的数据源（src/console.rs）。
//!
//! 统计口径非记账：计数丢误差可接受，chat 表用一把 Mutex 而不分片。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

// worker 池参数：事件处理挪出主循环，慢回复（LLM 最长 60s）不再阻塞
// 其他聊天与控制台命令（清单 26101801 #1）。
// chat_id hash 定 worker：同一会话恒在同一线程串行（插件模块级状态免锁），
// 跨会话并行。并行度来自 LLM/HTTP 等待（socket 等待自动放 GIL）。
pub const WORKER_COUNT: usize = 4;
// 每 worker 的有界队列：积压超限丢消息并记日志（与 Go 核心队列同语义）。
pub const WORKER_QUEUE_CAP: usize = 64;

static EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static REPLY_COUNT: AtomicU64 = AtomicU64::new(0);
static WORKER_BACKLOG: [AtomicUsize; WORKER_COUNT] =
    [const { AtomicUsize::new(0) }; WORKER_COUNT];
static WORKER_DONE: [AtomicU64; WORKER_COUNT] = [const { AtomicU64::new(0) }; WORKER_COUNT];
static CHAT_STATS: OnceLock<Mutex<HashMap<String, ChatStat>>> = OnceLock::new();

struct ChatStat {
    backlog: usize,
    total: u64,
}

pub fn bump_event() {
    EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn bump_reply() {
    REPLY_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn event_count() -> u64 {
    EVENT_COUNT.load(Ordering::Relaxed)
}

pub fn reply_count() -> u64 {
    REPLY_COUNT.load(Ordering::Relaxed)
}

// 事件入队成功后记一笔（worker 处理完由 worker_settle 归还）。
pub fn worker_enqueue(idx: usize, chat_id: &str) {
    WORKER_BACKLOG[idx].fetch_add(1, Ordering::Relaxed);
    let map = CHAT_STATS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut m) = map.lock() {
        let st = m.entry(chat_id.to_string()).or_insert(ChatStat { backlog: 0, total: 0 });
        st.backlog += 1;
        st.total += 1;
    }
}

// worker 处理完一条（无论 panic 与否）统一归还，统计口径才守得住。
pub fn worker_settle(idx: usize, chat_id: &str) {
    WORKER_BACKLOG[idx].fetch_sub(1, Ordering::Relaxed);
    WORKER_DONE[idx].fetch_add(1, Ordering::Relaxed);
    if let Some(map) = CHAT_STATS.get() {
        if let Ok(mut m) = map.lock() {
            if let Some(st) = m.get_mut(chat_id) {
                st.backlog = st.backlog.saturating_sub(1);
            }
        }
    }
}

pub fn worker_backlogs() -> Vec<usize> {
    WORKER_BACKLOG.iter().map(|c| c.load(Ordering::Relaxed)).collect()
}

pub fn worker_done() -> Vec<u64> {
    WORKER_DONE.iter().map(|c| c.load(Ordering::Relaxed)).collect()
}

// 积压中的会话（正在处理或排队），agents 命令展示用。
pub fn active_chats() -> Vec<String> {
    let Some(map) = CHAT_STATS.get() else {
        return Vec::new();
    };
    match map.lock() {
        Ok(m) => m.iter().filter(|(_, st)| st.backlog > 0).map(|(id, _)| id.clone()).collect(),
        Err(_) => Vec::new(),
    }
}
