//! 管线统计：事件/回复计数、worker 积压与累计、per-chat 积压，
//! 以及可观测性最小集（清单 26102701 建议 8）：分阶段延迟（parse/
//! chain/send 的 p50/p95/p99）、主队列占用、worker 侧丢弃计数。
//! agents / status 控制台命令与 /api/status 的数据源。
//!
//! 统计口径非记账：计数丢误差可接受，chat 表用一把 Mutex 而不分片。

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use serde_json::json;

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

// ---- 可观测性最小集（26102701 建议 8）----

/// 主队列占用：无界 mpsc 看不到长度，回调入队 +1、主循环取出 -1。
/// 只计事件（控制台命令量可忽略）；数字只在增长时才有意义。
static MAIN_BACKLOG: AtomicUsize = AtomicUsize::new(0);

pub fn main_enqueue() {
    MAIN_BACKLOG.fetch_add(1, Ordering::Relaxed);
}

pub fn main_dequeue() {
    MAIN_BACKLOG.fetch_sub(1, Ordering::Relaxed);
}

pub fn main_backlog() -> usize {
    MAIN_BACKLOG.load(Ordering::Relaxed)
}

/// worker 队列满被丢的消息数（worker 层丢弃；adapter/core 层在 Go 侧）。
static WORKER_DROPPED: AtomicU64 = AtomicU64::new(0);

pub fn bump_worker_dropped() {
    WORKER_DROPPED.fetch_add(1, Ordering::Relaxed);
}

pub fn worker_dropped() -> u64 {
    WORKER_DROPPED.load(Ordering::Relaxed)
}

/// 分阶段延迟：环形缓冲记最近 N 次耗时（微秒），读时排序取分位。
/// 锁内是一次 push/pop，worker 间争用可忽略；不上直方图库，
/// 256 样本对诊断足够（评审：只加最小集，不要 OpenTelemetry）。
const LATENCY_CAP: usize = 256;

pub struct Latency {
    ring: Mutex<VecDeque<u64>>,
}

// 三个阶段各一个实例：parse 解析信封 / chain 插件责任链 / send 出站发送。
static PARSE: Latency = Latency { ring: Mutex::new(VecDeque::new()) };
static CHAIN: Latency = Latency { ring: Mutex::new(VecDeque::new()) };
static SEND: Latency = Latency { ring: Mutex::new(VecDeque::new()) };

impl Latency {
    fn record(&self, us: u64) {
        if let Ok(mut r) = self.ring.lock() {
            if r.len() == LATENCY_CAP {
                r.pop_front();
            }
            r.push_back(us);
        }
    }

    /// 分位值（ms）。n=样本数；样本不足时该分位为 null。
    fn report(&self) -> serde_json::Value {
        let Ok(r) = self.ring.lock() else {
            return json!({ "n": 0 });
        };
        let mut v: Vec<u64> = r.iter().copied().collect();
        v.sort_unstable();
        let pct = |p: f64| -> serde_json::Value {
            if v.is_empty() {
                return serde_json::Value::Null;
            }
            let idx = ((v.len() - 1) as f64 * p).round() as usize;
            json!((v[idx] as f64 / 1000.0))
        };
        json!({ "p50": pct(0.50), "p95": pct(0.95), "p99": pct(0.99), "n": v.len() })
    }
}

pub fn record_parse(d: std::time::Duration) {
    PARSE.record(d.as_micros() as u64);
}

pub fn record_chain(d: std::time::Duration) {
    CHAIN.record(d.as_micros() as u64);
}

pub fn record_send(d: std::time::Duration) {
    SEND.record(d.as_micros() as u64);
}

/// 三阶段延迟报告（ms），进 /api/status。
pub fn latency_report() -> serde_json::Value {
    json!({
        "parse": PARSE.report(),
        "chain": CHAIN.report(),
        "send": SEND.report(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_percentiles_and_ring_wrap() {
        let l = Latency { ring: Mutex::new(VecDeque::new()) };
        // 空报告：n=0，分位为 null
        let rep = l.report();
        assert_eq!(rep["n"], 0);
        assert!(rep["p50"].is_null());

        // 1..=100ms 各记一次：最近邻秩（v[round(p*(n-1))])下 p50=v[50]=51、
        // p95=v[94]=95、p99=v[98]=99
        for ms in 1..=100u64 {
            l.record(ms * 1000);
        }
        let rep = l.report();
        assert_eq!(rep["n"], 100);
        assert_eq!(rep["p50"], 51.0);
        assert_eq!(rep["p95"], 95.0);
        assert_eq!(rep["p99"], 99.0);

        // 超容量：只留最近 LATENCY_CAP 个样本
        for ms in 1..=(LATENCY_CAP as u64 + 50) {
            l.record(ms * 1000);
        }
        assert_eq!(l.report()["n"], LATENCY_CAP);
    }
}
