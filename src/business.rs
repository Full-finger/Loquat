//! Python 业务（qa.py）的加载、调用与热重载。
//! 热重载是惰性的：每次调用前比对脚本 mtime，变了才重载，
//! 重载失败沿用旧版本继续服务。对问答场景即改即生效。

use std::error::Error;
use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

use crate::ffi;

static MODULE_SEQ: AtomicU64 = AtomicU64::new(0);

struct Loaded {
    module: Py<PyModule>,
    mtime: SystemTime,
}

pub struct Business {
    path: String,
    inner: Mutex<Loaded>,
    reloads: AtomicU64,
}

impl Business {
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let loaded = Self::load_module(path)?;
        Ok(Self {
            path: path.into(),
            inner: Mutex::new(loaded),
            reloads: AtomicU64::new(0),
        })
    }

    fn load_module(path: &str) -> Result<Loaded, Box<dyn Error>> {
        let source =
            std::fs::read_to_string(path).map_err(|e| format!("读业务脚本失败 {path}: {e}"))?;
        Python::attach(|py| {
            let file = CString::new(path)?;
            let name = next_module_name("qa")?;
            let code = CString::new(source)?;
            let module = PyModule::from_code(py, &code, &file, &name)?;
            attach_core_bridge(&module)?;
            let mtime = std::fs::metadata(path)?.modified()?;
            Ok(Loaded {
                module: module.into(),
                mtime,
            })
        })
    }

    pub fn handle(&self, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut loaded = self.inner.lock().expect("业务锁中毒");
        if let Ok(meta) = std::fs::metadata(&self.path) {
            if let Ok(mtime) = meta.modified() {
                if mtime > loaded.mtime {
                    match Self::load_module(&self.path) {
                        Ok(new) => {
                            *loaded = new;
                            self.reloads.fetch_add(1, Ordering::Relaxed);
                            eprintln!(
                                "[rust] qa.py 已热重载（第 {} 次）",
                                self.reloads.load(Ordering::Relaxed)
                            );
                        }
                        Err(e) => eprintln!("[rust] qa.py 重载失败，沿用旧版本: {e}"),
                    }
                }
            }
        }
        Python::attach(|py| {
            let replies = loaded.module.bind(py).call_method1("handle", (text,))?;
            Ok(replies.extract()?)
        })
    }

    pub fn info(&self) -> String {
        format!(
            "{}（重载 {} 次）",
            self.path,
            self.reloads.load(Ordering::Relaxed)
        )
    }
}

// 每次加载用唯一模块名：pyo3 的 from_code 按名字复用 sys.modules 里的
// 已存模块并在其 __dict__ 里覆盖执行，同名会串台。
fn next_module_name(tag: &str) -> Result<CString, Box<dyn Error>> {
    Ok(CString::new(format!(
        "loquat_{}_{}",
        tag,
        MODULE_SEQ.fetch_add(1, Ordering::Relaxed)
    ))?)
}

// 能力桥：把 Go 核心的能力包成 Python 可调用对象，注入扩展模块。
// 业务（qa.py）由此使用核心能力，自身保持纯 Python。
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

// 把能力桥以 core 名字挂到扩展模块上；热重载后重新挂载。
fn attach_core_bridge(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let bridge = PyModule::new(module.py(), "loquat_core_bridge")?;
    bridge.add_function(wrap_pyfunction!(system_status, &bridge)?)?;
    module.setattr("core", bridge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_hot_reload() {
        let path = std::env::temp_dir().join("loquat_qa_reload_test.py");
        std::fs::write(&path, "def handle(text):\n    return ['v1']\n").unwrap();
        let business = Business::load(path.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("k").unwrap(), vec!["v1".to_string()]);

        // mtime 有粒度，等一会让改动可观测
        std::thread::sleep(std::time::Duration::from_millis(1500));
        std::fs::write(&path, "def handle(text):\n    return ['v2']\n").unwrap();
        assert_eq!(business.handle("k").unwrap(), vec!["v2".to_string()]);
        assert_eq!(business.reloads.load(Ordering::Relaxed), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn qa_rules() {
        let business = Business::load("extensions/qa.py").unwrap();
        assert_eq!(business.handle("ping").unwrap(), vec!["pong".to_string()]);
        assert_eq!(business.handle("/ping").unwrap(), vec!["pong".to_string()]);
        let r = business.handle("没有这个关键词").unwrap();
        assert!(r.is_empty(), "应无回复，实际: {r:?}");
    }
}
