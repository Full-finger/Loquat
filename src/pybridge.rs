//! Python 注入层：能力桥、插件环境挂载与解释器信号屏蔽。
//!
//! 职责：把 Go 核心能力（FFI）包装成 Python 可调用对象、把插件配置
//! （YAML 文本）注入为模块全局、屏蔽 CPython 对 Ctrl+C 的反应
//! （SIGINT 由 Rust 侧 ctrlc 处理器独占，见 src/main.rs）。

use crate::log_error;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::CString;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::Bound;
use yaml_rust2::Yaml;

use crate::ffi;

static MODULE_SEQ: AtomicU64 = AtomicU64::new(0);
static STATE_INIT: OnceLock<()> = OnceLock::new();

// 能力桥：把 Go 核心的能力包成 Python 可调用对象，注入扩展模块。
#[pyfunction]
fn system_status() -> PyResult<String> {
    let dll = ffi::core_dll().ok_or_else(|| PyRuntimeError::new_err("核心未就绪"))?;
    match dll.system_status() {
        Ok(s) => Ok(s),
        Err(e) => {
            let err: PyErr = PyRuntimeError::new_err(e.to_string());
            Err(err)
        }
    }
}

/// 下发动作（撤回/戳一戳等平台长尾能力），返回平台应答 JSON；
/// 失败抛异常由插件决定降级。
/// platform 缺省“哪里来哪里去”：读当前 worker 线程的消息来源平台
/// （business 调 handle 前登记），非消息上下文缺省 napcat；
/// 插件也可显式传 platform= 覆盖。
// Option 尾参 pyo3 自动可选：core.send_action(a, p, ms) 或 (a, p, ms, platform)
#[pyfunction]
fn send_action(
    py: Python,
    action: String,
    params: String,
    timeout_ms: i64,
    platform: Option<String>,
) -> PyResult<String> {
    let dll = ffi::core_dll().ok_or_else(|| PyRuntimeError::new_err("核心未就绪"))?;
    let resolved = match platform {
        Some(p) => p,
        None => current_platform().unwrap_or_else(|| "napcat".to_string()),
    };
    // FFI 等平台应答最长 timeout_ms，期间放掉 GIL，其他 worker 的 Python 不停摆
    py.detach(|| {
        dll.send_action(&resolved, &action, &params, timeout_ms as i32)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    })
}

// 每线程当前消息来源平台：worker 与会话一一绑定（chat 哈希分桶），
// 同一线程内 handle 的调用栈都属同一条消息，线程 ID 做 key 无歧义。
// 只有 business::call 会写，容量恒等于 worker 数，不设上限。
static CURRENT_PLATFORM: OnceLock<Mutex<HashMap<ThreadId, String>>> = OnceLock::new();

/// business 调插件 handle 前登记来源平台，调用后清除。
pub fn set_current_platform(platform: &str) {
    let map = CURRENT_PLATFORM.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut m) = map.lock() {
        m.insert(std::thread::current().id(), platform.to_string());
    }
}

pub fn clear_current_platform() {
    if let Some(map) = CURRENT_PLATFORM.get() {
        if let Ok(mut m) = map.lock() {
            m.remove(&std::thread::current().id());
        }
    }
}

fn current_platform() -> Option<String> {
    let map = CURRENT_PLATFORM.get()?;
    map.lock().ok()?.get(&std::thread::current().id()).cloned()
}

/// 把插件环境挂到模块上：config（来自插件目录的 config.yaml 文本）、
/// providers（宿主解析后的服务配置快照）与 core（Go 核心能力桥）。
/// 热重载后重新挂载。
pub fn attach_plugin_env(module: &Bound<'_, PyModule>, config_yaml: &str) -> PyResult<()> {
    let config = yaml_text_to_py_dict(module.py(), config_yaml)?;
    module.setattr("config", config)?;

    // provider 层（config/providers.yaml 的激活项快照）：声明了即可，
    // 未声明（旧插件/测试插件） globals().get("providers") 拿到 None
    let providers_json = crate::providers::resolved_json();
    let providers = module.py().import("json")?.call_method1("loads", (providers_json,))?;
    module.setattr("providers", providers)?;

    let bridge = PyModule::new(module.py(), "loquat_core_bridge")?;
    bridge.add_function(wrap_pyfunction!(system_status, &bridge)?)?;
    bridge.add_function(wrap_pyfunction!(send_action, &bridge)?)?;
    module.setattr("core", bridge)
}

/// 会话状态锚点：向 sys.modules 注册 loquat_state 模块（含 sessions dict），
/// 进程生命周期内永不重载。_lib 库热重载是整模块替换，模块属性存状态会丢，
/// 会话状态锚在这里，库重载后从 sys.modules 取回（docs/agent.md §状态锚点）。
/// 必须在首次加载 _lib 库之前调用（main 里 Business::load 之前）。
pub fn ensure_state_module() {
    STATE_INIT.get_or_init(|| {
        Python::attach(|py| {
            let result = (|| -> PyResult<()> {
                let module = PyModule::new(py, "loquat_state")?;
                module.setattr("sessions", PyDict::new(py))?;
                // agent 引擎计数（responds/turns）：统计口径，GIL 下并发 += 允许小误差
                module.setattr("stats", PyDict::new(py))?;
                // 会话历史落盘目录（相对 cwd，与 config/bin 同约定）；
                // 可改指临时目录，测试用
                module.setattr("persist_dir", "data/sessions")?;
                let modules = py.import("sys")?.getattr("modules")?;
                modules.set_item("loquat_state", &module)?;
                Ok(())
            })();
            if let Err(e) = result {
                log_error!("loquat_state 锚点注册失败（历史将随热重载丢失）: {e}");
            }
        });
    });
}

// 会话历史回填：遍历 persist_dir 下的 *.json 填充锚点 sessions。
// 文件格式 {"version": 1, "messages": [...]}，文件名 <message_type>_<chat_id>.json。
// 损坏/版本不符的文件跳过计数，不拖垮启动（清单 26102201 #2）。
// 标准库实现，此阶段 loquat_log 尚未注册，单个文件的错误原因走 stderr。
const LOAD_SESSIONS_SRC: &str = r#"
import json, os, sys
result = {"loaded": 0, "skipped": 0}
loquat_state = sys.modules.get("loquat_state")
if loquat_state is not None:
    sess_dir = getattr(loquat_state, "persist_dir", "data/sessions")
    if os.path.isdir(sess_dir):
        for name in sorted(os.listdir(sess_dir)):
            if not name.endswith(".json"):
                continue
            try:
                with open(os.path.join(sess_dir, name), encoding="utf-8") as f:
                    data = json.load(f)
                if data.get("version") != 1:
                    raise ValueError(f"未知版本 {data.get('version')!r}")
                stem = name[:-5]
                message_type, sep, chat_id = stem.rpartition("_")
                if not sep or message_type not in ("private", "group"):
                    raise ValueError(f"文件名不是 <message_type>_<chat_id> 形式: {stem!r}")
                messages = data.get("messages")
                if not isinstance(messages, list):
                    raise ValueError("messages 不是列表")
                loquat_state.sessions[(message_type, chat_id)] = messages
                result["loaded"] += 1
            except Exception as e:
                print(f"[loquat] 会话文件 {name} 跳过: {e}", file=sys.stderr)
                result["skipped"] += 1
"#;

/// 启动时回填会话历史，返回 (恢复数, 跳过数)。必须在 ensure_state_module 之后。
pub fn load_sessions() -> (usize, usize) {
    Python::attach(|py| {
        let run = || -> PyResult<(usize, usize)> {
            let module = PyModule::from_code(
                py,
                &CString::new(LOAD_SESSIONS_SRC)?,
                &CString::new("loquat_load_sessions.py")?,
                &CString::new("loquat_load_sessions")?,
            )?;
            let result = module.getattr("result")?;
            let loaded = result.get_item("loaded")?.extract::<usize>()?;
            let skipped = result.get_item("skipped")?.extract::<usize>()?;
            Ok((loaded, skipped))
        };
        match run() {
            Ok(n) => n,
            Err(e) => {
                log_error!("会话历史回填失败（从空历史继续）: {e}");
                (0, 0)
            }
        }
    })
}

/// 改锚点的 persist_dir（落盘/回填共用）。测试用：指到临时目录，
/// 避免测试落盘污染真实 data/。
#[cfg_attr(not(test), allow(dead_code))]
pub fn set_persist_dir(path: &str) {
    Python::attach(|py| {
        let set = || -> PyResult<()> {
            use pyo3::types::PyDict;
            let modules = py.import("sys")?.getattr("modules")?;
            let modules: &Bound<PyDict> = modules.downcast()?;
            if let Some(state) = modules.get_item("loquat_state")? {
                state.setattr("persist_dir", path)?;
            }
            Ok(())
        };
        if let Err(e) = set() {
            log_error!("persist_dir 设置失败: {e}");
        }
    });
}

/// 清空锚点 sessions（模拟重启后空内存）。仅测试用。
#[cfg_attr(not(test), allow(dead_code))]
pub fn for_test_clear_sessions() {
    Python::attach(|py| {
        let clear = || -> PyResult<()> {
            use pyo3::types::PyDict;
            let modules = py.import("sys")?.getattr("modules")?;
            let modules: &Bound<PyDict> = modules.downcast()?;
            if let Some(state) = modules.get_item("loquat_state")? {
                state.getattr("sessions")?.call_method0("clear")?;
            }
            Ok(())
        };
        if let Err(e) = clear() {
            log_error!("sessions 清空失败: {e}");
        }
    });
}

/// agent 引擎侧统计（loquat_state 锚点）：有历史会话数、respond 次数、LLM 轮次。
/// agents 控制台命令读。引擎未初始化（锚点注册失败）返回 None。
/// 主循环调：GIL 可能被 worker 短暂持有（LLM HTTP 等待已放），最坏等一小段。
pub struct AgentEngineStats {
    pub sessions: usize,
    pub responds: u64,
    pub turns: u64,
}

pub fn agent_engine_stats() -> Option<AgentEngineStats> {
    Python::attach(|py| {
        let read = || -> PyResult<AgentEngineStats> {
            use pyo3::types::PyDict;
            let modules = py.import("sys")?.getattr("modules")?;
            let modules: &Bound<PyDict> = modules.downcast()?;
            let state = modules
                .get_item("loquat_state")?
                .ok_or_else(|| PyRuntimeError::new_err("loquat_state 未注册"))?;
            let sessions = state.getattr("sessions")?.len()?;
            let stats = state.getattr("stats")?;
            let stats: &Bound<PyDict> = stats.downcast()?;
            let get = |key: &str| -> u64 {
                stats
                    .get_item(key)
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<u64>().ok())
                    .unwrap_or(0)
            };
            Ok(AgentEngineStats {
                sessions,
                responds: get("responds"),
                turns: get("turns"),
            })
        };
        read().ok()
    })
}

/// 生成全局唯一的 Python 模块名：from_code 按名字复用 sys.modules 里的
/// 已存模块并在其 __dict__ 里覆盖执行，同名会串台。
pub fn next_module_name(tag: &str) -> Result<CString, Box<dyn Error>> {
    Ok(CString::new(format!(
        "loquat_{}_{}",
        tag,
        MODULE_SEQ.fetch_add(1, Ordering::Relaxed)
    ))?)
}

/// 嵌入的 CPython 不允许抢 Ctrl+C：SIGINT 由 Rust 侧 ctrlc 处理器独占。
/// 否则同一个控制台事件会被 CPython 转成 KeyboardInterrupt 随机打进业务调用，
/// 退出行为也会变得不确定（时而退出、时而卡住）。
/// 显式触发一轮 Python GC（重载等批量丢弃对象后调用）。
pub fn collect_garbage() {
    Python::attach(|py| {
        if let Ok(gc) = py.import("gc") {
            let _ = gc.call_method0("collect");
        }
    });
}

pub fn suppress_sigint() {
    Python::attach(|py| {
        let _ = py.import("signal").and_then(|m| {
            let sigint = m.getattr("SIGINT")?;
            let ign = m.getattr("SIG_IGN")?;
            m.call_method1("signal", (sigint, ign))?;
            Ok(())
        });
    });
}

// YAML 文本 -> Python dict：先把 YAML 树转成 serde_json::Value，
// 再经标准库 json.loads 反序列化，避免逐类型走 IntoPyObject。
fn yaml_text_to_py_dict(py: Python<'_>, yaml_text: &str) -> PyResult<Py<PyAny>> {
    let docs = yaml_rust2::YamlLoader::load_from_str(yaml_text)
        .map_err(|e| PyRuntimeError::new_err(format!("配置 YAML 解析失败: {e}")))?;
    let root = docs.into_iter().next().unwrap_or(Yaml::BadValue);
    let json_text = serde_json::to_string(&yaml_to_json(&root))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let json_mod = py.import("json")?;
    let obj = json_mod.call_method1("loads", (json_text,))?;
    if obj.is_none() {
        // 空文档（如空配置文件）反序列化为 None，统一给空 dict
        let empty = pyo3::types::PyDict::new(py);
        return Ok(empty.into_any().unbind());
    }
    Ok(obj.into())
}

fn yaml_to_json(v: &Yaml) -> serde_json::Value {
    match v {
        Yaml::Real(f) => serde_json::json!(f.parse::<f64>().unwrap_or(0.0)),
        Yaml::Integer(i) => serde_json::json!(i),
        Yaml::String(s) => serde_json::json!(s),
        Yaml::Boolean(b) => serde_json::json!(b),
        Yaml::Array(items) => serde_json::Value::Array(items.iter().map(yaml_to_json).collect()),
        Yaml::Hash(h) => {
            let mut m = serde_json::Map::new();
            for (k, val) in h {
                let key = match k {
                    Yaml::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                m.insert(key, yaml_to_json(val));
            }
            serde_json::Value::Object(m)
        }
        _ => serde_json::Value::Null,
    }
}
