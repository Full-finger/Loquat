//! 业务插件加载器：扫描 extensions/<名>/plugin.py，责任链路由，
//! 逐插件惰性热重载。Python 注入层（能力桥/配置挂载/SIGINT 屏蔽）
//! 见 src/pybridge.rs。
//!
//! 插件约定（extensions/<名>/ 目录，自包含）：
//!   plugin.py               入口：必须实现 handle(text) -> list[str]
//!                           （返回空列表 = 不归我管，路由链继续）；
//!                           可声明 PRIORITY = <int>（越小越先，缺省 100）
//!   config.yaml             可选：插件配置，注入为全局 config（dict），
//!                           其中 enabled: false 可停用插件
//!   .disabled               运行时启停标记（控制台命令写入/删除）
//!   全局 core                宿主注入的能力桥（pybridge 注入），重载后自动重挂
//!
//! 路由语义：按 (PRIORITY, 目录名) 排序逐个调用，第一个返回非空的插件接管。

use std::error::Error;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::pybridge;

const DEFAULT_PRIORITY: i64 = 100;
const ENTRY_FILE: &str = "plugin.py";
const CONFIG_FILE: &str = "config.yaml";
const DISABLE_MARKER: &str = ".disabled";

struct LoadedPlugin {
    dir_name: String,
    path: PathBuf,        // plugin.py
    priority: i64,
    config_enabled: bool, // config.yaml 声明
    marker_disabled: bool, // .disabled 标记（控制台启停）
    module: Py<PyModule>,
    mtime: SystemTime,
}

impl LoadedPlugin {
    fn enabled(&self) -> bool {
        self.config_enabled && !self.marker_disabled
    }
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

    /// 重新扫描插件目录：发现新插件、重载有改动的（plugin.py 或 config.yaml）、
    /// 移除已删除的。单个插件加载失败只跳过自身，不拖垮整条链。
    fn sync(&self) -> Result<(), Box<dyn Error>> {
        let mut plugins = self.plugins.lock().expect("业务锁中毒");

        // mtime 取 plugin.py 与 config.yaml 的较新者：任一变更都触发重载
        let mut wanted: Vec<(PathBuf, String, SystemTime)> = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let dir_path = entry?.path();
            if !dir_path.is_dir() {
                continue;
            }
            let entry_py = dir_path.join(ENTRY_FILE);
            if !entry_py.is_file() {
                continue; // 没有入口的目录不是插件
            }
            let name = dir_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let mut mtime = file_mtime(&entry_py).unwrap_or(SystemTime::UNIX_EPOCH);
            let config_path = dir_path.join(CONFIG_FILE);
            if let Ok(mt) = file_mtime(&config_path) {
                if mt > mtime {
                    mtime = mt;
                }
            }
            wanted.push((entry_py, name, mtime));
        }
        wanted.sort_by(|a, b| a.1.cmp(&b.1));

        // 磁盘上已删除的插件，同步移除
        plugins.retain(|p| wanted.iter().any(|(path, _, _)| path == &p.path));

        for (path, name, mtime) in wanted {
            let marker = self.dir.join(&name).join(DISABLE_MARKER);
            let marker_now = marker.exists();

            if let Some(existing) = plugins.iter_mut().find(|p| p.dir_name == name) {
                let config_changed = mtime > existing.mtime;
                let toggled = marker_now != existing.marker_disabled;
                if !config_changed && !toggled {
                    continue;
                }
                if config_changed {
                    match Self::load_module(&path) {
                        Ok((module, priority, config_enabled)) => {
                            existing.module = module;
                            existing.priority = priority;
                            existing.config_enabled = config_enabled;
                            existing.mtime = mtime;
                            self.reloads.fetch_add(1, Ordering::Relaxed);
                            eprintln!(
                                "[rust] 插件已热重载: {name}（第 {} 次）",
                                self.reloads.load(Ordering::Relaxed)
                            );
                        }
                        Err(e) => {
                            eprintln!("[rust] 插件 {name} 重载失败，沿用旧版本: {e}");
                            continue;
                        }
                    }
                }
                if toggled {
                    existing.marker_disabled = marker_now;
                    eprintln!(
                        "[rust] 插件 {name} 已{}",
                        if marker_now { "禁用" } else { "启用" }
                    );
                }
                continue;
            }

            // 新插件
            let marker_disabled = marker.exists();
            match Self::load_module(&path) {
                Ok((module, priority, config_enabled)) => {
                    let enabled = config_enabled && !marker_disabled;
                    eprintln!(
                        "[rust] 插件已加载: {name}（优先级 {priority}，{}）",
                        if enabled { "启用" } else { "禁用" }
                    );
                    plugins.push(LoadedPlugin {
                        dir_name: name,
                        path,
                        priority,
                        config_enabled,
                        marker_disabled,
                        module,
                        mtime,
                    });
                }
                Err(e) => eprintln!("[rust] 插件 {name} 加载失败，已跳过: {e}"),
            }
        }

        plugins.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.dir_name.cmp(&b.dir_name))
        });
        Ok(())
    }

    /// 加载插件入口，注入 config / core 环境。返回模块、优先级与配置内 enabled 声明。
    fn load_module(
        path: &Path,
    ) -> Result<(Py<PyModule>, i64, bool), Box<dyn Error>> {
        let source =
            std::fs::read_to_string(path).map_err(|e| format!("读插件失败 {path:?}: {e}"))?;
        let config_path = path.parent().unwrap_or(Path::new(".")).join(CONFIG_FILE);
        // 没有配置文件 = 空配置，不算错误
        let config_yaml = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
            Err(e) => return Err(format!("读配置失败 {config_path:?}: {e}").into()),
        };

        Python::attach(|py| {
            let file = CString::new(path.to_string_lossy().as_bytes())?;
            let dir_name = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("plugin");
            let name = pybridge::next_module_name(&format!("ext_{dir_name}"))?;
            let code = CString::new(source)?;
            let module = PyModule::from_code(py, &code, &file, &name)?;
            pybridge::attach_plugin_env(&module, &config_yaml)?;

            let priority = module
                .getattr("PRIORITY")
                .and_then(|v| v.extract::<i64>())
                .unwrap_or(DEFAULT_PRIORITY);
            let enabled = module
                .getattr("config")
                .and_then(|c| c.get_item("enabled"))
                .and_then(|v| v.extract::<bool>())
                .unwrap_or(true);
            Ok((module.into(), priority, enabled))
        })
    }

    /// 责任链路由：按优先级逐个调用，第一个返回非空的插件接管。
    /// 每条消息前惰性同步一次插件目录（扫描开销可忽略）。
    pub fn handle(&self, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        if let Err(e) = self.sync() {
            eprintln!("[rust] 插件目录扫描失败（沿用已加载插件）: {e}");
        }
        let plugins = self.plugins.lock().expect("业务锁中毒");
        for plugin in plugins.iter().filter(|p| p.enabled()) {
            match Self::call(&plugin.module, text) {
                Ok(replies) if !replies.is_empty() => return Ok(replies),
                Ok(_) => continue,
                Err(e) => eprintln!("[rust] 插件 {} 处理出错: {e}", plugin.dir_name),
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

    /// 运行时启停插件：写/删 .disabled 标记并同步内存。持久化，重启依然生效。
    /// 返回是否找到对应插件。
    pub fn set_enabled(&self, name: &str, enabled: bool) -> bool {
        let marker = self.dir.join(name).join(DISABLE_MARKER);
        let marker_result = if enabled {
            let _ = std::fs::remove_file(&marker);
            Ok(())
        } else {
            std::fs::write(&marker, "")
        };
        if let Err(e) = marker_result {
            eprintln!("[rust] 写启停标记失败: {e}");
            return false;
        }

        let mut plugins = self.plugins.lock().expect("业务锁中毒");
        match plugins.iter_mut().find(|p| p.dir_name == name) {
            Some(p) => {
                p.marker_disabled = !enabled;
                true
            }
            // 插件尚未被扫描到：标记已落盘，下次扫描自然生效
            None => false,
        }
    }

    pub fn info(&self) -> String {
        let plugins = self.plugins.lock().expect("业务锁中毒");
        let names: Vec<String> = plugins
            .iter()
            .map(|p| {
                if p.enabled() {
                    p.dir_name.clone()
                } else {
                    format!("{}[禁用]", p.dir_name)
                }
            })
            .collect();
        format!(
            "{}（{} 个插件，重载 {} 次）：{}",
            self.dir.display(),
            plugins.len(),
            self.reloads.load(Ordering::Relaxed),
            names.join(" → ")
        )
    }
}

fn file_mtime(path: &Path) -> Result<SystemTime, std::io::Error> {
    std::fs::metadata(path)?.modified()
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

    fn write_plugin(dir: &Path, folder: &str, code: &str) {
        let d = dir.join(folder);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(ENTRY_FILE), code).unwrap();
    }

    #[test]
    fn business_hot_reload() {
        let dir = plugin_dir("hot_reload");
        write_plugin(
            &dir,
            "p",
            "def handle(text):\n    return ['v1']\n",
        );
        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("k").unwrap(), vec!["v1".to_string()]);

        // mtime 有粒度，等一会让改动可观测
        std::thread::sleep(std::time::Duration::from_millis(1500));
        write_plugin(&dir, "p", "def handle(text):\n    return ['v2']\n");
        assert_eq!(business.handle("k").unwrap(), vec!["v2".to_string()]);
        assert_eq!(business.reloads.load(Ordering::Relaxed), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chain_routes_by_priority_and_survives_removal() {
        let dir = plugin_dir("chain");
        write_plugin(
            &dir,
            "zz_low",
            "PRIORITY = 999\ndef handle(text):\n    return ['兜底']\n",
        );
        write_plugin(
            &dir,
            "aa_high",
            "PRIORITY = 1\ndef handle(text):\n    return ['高优先级']\n",
        );
        // 空回复的插件不接管，链继续
        write_plugin(&dir, "mm_none", "def handle(text):\n    return []\n");

        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("x").unwrap(), vec!["高优先级".to_string()]);

        // 删掉高优先级插件后，同步应落到兜底
        std::fs::remove_dir_all(dir.join("aa_high")).unwrap();
        assert_eq!(business.handle("x").unwrap(), vec!["兜底".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plugin_can_be_disabled() {
        let dir = plugin_dir("disable");
        write_plugin(&dir, "p", "def handle(text):\n    return ['ok']\n");
        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("x").unwrap(), vec!["ok".to_string()]);

        assert!(business.set_enabled("p", false));
        assert!(business.handle("x").unwrap().is_empty());
        assert!(business.set_enabled("p", true));
        assert_eq!(business.handle("x").unwrap(), vec!["ok".to_string()]);
        assert!(!business.set_enabled("不存在", true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_yaml_is_injected_and_gates_enabled() {
        let dir = plugin_dir("config");
        write_plugin(
            &dir,
            "cfgtest",
            "def handle(text):\n    return [config.get('greeting', '缺失')]\n",
        );
        std::fs::write(
            dir.join("cfgtest").join(CONFIG_FILE),
            "enabled: true\ngreeting: 你好呀\n",
        )
        .unwrap();
        let business = Business::load(dir.to_str().unwrap()).unwrap();
        assert_eq!(business.handle("x").unwrap(), vec!["你好呀".to_string()]);

        // enabled: false —— 路由链跳过
        std::fs::write(
            dir.join("cfgtest").join(CONFIG_FILE),
            "enabled: false\ngreeting: 你好呀\n",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(business.handle("x").unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
