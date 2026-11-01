//! 责任链派发（Business::handle）：三阶段管线（评审建议 6）。
//! 从 mod.rs 迁出：R4 规则限 400 行。
//!
//!   ① 观察  on_observe(ctx)         全部执行，返回值忽略——历史、审计等横切关注点
//!                                   不再被抢答截胡
//!   ② 路由  on_message(ctx)         按优先级首个非空接管（v1 handle 经垫片）
//!   ③ 变换  before_send(ctx, segs)  对胜者每条消息逐级 fold：None 不改 / list 替换 /
//!                                   空表丢弃 / str 视为单 text 段；出错保留原段
//!
//! 执行预算（评审建议 4）：每条消息一份总预算 PLUGIN_BUDGET_MS，从进入链路起算，
//! 三阶段所有钩子串行消耗；ctx["deadline_ms"] 给插件的是**剩余量**（提案 §4.3）。
//! 到期由 watchdog 注入 TimeoutError 中断；注入无效的插件被标 stuck，三阶段都跳过。
//! 预算在 ①② 耗尽 = 本条无回复；在 ③ 耗尽 = 胜者回复原样发出（已产出的不丢）。

use std::error::Error;
use std::time::{Duration, Instant};

use pyo3::exceptions::PyTimeoutError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};

use super::{recover_poison, reply, Business, Message, MsgCtx};
use crate::{log_error, log_warn, pybridge, stats, watchdog};

/// 快照里的一个插件：钩子是模块的函数对象，连模块一起持强引用——
/// 热重载换掉模块也不影响本次在途调用。
struct ChainNode {
    name: String,
    api_version: u8,
    entry: Option<Py<PyAny>>,
    on_observe: Option<Py<PyAny>>,
    before_send: Option<Py<PyAny>>,
    _module: Py<PyModule>,
}

/// 单次钩子调用的失败：超时单列——计数、日志措辞都不同于普通异常。
enum CallError {
    Timeout,
    Other(Box<dyn Error>),
}

impl From<PyErr> for CallError {
    fn from(e: PyErr) -> Self {
        CallError::Other(Box::new(e))
    }
}

/// 本条消息的预算：三阶段共用，耗尽只告警一次。
struct Budget {
    total: Duration,
    started: Instant,
    warned: bool,
}

impl Budget {
    /// 剩余预算；耗尽返回 None 并（首次）告警。
    fn remaining(&mut self, phase: &str, name: &str) -> Option<Duration> {
        let r = self.total.saturating_sub(self.started.elapsed());
        if r.is_zero() {
            if !self.warned {
                log_warn!(
                    "本条消息预算（{} ms）已耗尽，{phase}阶段从 {name} 起跳过",
                    self.total.as_millis()
                );
                self.warned = true;
            }
            return None;
        }
        Some(r)
    }
}

impl Business {
    /// 三阶段管线入口。目录同步按 SYNC_INTERVAL_MS 节流，其余消息直接用快照。
    pub fn handle(&self, ctx: &MsgCtx) -> Result<Vec<Message>, Box<dyn Error>> {
        if self.sync_due() {
            if let Err(e) = self.sync() {
                log_error!("插件目录扫描失败（沿用已加载插件）: {e}");
            }
        }
        // 锁内只做快照，链路放锁外跑：LLM 等长等待不堵其他线程
        let chain: Vec<ChainNode> = Python::attach(|py| {
            let plugins = self.plugins.read().unwrap_or_else(recover_poison);
            plugins
                .iter()
                .filter(|p| p.enabled())
                .map(|p| ChainNode {
                    name: p.dir_name.clone(),
                    api_version: p.api_version,
                    entry: p.hooks.entry.as_ref().map(|f| f.clone_ref(py)),
                    on_observe: p.hooks.on_observe.as_ref().map(|f| f.clone_ref(py)),
                    before_send: p.hooks.before_send.as_ref().map(|f| f.clone_ref(py)),
                    _module: p.module.clone_ref(py),
                })
                .collect()
        });
        // 登记"哪里来"：插件经能力桥发逃生舱动作时按此路由平台
        pybridge::set_current_platform(ctx.platform);
        let result = Python::attach(|py| self.run_chain(py, ctx, &chain));
        pybridge::clear_current_platform();
        result
    }

    fn run_chain(
        &self,
        py: Python<'_>,
        ctx: &MsgCtx,
        chain: &[ChainNode],
    ) -> Result<Vec<Message>, Box<dyn Error>> {
        let dict = build_ctx(py, ctx)?;
        let json_mod = py.import("json")?;
        let tid = watchdog::current_py_thread(py)?;
        let mut budget = Budget {
            total: self.budget,
            started: ctx.started,
            warned: false,
        };

        // ① 观察：全部执行，错误与超时只记日志
        for node in chain {
            let Some(hook) = &node.on_observe else {
                continue;
            };
            if watchdog::is_stuck(&node.name) {
                continue;
            }
            let Some(remaining) = budget.remaining("观察", &node.name) else {
                return Ok(Vec::new());
            };
            dict.set_item("deadline_ms", remaining.as_millis() as u64)?;
            let started = Instant::now();
            let r = invoke(py, tid, &node.name, remaining, || {
                hook.bind(py).call1((&dict,))
            });
            stats::record_plugin(&node.name, started.elapsed());
            report(&node.name, "on_observe", r.map(|_| ()));
        }

        // ② 路由：首个非空接管
        let mut winner: Option<Vec<Message>> = None;
        for node in chain {
            let Some(entry) = &node.entry else {
                continue;
            };
            if watchdog::is_stuck(&node.name) {
                continue;
            }
            let Some(remaining) = budget.remaining("路由", &node.name) else {
                return Ok(Vec::new());
            };
            dict.set_item("deadline_ms", remaining.as_millis() as u64)?;
            let started = Instant::now();
            let r = invoke(py, tid, &node.name, remaining, || {
                entry.bind(py).call1((&dict,))
            })
            .and_then(|ret| {
                reply::from_python(py, &ret, node.api_version).map_err(CallError::Other)
            });
            stats::record_plugin(&node.name, started.elapsed());
            match r {
                Ok(replies) if !replies.is_empty() => {
                    winner = Some(replies);
                    break;
                }
                Ok(_) => {}
                Err(e) => report(&node.name, "on_message", Err(e)),
            }
        }
        let Some(mut replies) = winner else {
            return Ok(Vec::new());
        };

        // ③ 变换：胜者的每条消息逐级 fold；预算耗尽则原样发出
        for node in chain {
            let Some(hook) = &node.before_send else {
                continue;
            };
            if watchdog::is_stuck(&node.name) {
                continue;
            }
            let Some(remaining) = budget.remaining("变换", &node.name) else {
                break;
            };
            dict.set_item("deadline_ms", remaining.as_millis() as u64)?;
            let started = Instant::now();
            let mut folded: Vec<Message> = Vec::with_capacity(replies.len());
            for msg in &replies {
                let segs_py = json_mod.call_method1("loads", (serde_json::to_string(msg)?,))?;
                let r = invoke(py, tid, &node.name, remaining, || {
                    hook.bind(py).call1((&dict, segs_py))
                })
                .and_then(|ret| fold_result(py, &ret));
                match r {
                    Ok(None) => folded.push(msg.clone()),
                    Ok(Some(segs)) if segs.is_empty() => {} // 空表 = 丢弃该条
                    Ok(Some(segs)) => folded.push(segs),
                    Err(e) => {
                        report(&node.name, "before_send", Err(e));
                        folded.push(msg.clone());
                    }
                }
            }
            stats::record_plugin(&node.name, started.elapsed());
            replies = folded;
        }
        Ok(replies)
    }

    /// 测试便利：回复投影为文本（v1 字符串经 text 段往返恒等）。
    #[cfg(test)]
    pub(super) fn texts(&self, ctx: &MsgCtx) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(self
            .handle(ctx)?
            .iter()
            .map(|m| crate::event::derive_text(m))
            .collect())
    }
}

/// 传给钩子的 ctx dict（deadline_ms 每次调用前刷新）。
fn build_ctx<'py>(py: Python<'py>, ctx: &MsgCtx) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("text", ctx.text)?;
    dict.set_item("message_type", ctx.chat.message_type())?;
    dict.set_item("chat_id", ctx.chat.id())?;
    dict.set_item("user_id", ctx.sender_id)?;
    dict.set_item("self_id", ctx.self_id)?;
    dict.set_item("message_id", ctx.message_id)?;
    dict.set_item("is_at_bot", ctx.mentions_bot)?;
    dict.set_item("images", ctx.images.to_vec())?;
    // serde_json::Value 无 IntoPyObject，走 json 字符串中转（同 pybridge 的 config 注入）
    let segments_json = serde_json::to_string(ctx.segments)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let segments = py.import("json")?.call_method1("loads", (segments_json,))?;
    dict.set_item("segments", segments)?;
    dict.set_item("platform", ctx.platform)?;
    Ok(dict)
}

/// 在看门狗登记下调一次钩子；返回后清掉可能残留的 pending 异常。
fn invoke<'py>(
    py: Python<'py>,
    tid: u64,
    name: &str,
    remaining: Duration,
    f: impl FnOnce() -> PyResult<Bound<'py, PyAny>>,
) -> Result<Bound<'py, PyAny>, CallError> {
    let ret = {
        let _guard = watchdog::guard(tid, name, remaining);
        f()
    };
    watchdog::clear_pending(py, tid);
    match ret {
        Ok(v) => Ok(v),
        Err(e) if e.is_instance_of::<PyTimeoutError>(py) => Err(CallError::Timeout),
        Err(e) => Err(e.into()),
    }
}

/// 钩子失败的统一记账：超时计数 + 日志措辞区分。
fn report(name: &str, hook: &str, r: Result<(), CallError>) {
    match r {
        Ok(()) => {}
        Err(CallError::Timeout) => {
            stats::bump_plugin_timeout();
            log_error!("插件 {name} {hook} 超时（预算耗尽）");
        }
        Err(CallError::Other(e)) => log_error!("插件 {name} {hook} 出错: {e}"),
    }
}

/// before_send 返回值：None → 不改；其余按 v2 归一化（str 单段 / list 段列表），
/// 归一化为空（[] / ""）= 丢弃该条消息。
fn fold_result(py: Python<'_>, ret: &Bound<'_, PyAny>) -> Result<Option<Message>, CallError> {
    if ret.is_none() {
        return Ok(None);
    }
    let mut msgs = reply::from_python(py, ret, 2).map_err(CallError::Other)?;
    Ok(Some(msgs.pop().unwrap_or_default()))
}
