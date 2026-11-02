//! 每会话串行执行器（评审建议 7）：取代"chat_id 哈希定 worker + 每 worker 有界队列"。
//!
//! 旧模型的问题是同桶会话互相 head-of-line blocking：一个会话慢（LLM 最长 60s），
//! 哈希到同一 worker 的其他会话全部排队；热门群的爆发能把整桶 64 格挤满，
//! 无辜会话被丢。
//!
//! 新模型：每会话一条队列 + 全局就绪队列。
//!   - 同会话严格 FIFO：会话处于 Running 时它的下一条绝不会被取出，done() 后才回
//!     就绪队列——"顺序保证"从哈希的巧合变成显式契约（插件模块级状态照旧免锁）
//!   - 不同会话互不阻塞：就绪队列按会话轮转，谁有活谁上，worker 池是唯一共享资源
//!   - 每会话独立上限 SESSION_QUEUE_CAP：爆发只影响自己，满了返还给调用方做降级回复
//!   - 排空有界：close() 后 worker 取完即退；wait_drained 超时返回 false，
//!     卡死会话不再让退出永远挂住
//!
//! 会话键 (platform, message_type, chat_id)：与 Python 侧 sessions 的键同构，
//! 私聊 42 与群 42 是两个会话（旧模型按纯 chat_id 哈希把它们捆在一起）。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::json;

use crate::event::Incoming;

/// 每会话待处理上限：超出返还给调用方（丢弃计数 + 私聊降级回复）。
pub(crate) const SESSION_QUEUE_CAP: usize = 16;

pub(crate) type SessionKey = (String, String, String);

#[derive(PartialEq, Eq)]
enum State {
    Idle,
    Queued,
    Running,
}

struct Session {
    queue: VecDeque<Incoming>,
    state: State,
}

struct Inner {
    sessions: HashMap<SessionKey, Session>,
    ready: VecDeque<SessionKey>,
    running: usize,
    closed: bool,
}

pub(crate) struct Executor {
    inner: Mutex<Inner>,
    cv: Condvar,
}

/// 队列概况（/api/status 与 agents 命令）。
pub(crate) struct Snapshot {
    /// 就绪待取的会话数
    pub ready: usize,
    /// 待处理消息总数（不含正在处理的）
    pub pending: usize,
    /// 有积压或正在处理的会话数
    pub sessions: usize,
}

/// 进程级单例：main 创建后登记，status/agents 读概况。
static GLOBAL: OnceLock<Arc<Executor>> = OnceLock::new();

pub(crate) fn install(ex: Arc<Executor>) {
    let _ = GLOBAL.set(ex);
}

/// 概况的 JSON 形态；未登记（测试、启动早期）给全零。
pub(crate) fn snapshot_json() -> serde_json::Value {
    let s = GLOBAL.get().map(|e| e.snapshot()).unwrap_or(Snapshot {
        ready: 0,
        pending: 0,
        sessions: 0,
    });
    json!({ "ready": s.ready, "pending": s.pending, "sessions": s.sessions })
}

pub(crate) fn session_key(msg: &Incoming) -> SessionKey {
    (
        msg.platform.clone(),
        msg.chat.message_type().to_string(),
        msg.chat.id().to_string(),
    )
}

impl Executor {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                sessions: HashMap::new(),
                ready: VecDeque::new(),
                running: 0,
                closed: false,
            }),
            cv: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 入队；该会话已满或执行器已关闭时把消息返还（Err）。
    /// Err 带整条消息是有意的：调用方要用它做丢弃日志与私聊降级回复，
    /// 且只在冷路径（队列满）才会构造，不值得为 clippy 装箱。
    #[allow(clippy::result_large_err)]
    pub(crate) fn submit(&self, msg: Incoming) -> Result<(), Incoming> {
        let mut g = self.lock();
        if g.closed {
            return Err(msg);
        }
        let key = session_key(&msg);
        if g.sessions
            .get(&key)
            .is_some_and(|s| s.queue.len() >= SESSION_QUEUE_CAP)
        {
            return Err(msg);
        }
        let inner = &mut *g;
        let s = inner
            .sessions
            .entry(key.clone())
            .or_insert_with(|| Session {
                queue: VecDeque::new(),
                state: State::Idle,
            });
        s.queue.push_back(msg);
        if s.state == State::Idle {
            s.state = State::Queued;
            inner.ready.push_back(key);
            self.cv.notify_one();
        }
        Ok(())
    }

    /// worker 阻塞取下一条：弹出的会话置 Running，done() 前它的后续消息不会被取出。
    /// 关闭且无在途、无就绪时返回 None（worker 退出）。
    pub(crate) fn next(&self) -> Option<(SessionKey, Incoming)> {
        let mut g = self.lock();
        loop {
            if let Some(key) = g.ready.pop_front() {
                let inner = &mut *g;
                let popped = inner.sessions.get_mut(&key).and_then(|s| {
                    let msg = s.queue.pop_front()?;
                    s.state = State::Running;
                    Some(msg)
                });
                match popped {
                    Some(msg) => {
                        inner.running += 1;
                        return Some((key, msg));
                    }
                    // 就绪却空队列：不该发生，清掉会话壳继续
                    None => {
                        inner.sessions.remove(&key);
                    }
                }
                continue;
            }
            if g.closed && g.running == 0 {
                return None;
            }
            g = self.cv.wait(g).unwrap_or_else(|p| p.into_inner());
        }
    }

    /// 处理完一条：该会话还有待处理 → 回就绪队列；否则移除。
    pub(crate) fn done(&self, key: &SessionKey) {
        let mut g = self.lock();
        g.running = g.running.saturating_sub(1);
        let inner = &mut *g;
        let requeue = inner.sessions.get_mut(key).map(|s| {
            if s.queue.is_empty() {
                false
            } else {
                s.state = State::Queued;
                true
            }
        });
        match requeue {
            Some(true) => inner.ready.push_back(key.clone()),
            Some(false) => {
                inner.sessions.remove(key);
            }
            None => {}
        }
        self.cv.notify_all();
    }

    /// 停收新消息；已入队的照常处理完。
    pub(crate) fn close(&self) {
        self.lock().closed = true;
        self.cv.notify_all();
    }

    /// 等全部在途与待处理清空；超时返回 false（有会话卡死时不永远等）。
    pub(crate) fn wait_drained(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut g = self.lock();
        loop {
            if g.running == 0 && g.sessions.values().all(|s| s.queue.is_empty()) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            g = self
                .cv
                .wait_timeout(g, deadline - now)
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        let g = self.lock();
        Snapshot {
            ready: g.ready.len(),
            pending: g.sessions.values().map(|s| s.queue.len()).sum(),
            sessions: g
                .sessions
                .values()
                .filter(|s| !s.queue.is_empty() || s.state == State::Running)
                .count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Chat;

    fn msg(platform: &str, chat: Chat, text: &str) -> Incoming {
        Incoming {
            platform: platform.to_string(),
            chat,
            sender_id: "1".to_string(),
            self_id: "100".to_string(),
            message_id: String::new(),
            mentions_bot: false,
            segments: Vec::new(),
            text: text.to_string(),
            images: Vec::new(),
        }
    }

    fn private(id: &str, text: &str) -> Incoming {
        msg("p", Chat::Private { id: id.to_string() }, text)
    }

    #[test]
    fn same_session_is_strictly_serial() {
        let ex = Executor::new();
        ex.submit(private("A", "a1")).unwrap();
        ex.submit(private("A", "a2")).unwrap();
        ex.submit(private("B", "b1")).unwrap();
        let (ka, m) = ex.next().unwrap();
        assert_eq!(m.text, "a1");
        // A 还在 Running：下一条必须是 B，不能是 a2
        let (kb, m) = ex.next().unwrap();
        assert_eq!(m.text, "b1");
        ex.done(&ka);
        let (_, m) = ex.next().unwrap();
        assert_eq!(m.text, "a2", "done 之后 A 才回就绪队列");
        ex.done(&kb);
    }

    #[test]
    fn sessions_interleave_fairly() {
        let ex = Executor::new();
        for t in ["a1", "a2", "a3"] {
            ex.submit(private("A", t)).unwrap();
        }
        for t in ["b1", "b2"] {
            ex.submit(private("B", t)).unwrap();
        }
        let mut order = Vec::new();
        for _ in 0..5 {
            let (k, m) = ex.next().unwrap();
            order.push(m.text.clone());
            ex.done(&k);
        }
        assert_eq!(
            order,
            ["a1", "b1", "a2", "b2", "a3"],
            "按会话轮转，不被 A 的连发独占"
        );
    }

    #[test]
    fn session_cap_returns_message_to_caller() {
        let ex = Executor::new();
        for i in 0..SESSION_QUEUE_CAP {
            ex.submit(private("A", &i.to_string())).unwrap();
        }
        let dropped = ex.submit(private("A", "overflow")).unwrap_err();
        assert_eq!(dropped.text, "overflow");
        // 其他会话不受 A 爆满影响
        ex.submit(private("B", "fine")).unwrap();
        assert_eq!(ex.snapshot().pending, SESSION_QUEUE_CAP + 1);
    }

    #[test]
    fn close_drains_then_returns_none() {
        let ex = Executor::new();
        ex.submit(private("A", "a1")).unwrap();
        ex.submit(private("B", "b1")).unwrap();
        ex.close();
        assert!(ex.submit(private("C", "late")).is_err(), "关闭后拒收");
        let (k, _) = ex.next().unwrap();
        ex.done(&k);
        let (k, _) = ex.next().unwrap();
        ex.done(&k);
        assert!(ex.next().is_none(), "排空后 worker 退出");
        assert!(ex.wait_drained(Duration::from_millis(10)));
    }

    #[test]
    fn wait_drained_times_out_while_a_session_is_stuck() {
        let ex = Executor::new();
        ex.submit(private("A", "a1")).unwrap();
        let (k, _) = ex.next().unwrap(); // 永不 done = 卡死
        assert!(!ex.wait_drained(Duration::from_millis(100)));
        ex.done(&k);
        assert!(ex.wait_drained(Duration::from_millis(100)));
    }

    #[test]
    fn private_and_group_with_same_id_are_different_sessions() {
        let ex = Executor::new();
        ex.submit(private("42", "p")).unwrap();
        ex.submit(msg("p", Chat::Group { id: "42".into() }, "g"))
            .unwrap();
        // 两者都能在对方未 done 时被取出 = 互不串行
        let (k1, m1) = ex.next().unwrap();
        let (k2, m2) = ex.next().unwrap();
        assert_ne!(k1, k2);
        assert_eq!([m1.text.as_str(), m2.text.as_str()], ["p", "g"]);
    }
}
