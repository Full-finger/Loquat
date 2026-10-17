//! Python 注入层：能力桥、插件环境挂载与解释器信号屏蔽。
//!
//! 职责：把 Go 核心能力（FFI）包装成 Python 可调用对象、把插件配置
//! （YAML 文本）注入为模块全局、屏蔽 CPython 对 Ctrl+C 的反应
//! （SIGINT 由 Rust 侧 ctrlc 处理器独占，见 src/main.rs）。

use std::error::Error;
use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use yaml_rust2::Yaml;

use crate::ffi;

static MODULE_SEQ: AtomicU64 = AtomicU64::new(0);

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

/// 把插件环境挂到模块上：config（来自插件目录的 config.yaml 文本）
/// 与 core（Go 核心能力桥）。热重载后重新挂载。
pub fn attach_plugin_env(module: &Bound<'_, PyModule>, config_yaml: &str) -> PyResult<()> {
    let config = yaml_text_to_py_dict(module.py(), config_yaml)?;
    module.setattr("config", config)?;

    let bridge = PyModule::new(module.py(), "loquat_core_bridge")?;
    bridge.add_function(wrap_pyfunction!(system_status, &bridge)?)?;
    module.setattr("core", bridge)
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
