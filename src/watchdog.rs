//! 插件执行看门狗（评审建议 4）：进程内超时注入 + 卡死退避。
//!
//! Rust 杀不掉线程，GIL 下单插件死循环会冻住全部 worker。这里不迁子进程，分两层：
//!   1. 到期注入：PyThreadState_SetAsyncExc 往插件所在线程塞 TimeoutError，下一个
//!      字节码边界即抛——纯 Python 死循环、sleep / 网络调用返回后都能中断。
//!      每秒重注入：插件 `except Exception` 吞掉时再给机会。
//!   2. 卡死退避：注入后超过宽限仍未返回（卡在永不返回的 C 调用里），标记 stuck，
//!      责任链跳过该插件，重载成功时清除。该 worker 线程随之泄漏（1/N），
//!      其他插件与其他 worker 照常——这是不迁子进程的代价，明确接受。
//!
//! 看门狗线程注入时**要 attach GIL**：CPython 3.12+ 的 SetAsyncExc 开头就解引用
//! 当前线程 tstate，裸线程调用直接段错误。attach 不会比现状更糟——唯一让它死等
//! 的情形是某个 C 调用永久持有 GIL，那时整个解释器已经冻住。
//! 锁序：worker 是 GIL → 登记表锁，看门狗绝不能持着表锁等 GIL——锁内只收集，
//! 放锁后再 attach 注入。

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use pyo3::exceptions::PyTimeoutError;
use pyo3::prelude::*;

use crate::{log_error, log_warn, stats};

/// 注入后仍未返回的宽限；超过即标 stuck（可被测试调小）。
pub(crate) const STUCK_GRACE_MS: u64 = 10_000;
const SCAN_INTERVAL: Duration = Duration::from_millis(100);
const REINJECT_INTERVAL: Duration = Duration::from_secs(1);

static GRACE_MS: AtomicU64 = AtomicU64::new(STUCK_GRACE_MS);

struct Entry {
    plugin: String,
    deadline: Instant,
    /// 下次允许注入的时刻（首次 = deadline，之后每 REINJECT_INTERVAL）
    next_inject: Instant,
}

static CALLS: OnceLock<Mutex<HashMap<u64, Entry>>> = OnceLock::new();
static STUCK: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static THREAD: OnceLock<()> = OnceLock::new();

fn calls() -> &'static Mutex<HashMap<u64, Entry>> {
    CALLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn stuck() -> &'static Mutex<HashSet<String>> {
    STUCK.get_or_init(|| Mutex::new(HashSet::new()))
}

thread_local! {
    /// 本 OS 线程对应的 Python 线程 ident（threading.get_ident），worker 线程固定，取一次缓存
    static PY_TID: Cell<u64> = const { Cell::new(0) };
}

/// 当前线程的 Python ident：SetAsyncExc 按它定位 tstate。
pub(crate) fn current_py_thread(py: Python<'_>) -> PyResult<u64> {
    let cached = PY_TID.with(Cell::get);
    if cached != 0 {
        return Ok(cached);
    }
    let tid: u64 = py
        .import("threading")?
        .call_method0("get_ident")?
        .extract()?;
    PY_TID.with(|c| c.set(tid));
    Ok(tid)
}

/// 一次插件调用的登记凭证：Drop 即注销（Ok / Err / 异常路径都走到）。
pub(crate) struct Guard {
    tid: u64,
}

impl Drop for Guard {
    fn drop(&mut self) {
        calls()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&self.tid);
    }
}

/// 登记一次插件调用：budget 后注入 TimeoutError。调用方持 Guard 直到调用返回。
pub(crate) fn guard(tid: u64, plugin: &str, budget: Duration) -> Guard {
    THREAD.get_or_init(|| {
        std::thread::Builder::new()
            .name("plugin-watchdog".into())
            .spawn(run)
            .expect("看门狗线程启动失败");
    });
    let deadline = Instant::now() + budget;
    calls().lock().unwrap_or_else(|p| p.into_inner()).insert(
        tid,
        Entry {
            plugin: plugin.to_string(),
            deadline,
            next_inject: deadline,
        },
    );
    Guard { tid }
}

pub(crate) fn is_stuck(plugin: &str) -> bool {
    stuck()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains(plugin)
}

/// 重载成功时调用：新模块重新获得被调用的机会。
pub(crate) fn clear_stuck(plugin: &str) {
    stuck()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(plugin);
}

fn registered(tid: u64) -> bool {
    calls()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains_key(&tid)
}

/// 调用返回后兜底：清掉本线程可能残留的 pending 异常，绝不带进下一次调用。
/// 须持 GIL 调用（与注入互斥）。
pub(crate) fn clear_pending(_py: Python<'_>, tid: u64) {
    // SAFETY: exc 传 NULL 即清除，CPython 文档明确的用法
    unsafe {
        pyo3::ffi::PyThreadState_SetAsyncExc(tid as std::os::raw::c_long, std::ptr::null_mut());
    }
}

fn run() {
    loop {
        std::thread::sleep(SCAN_INTERVAL);
        let now = Instant::now();
        let grace = Duration::from_millis(GRACE_MS.load(Ordering::Relaxed));
        // 锁内只收集，不碰 GIL（锁序见模块注释）
        let (due, stuck_now) = {
            let mut m = calls().lock().unwrap_or_else(|p| p.into_inner());
            let mut due: Vec<(u64, String)> = Vec::new();
            let mut stuck_now: Vec<(u64, String)> = Vec::new();
            for (&tid, e) in m.iter_mut() {
                if now < e.next_inject {
                    continue;
                }
                if now >= e.deadline + grace {
                    stuck_now.push((tid, e.plugin.clone()));
                    continue;
                }
                e.next_inject = now + REINJECT_INTERVAL;
                due.push((tid, e.plugin.clone()));
            }
            // 标 stuck 的从登记表摘掉：不再重复注入与刷日志
            for (tid, _) in &stuck_now {
                m.remove(tid);
            }
            (due, stuck_now)
        };
        if !due.is_empty() {
            Python::attach(|py| {
                let exc = py.get_type::<PyTimeoutError>();
                for (tid, plugin) in &due {
                    // 持 GIL 再确认仍在登记：worker 的注销也在 GIL 下，二者互斥——
                    // 否则会往已返回的线程塞异常，误伤它的下一次调用
                    if !registered(*tid) {
                        continue;
                    }
                    // SAFETY: 只改目标 tstate 的 pending 异常，CPython 内部持 head 锁；
                    // exc 是解释器常驻的类型对象，SetAsyncExc 自行 incref。
                    let hit = unsafe {
                        pyo3::ffi::PyThreadState_SetAsyncExc(
                            *tid as std::os::raw::c_long,
                            exc.as_ptr(),
                        )
                    };
                    if hit == 1 {
                        log_warn!("插件 {plugin} 超时，已注入 TimeoutError");
                    }
                }
            });
        }
        for (_, plugin) in stuck_now {
            let fresh = stuck()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .insert(plugin.clone());
            if fresh {
                stats::bump_plugin_stuck();
                log_error!(
                    "插件 {plugin} 超时注入无效（疑似卡在不返回的 C 调用），已标记 stuck：\
                     责任链跳过，热重载后恢复"
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn set_stuck_grace_for_test(ms: u64) {
    GRACE_MS.store(ms, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn is_registered(tid: u64) -> bool {
    registered(tid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_registers_and_drop_unregisters() {
        // 不碰 Python：登记表语义独立可测。tid 取个不可能撞车的值
        let tid = 0xFFFF_FFF0 + std::process::id() as u64;
        {
            let _g = guard(tid, "unit", Duration::from_secs(60));
            assert!(is_registered(tid));
        }
        assert!(!is_registered(tid), "Drop 必注销");
    }

    #[test]
    fn stuck_set_is_explicit() {
        assert!(!is_stuck("ghost"));
        stuck().lock().unwrap().insert("ghost".into());
        assert!(is_stuck("ghost"));
        clear_stuck("ghost");
        assert!(!is_stuck("ghost"));
    }
}
