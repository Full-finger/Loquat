//! Python 业务插件加载器：扫描插件目录，责任链路由，逐文件惰性热重载。
//!
//! 插件约定（extensions/ 下每个 .py）：
//!   handle(text) -> list[str]   必须实现；返回空列表 = 不归我管，路由链继续
//!   PRIORITY = <int>            可选；越小越先被路由（缺省 100，同优先级按文件名）
//!   全局 core                    宿主注入的能力桥（Go 核心能力），热重载后自动重挂
//!
//! 路由语义：按 (PRIORITY, 文件名) 排序逐个调用，第一个返回非空的插件接管。
//! 兜底插件想沉底：PRIORITY 设大即可。

use std::error::Error;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

use crate::ffi;

static MODULE_SEQ: AtomicU64 = AtomicU64::new(0);

const DEFAULT_PRIORITY: i64 = 100;

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

// 把能力桥以 core 名字挂到扩展模块上；热重载后重新挂载。
fn attach_core_bridge(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let bridge = PyModule::new(module.py(), "loquat_core_bridge")?;
    bridge.add_function(wrap_pyfunction!(system_status, &bridge)?)?;
    module.setattr("core", bridge)
}

fn next_module_name(tag: &str) -> Result<CString, Box<dyn Error>> {
    Ok(CString::new(format!(
        "loquat_{}_{}",
        tag,
        MODULE_SEQ.fetch_add(1, Ordering::Relaxed)
    ))?)
}

struct LoadedPlugin {
    path: PathBuf,
    name: String,
    priority: i64,
    module: Py<PyModule>,
    mtime: SystemTime,
}

pub struct Business {
    dir: PathBuf,
    plugins: Mutex<Vec<LoadedPlugin>>,
    reloads: AtomicU64,
}

impl Business {
    pub fn load(dir: &str) -> Result<Self, Box<dyn Error>> {
        let business = Self {
            dir: dir.into(),
            plugins: Mutex::new(Vec::new()),
            reloads: AtomicU64::new(0),
        };
        business.sync()?;
        Ok(business)
    }

    /// 重新扫描插件目录：发现新插件、重载有改动的、移除已删除的。
    /// 单个插件加载失败只跳过自身，不拖垮整条链。
    fn sync(&self) -> Result<(), Box<dyn Error>> {
        let mut plugins = self.plugins.lock().expect("业务锁中毒");

        let mut wanted: Vec<(PathBuf, String, SystemTime)> = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("py") {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let mtime = entry.metadata()?.modified()?;
            wanted.push((path, name, mtime));
        }
        wanted.sort_by(|a, b| a.1.cmp(&b.1));

        // 磁盘上已删除的插件，同步移除
        plugins.retain(|p| wanted.iter().any(|(path, _, _)| path == &p.path));

        for (path, name, mtime) in wanted {
            if let Some(existing) = plugins.iter_mut().find(|p| p.path == path) {
                // 已加载：mtime 变了才重载，重载失败沿用旧版本
                if mtime > existing.mtime {
                    match Self::load_module(&path) {
                        Ok((module, priority)) => {
                            existing.module = module;
                            existing.priority = priority;
                            existing.mtime = mtime;
                            self.reloads.fetch_add(1, Ordering::Relaxed);
                            eprintln!(
                                "[rust] 插件已热重载: {name}（第 {} 次）",
                                self.reloads.load(Ordering::Relaxed)
                            );
                        }
                        Err(e) => eprintln!("[rust] 插件 {name} 重载失败，沿用旧版本: {e}"),
                    }
                }
                continue;
            }
            match Self::load_module(&path) {
                Ok((module, priority)) => {
                    eprintln!("[rust] 插件已加载: {name}（优先级 {priority}）");
                    plugins.push(LoadedPlugin {
                        path: path.clone(),
                        name,
                        priority,
                        module,
                        mtime,
                    });
                }
                Err(e) => eprintln!("[rust] 插件 {name} 加载失败，已跳过: {e}"),
            }
        }

        plugins.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.name.cmp(&b.name)));
        Ok(())
    }

    fn load_module(path: &Path) -> Result<(Py<PyModule>, i64), Box<dyn Error>> {
        let source =
            std::fs::read_to_string(path).map_err(|e| format!("读插件失败 {path:?}: {e}"))?;
        Python::attach(|py| {
            let file = CString::new(path.to_string_lossy().as_bytes())?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("plugin");
            let name = next_module_name(&format!("ext_{stem}"))?;
            let code = CString::new(source)?;
            let module = PyModule::from_code(py, &code, &file, &name)?;
            attach_core_bridge(&module)?;
            let priority = module
                .getattr("PRIORITY")
                .and_then(|v| v.extract::<i64>())
                .unwrap_or(DEFAULT_PRIORITY);
            Ok((module.into(), priority))
        })
    }

    /// 责任链路由：按优先级逐个调用，第一个返回非空的插件接管。
    /// 每条消息前惰性同步一次插件目录（扫描开销可忽略）。
    pub fn handle(&self, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        if let Err(e) = self.sync() {
            eprintln!("[rust] 插件目录扫描失败（沿用已加载插件）: {e}");
        }
        let plugins = self.plugins.lock().expect("业务锁中毒");
        for plugin in plugins.iter() {
            match Self::call(&plugin.module, text) {
                Ok(replies) if !replies.is_empty() => return Ok(replies),
                Ok(_) => continue,
                Err(e) => eprintln!("[rust] 插件 {} 处理出错: {e}", plugin.name),
            }
        }
        Ok(Vec::new())
    }

    fn call(module: &Py<PyModule>, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        Python::attach(|py| {
            let replies = module.bind(py).call_method1("handle", (text,))?;
            Ok(replies.extract()?)
        })
    }

    pub fn info(&self) -> String {
        let plugins = self.plugins.lock().expect("业务锁中毒");
        let names: Vec<String> = plugins.iter().map(|p| p.name.clone()).collect();
        format!(
            "{}（{} 个插件，重载 {} 次）：{}",
            self.dir.display(),
            plugins.len(),
            self.reloads.load(Ordering::Relaxed),
            names.join(" → ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("loquat_biz_test_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn business_hot_reload() {
        let dir = plugin_dir("hot_reload");
        let plugin = dir.join("p1.py");
        std::fs::write(&plugin, "def handle(text):\n    return ['v1']\n").unwrap();
        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("k").unwrap(), vec!["v1".to_string()]);

        // mtime 有粒度，等一会让改动可观测
        std::thread::sleep(std::time::Duration::from_millis(1500));
        std::fs::write(&plugin, "def handle(text):\n    return ['v2']\n").unwrap();
        assert_eq!(business.handle("k").unwrap(), vec!["v2".to_string()]);
        assert_eq!(business.reloads.load(Ordering::Relaxed), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chain_routes_by_priority() {
        let dir = plugin_dir("chain");
        std::fs::write(
            dir.join("zz_low.py"),
            "PRIORITY = 999\ndef handle(text):\n    return ['兜底']\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("aa_high.py"),
            "PRIORITY = 1\ndef handle(text):\n    return ['高优先级']\n",
        )
        .unwrap();
        // 空回复的插件不接管，链继续
        std::fs::write(dir.join("mm_none.py"), "def handle(text):\n    return []\n").unwrap();

        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("x").unwrap(), vec!["高优先级".to_string()]);

        // 删掉高优先级插件后，同步应落到兜底
        std::fs::remove_file(dir.join("aa_high.py")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert_eq!(business.handle("x").unwrap(), vec!["兜底".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
