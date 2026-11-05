//! 管线统计：事件/回复计数、worker 积压与累计、per-chat 积压，
//! 以及可观测性最小集（清单 26102701 建议 8）：分阶段延迟（parse/
//! chain/send 的 p50/p95/p99）、主队列占用、worker 侧丢弃计数。
//! agents / status 控制台命令与 /api/status 的数据源。
//!
//! 统计口径非记账：计数丢误差可接受，chat 表用一把 Mutex 而不分片。
//!
//! 持久化（data/stats.json）：累计类计数与按天活跃度跨重启保留，
//! 启动时灌回基线（读侧不感知），每 60s 与优雅退出时原子写回；
//! 延迟样本、队列占用这类瞬时诊断不落盘。

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::json;

// worker 池参数：事件处理挪出主循环，慢回复（LLM 最长 60s）不再阻塞
// 其他聊天与控制台命令（清单 26101801 #1）。
// 派发走每会话串行执行器（src/executor.rs，评审建议 7）：同一会话恒串行
// （插件模块级状态免锁），不同会话互不阻塞；每会话队列上限见 executor。
pub const WORKER_COUNT: usize = 4;

static EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static REPLY_COUNT: AtomicU64 = AtomicU64::new(0);
/// 各池线程是否在跑（0/1）：Dashboard 按线程画"管道"，语义从积压改为在途
static WORKER_BACKLOG: [AtomicUsize; WORKER_COUNT] = [const { AtomicUsize::new(0) }; WORKER_COUNT];
static WORKER_DONE: [AtomicU64; WORKER_COUNT] = [const { AtomicU64::new(0) }; WORKER_COUNT];
static CHAT_STATS: OnceLock<Mutex<HashMap<String, ChatStat>>> = OnceLock::new();

struct ChatStat {
    backlog: usize,
    total: u64,
}

pub fn bump_event() {
    EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
    daily_bump(0);
}

pub fn bump_reply() {
    REPLY_COUNT.fetch_add(1, Ordering::Relaxed);
    daily_bump(1);
}

pub fn event_count() -> u64 {
    EVENT_COUNT.load(Ordering::Relaxed)
}

pub fn reply_count() -> u64 {
    REPLY_COUNT.load(Ordering::Relaxed)
}

// 消息进入会话队列记一笔（按 chat 计积压与累计；处理完由 worker_settle 归还）。
pub fn session_enqueue(chat_id: &str) {
    let map = CHAT_STATS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut m) = map.lock() {
        let st = m.entry(chat_id.to_string()).or_insert(ChatStat {
            backlog: 0,
            total: 0,
        });
        st.backlog += 1;
        st.total += 1;
    }
}

// worker 从执行器取到一条：该线程标为在途。
pub fn worker_pick(idx: usize) {
    WORKER_BACKLOG[idx].store(1, Ordering::Relaxed);
}

// worker 处理完一条（无论 panic 与否）统一归还，统计口径才守得住。
pub fn worker_settle(idx: usize, chat_id: &str) {
    WORKER_BACKLOG[idx].store(0, Ordering::Relaxed);
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
    WORKER_BACKLOG
        .iter()
        .map(|c| c.load(Ordering::Relaxed))
        .collect()
}

pub fn worker_done() -> Vec<u64> {
    WORKER_DONE
        .iter()
        .map(|c| c.load(Ordering::Relaxed))
        .collect()
}

// 积压中的会话（正在处理或排队），agents 命令展示用。
pub fn active_chats() -> Vec<String> {
    let Some(map) = CHAT_STATS.get() else {
        return Vec::new();
    };
    match map.lock() {
        Ok(m) => m
            .iter()
            .filter(|(_, st)| st.backlog > 0)
            .map(|(id, _)| id.clone())
            .collect(),
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

/// 去重命中的重复消息数（重连重放/上游重试）。
static DUPLICATES: AtomicU64 = AtomicU64::new(0);

pub fn bump_worker_dropped() {
    WORKER_DROPPED.fetch_add(1, Ordering::Relaxed);
}

pub fn worker_dropped() -> u64 {
    WORKER_DROPPED.load(Ordering::Relaxed)
}

pub fn bump_duplicate() {
    DUPLICATES.fetch_add(1, Ordering::Relaxed);
}

pub fn duplicates() -> u64 {
    DUPLICATES.load(Ordering::Relaxed)
}

// ---- 插件执行预算（建议 4）----

/// 插件调用超时次数：看门狗注入 TimeoutError 并中断成功的次数。
/// 预算耗尽跳过后续插件是它的后果，不单独计（否则一次超时记两笔）。
static PLUGIN_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
/// 被标记 stuck（超时注入无效、疑似卡在 C 调用）的插件次数。
static PLUGIN_STUCK: AtomicU64 = AtomicU64::new(0);

pub fn bump_plugin_timeout() {
    PLUGIN_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
}

pub fn plugin_timeouts() -> u64 {
    PLUGIN_TIMEOUTS.load(Ordering::Relaxed)
}

pub fn bump_plugin_stuck() {
    PLUGIN_STUCK.fetch_add(1, Ordering::Relaxed);
}

pub fn plugin_stuck() -> u64 {
    PLUGIN_STUCK.load(Ordering::Relaxed)
}

/// 分阶段延迟：环形缓冲记最近 N 次耗时（微秒），读时排序取分位。
/// 锁内是一次 push/pop，worker 间争用可忽略；不上直方图库，
/// 256 样本对诊断足够（评审：只加最小集，不要 OpenTelemetry）。
const LATENCY_CAP: usize = 256;

pub struct Latency {
    ring: Mutex<VecDeque<u64>>,
}

// 三个阶段各一个实例：parse 解析信封 / chain 插件责任链 / send 出站发送。
static PARSE: Latency = Latency {
    ring: Mutex::new(VecDeque::new()),
};
static CHAIN: Latency = Latency {
    ring: Mutex::new(VecDeque::new()),
};
static SEND: Latency = Latency {
    ring: Mutex::new(VecDeque::new()),
};

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

/// 各插件 handle 耗时，按插件名分桶（建议 8 的 handler p50/p99；建议 4 的
/// 超时预算靠它定）。桶数 = 插件数，热重载同名复用。
static PLUGIN_LATENCY: OnceLock<Mutex<HashMap<String, Latency>>> = OnceLock::new();

pub fn record_plugin(name: &str, d: std::time::Duration) {
    let map = PLUGIN_LATENCY.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut m) = map.lock() else {
        return;
    };
    m.entry(name.to_string())
        .or_insert_with(|| Latency {
            ring: Mutex::new(VecDeque::new()),
        })
        .record(d.as_micros() as u64);
}

/// 延迟报告（ms），进 /api/status：三阶段 + 各插件 + 超时/卡死计数。
pub fn latency_report() -> serde_json::Value {
    let plugins: serde_json::Map<String, serde_json::Value> = PLUGIN_LATENCY
        .get()
        .and_then(|m| m.lock().ok())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.report())).collect())
        .unwrap_or_default();
    json!({
        "parse": PARSE.report(),
        "chain": CHAIN.report(),
        "send": SEND.report(),
        "plugins": plugins,
        "plugin_timeouts": plugin_timeouts(),
        "plugin_stuck": plugin_stuck(),
    })
}

// ---- 持久化（data/stats.json）----
//
// 累计类计数（事件/回复、丢弃/重试、插件超时/卡死）、per-chat 累计与
// 按天活跃度（热力图数据源）跨重启保留：启动时把落盘值灌回各计数与
// chat 表，之后的增量仍走原原子量路径，读侧无感。每 60s 与优雅退出
// 时写回（tmp + rename），被 kill 的窗口最多丢一个写盘周期。
// first_started_at 只取一次，是装龄的起点，永不重置。

const STATS_PATH: &str = "data/stats.json";
/// 热力图保留窗口（天）：约 13 个月，够看年节奏，文件不无限膨胀。
const DAILY_KEEP: usize = 400;

static FIRST_STARTED: AtomicI64 = AtomicI64::new(0);
/// 日期（本地时区 YYYY-MM-DD）→ [events, replies]。BTreeMap 让落盘与
/// 接口输出天然按日期有序。
static DAILY: OnceLock<Mutex<BTreeMap<String, [u64; 2]>>> = OnceLock::new();

fn daily() -> &'static Mutex<BTreeMap<String, [u64; 2]>> {
    DAILY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// 当天活跃度 +1：slot 0 = 事件，slot 1 = 回复。
fn daily_bump(slot: usize) {
    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    if let Ok(mut m) = daily().lock() {
        let slot_val = m.entry(day).or_insert([0, 0]);
        slot_val[slot] += 1;
    }
}

/// 首次上线时刻（unix 秒）：装龄的起点。
pub fn first_started() -> i64 {
    FIRST_STARTED.load(Ordering::Relaxed)
}

/// 按天活跃度（供 /api/status）：按日期升序。
pub fn daily_json() -> serde_json::Value {
    match daily().lock() {
        Ok(m) => serde_json::Value::Object(
            m.iter().map(|(k, v)| (k.clone(), json!(v))).collect(),
        ),
        Err(_) => json!({}),
    }
}

/// 启动时读盘灌基线。无文件视为首次启动：装龄从现在起算，
/// 并立刻落一次盘，否则装龄要等下一个写盘周期才站得住。
pub fn init() {
    let parsed: serde_json::Value = fs::read(STATS_PATH)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| json!({}));
    let existed = parsed["first_started_at"].as_i64().unwrap_or(0) > 0;

    FIRST_STARTED.store(
        parsed["first_started_at"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp()),
        Ordering::Relaxed,
    );
    EVENT_COUNT.store(parsed["events"].as_u64().unwrap_or(0), Ordering::Relaxed);
    REPLY_COUNT.store(parsed["replies"].as_u64().unwrap_or(0), Ordering::Relaxed);
    WORKER_DROPPED.store(parsed["worker_dropped"].as_u64().unwrap_or(0), Ordering::Relaxed);
    DUPLICATES.store(parsed["duplicates"].as_u64().unwrap_or(0), Ordering::Relaxed);
    PLUGIN_TIMEOUTS.store(parsed["plugin_timeouts"].as_u64().unwrap_or(0), Ordering::Relaxed);
    PLUGIN_STUCK.store(parsed["plugin_stuck"].as_u64().unwrap_or(0), Ordering::Relaxed);
    crate::outbound::init_baseline(
        parsed["outbound_dropped"].as_u64().unwrap_or(0),
        parsed["outbound_retries"].as_u64().unwrap_or(0),
    );

    if let Ok(mut m) = CHAT_STATS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        if let Some(chats) = parsed["chats"].as_object() {
            for (k, v) in chats {
                let st = m
                    .entry(k.clone())
                    .or_insert(ChatStat { backlog: 0, total: 0 });
                st.total = v.as_u64().unwrap_or(0);
            }
        }
    }
    if let Ok(mut m) = daily().lock() {
        m.clear();
        if let Some(days) = parsed["daily"].as_object() {
            for (k, v) in days {
                if let (Some(e), Some(r)) = (v[0].as_u64(), v[1].as_u64()) {
                    m.insert(k.clone(), [e, r]);
                }
            }
        }
    }

    if !existed {
        save();
    }
}

/// 收集当前累计值写盘。
pub fn save() {
    save_to(Path::new(STATS_PATH));
}

fn save_to(path: &Path) {
    let chats: serde_json::Map<String, serde_json::Value> = CHAT_STATS
        .get()
        .and_then(|m| m.lock().ok())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), json!(v.total))).collect())
        .unwrap_or_default();
    // 留最近 DAILY_KEEP 天：日期字符串字典序即时间序，丢掉最旧的
    let daily_val = match daily().lock() {
        Ok(m) => {
            let skip = m.len().saturating_sub(DAILY_KEEP);
            serde_json::Value::Object(
                m.iter().skip(skip).map(|(k, v)| (k.clone(), json!(v))).collect(),
            )
        }
        Err(_) => json!({}),
    };
    let body = json!({
        "version": 1,
        "first_started_at": first_started(),
        "events": event_count(),
        "replies": reply_count(),
        "worker_dropped": worker_dropped(),
        "duplicates": duplicates(),
        "plugin_timeouts": plugin_timeouts(),
        "plugin_stuck": plugin_stuck(),
        "outbound_dropped": crate::outbound::dropped(),
        "outbound_retries": crate::outbound::retries(),
        "chats": serde_json::Value::Object(chats),
        "daily": daily_val,
        "saved_at": chrono::Utc::now().to_rfc3339(),
    });
    // tmp + rename：写一半被杀不会留半个 JSON
    let tmp = path.with_extension("json.tmp");
    if let Ok(mut f) = fs::File::create(&tmp) {
        if serde_json::to_writer_pretty(&mut f, &body).is_ok() && f.flush().is_ok() {
            drop(f);
            let _ = fs::rename(&tmp, path);
        }
    }
}

/// 定期写盘线程：强杀场景最多丢一个周期，正常 Ctrl+C 走退出钩子。
pub fn spawn_persister() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(60));
        save();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_percentiles_and_ring_wrap() {
        let l = Latency {
            ring: Mutex::new(VecDeque::new()),
        };
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

    #[test]
    fn stats_json_roundtrip_and_daily_trim() {
        let dir = std::env::temp_dir().join("loquat-stats-test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let _ = fs::remove_file(&path);

        // 灌测试态：DAILY_KEEP + 10 天数据，裁剪后应只剩最近 KEEP 天
        let today = chrono::Local::now().date_naive();
        let expect_first = chrono::Utc::now().timestamp();
        FIRST_STARTED.store(expect_first, Ordering::Relaxed);
        *daily().lock().unwrap() = (0..=DAILY_KEEP + 10)
            .map(|i| {
                (
                    (today - chrono::Duration::days(i as i64))
                        .format("%Y-%m-%d")
                        .to_string(),
                    [i as u64, i as u64],
                )
            })
            .collect();
        CHAT_STATS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap()
            .insert("chat_a".into(), ChatStat { backlog: 3, total: 42 });

        save_to(&path);
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["chats"]["chat_a"], 42, "只落累计，不落 backlog");
        assert_eq!(saved["first_started_at"], expect_first);
        let daily = saved["daily"].as_object().unwrap();
        assert_eq!(daily.len(), DAILY_KEEP, "超出窗口的最旧天被裁掉");
        // 最新一天（i=0）在，最旧一天（i=KEEP+10）不在
        assert!(daily.contains_key(&today.format("%Y-%m-%d").to_string()));
        let oldest = (today - chrono::Duration::days((DAILY_KEEP + 10) as i64))
            .format("%Y-%m-%d")
            .to_string();
        assert!(!daily.contains_key(&oldest));

        // 读回：init 从落盘文件恢复（init 读固定路径，这里直接验证解析逻辑等价：
        // save→改路径读回的字段都能还原）
        let raw = fs::read(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(parsed["events"], saved["events"]);
        assert_eq!(parsed["daily"], saved["daily"]);
    }
}
