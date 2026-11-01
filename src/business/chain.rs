//! 责任链派发（Business::handle）：按优先级逐个调用插件，首个非空接管。
//! 从 mod.rs 迁出：R4 规则限 400 行。
//!
//! 执行预算（评审建议 4）：每条消息一份总预算 PLUGIN_BUDGET_MS，从进入链路起算，
//! 按插件串行消耗；ctx["deadline_ms"] 给插件的是**剩余量**（提案 §4.3），LLM 类
//! 插件据此收敛轮数、压缩 HTTP 超时。到期由 watchdog 注入 TimeoutError 中断；
//! 注入无效（卡在不返回的 C 调用）的插件被标 stuck，链路跳过直至热重载。

use std::error::Error;
use std::time::{Duration, Instant};

use pyo3::exceptions::PyTimeoutError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use super::{recover_poison, reply, Business, Message, MsgCtx};
use crate::{log_error, log_warn, pybridge, stats, watchdog};

/// 单次插件调用的失败：超时单列——计数、日志措辞都不同于普通异常。
enum CallError {
    Timeout,
    Other(Box<dyn Error>),
}

impl From<PyErr> for CallError {
    fn from(e: PyErr) -> Self {
        CallError::Other(Box::new(e))
    }
}

impl From<serde_json::Error> for CallError {
    fn from(e: serde_json::Error) -> Self {
        CallError::Other(Box::new(e))
    }
}

impl Business {
    /// 责任链路由：按优先级逐个调用，第一个返回非空的插件接管。
    /// 目录同步按 SYNC_INTERVAL_MS 节流，其余消息直接用快照。
    pub fn handle(&self, ctx: &MsgCtx) -> Result<Vec<Message>, Box<dyn Error>> {
        if self.sync_due() {
            if let Err(e) = self.sync() {
                log_error!("插件目录扫描失败（沿用已加载插件）: {e}");
            }
        }
        // 锁内只做快照，链路放锁外跑：Py<PyModule> 是强引用，即使中途热重载
        // 换掉模块，快照仍持有旧模块存活；LLM 等长等待不再堵其他线程
        let chain: Vec<(Py<PyModule>, String, u8)> = Python::attach(|py| {
            let plugins = self.plugins.read().unwrap_or_else(recover_poison);
            plugins
                .iter()
                .filter(|p| p.enabled())
                .map(|p| (p.module.clone_ref(py), p.dir_name.clone(), p.api_version))
                .collect()
        });
        for (idx, (module, name, api_version)) in chain.iter().enumerate() {
            // 卡死退避：超时注入无效的插件跳过，等热重载
            if watchdog::is_stuck(name) {
                continue;
            }
            let remaining = self.budget.saturating_sub(ctx.started.elapsed());
            if remaining.is_zero() {
                // 前面的插件（超时或慢而成功）吃光了预算：余下的一律不调，一行汇总
                log_warn!(
                    "本条消息预算（{} ms）已耗尽，跳过余下 {} 个插件",
                    self.budget.as_millis(),
                    chain.len() - idx
                );
                break;
            }
            let started = Instant::now();
            let result = Self::call(module, ctx, *api_version, name, remaining);
            stats::record_plugin(name, started.elapsed());
            match result {
                Ok(replies) if !replies.is_empty() => return Ok(replies),
                Ok(_) => continue,
                Err(CallError::Timeout) => {
                    stats::bump_plugin_timeout();
                    log_error!(
                        "插件 {name} 超时（本条消息预算 {} ms）",
                        self.budget.as_millis()
                    );
                }
                Err(CallError::Other(e)) => log_error!("插件 {name} 处理出错: {e}"),
            }
        }
        Ok(Vec::new())
    }

    fn call(
        module: &Py<PyModule>,
        ctx: &MsgCtx,
        api_version: u8,
        name: &str,
        remaining: Duration,
    ) -> Result<Vec<Message>, CallError> {
        // 登记"哪里来"：插件经能力桥发逃生舱动作时按此路由平台
        pybridge::set_current_platform(ctx.platform);
        let result = Python::attach(|py| -> Result<Vec<Message>, CallError> {
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
            let segments_json = serde_json::to_string(ctx.segments)?;
            let json_mod = py.import("json")?;
            let segments = json_mod.call_method1("loads", (segments_json,))?;
            dict.set_item("segments", segments)?;
            dict.set_item("platform", ctx.platform)?;
            dict.set_item("deadline_ms", remaining.as_millis() as u64)?;

            let tid = watchdog::current_py_thread(py)?;
            // Guard 只罩住 handle() 本身；from_python 的 json.dumps 不该被注入打断
            let ret = {
                let _guard = watchdog::guard(tid, name, remaining);
                module.bind(py).call_method1("handle", (dict,))
            };
            watchdog::clear_pending(py, tid);
            let ret = match ret {
                Ok(v) => v,
                Err(e) if e.is_instance_of::<PyTimeoutError>(py) => return Err(CallError::Timeout),
                Err(e) => return Err(e.into()),
            };
            reply::from_python(py, &ret, api_version).map_err(CallError::Other)
        });
        pybridge::clear_current_platform();
        result
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
