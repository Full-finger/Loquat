//! 业务插件加载器：扫描 extensions/<名>/plugin.py，责任链路由，
//! 逐插件惰性热重载。Python 注入层（能力桥/配置挂载/SIGINT 屏蔽）
//! 见 src/pybridge.rs。
//!
//! 插件约定（extensions/<名>/ 目录，自包含）：
//!   plugin.py               入口：首行必须是签名 `# loquat-plugin`（宿主
//!                           识别插件的依据，无签名目录忽略）；必须实现
//!                           handle(ctx) -> list[str]
//!                           （ctx: dict，含 text/message_type/chat_id/
//!                           user_id/self_id；返回空列表 = 不归我管，
//!                           路由链继续）；可声明 PRIORITY = <int>
//!                           （越小越先，缺省 100）
//!   config.yaml             可选：插件配置（运行时状态，不入库）。声明了
//!                           SCHEMA 却缺失 = 未安装：不加载，控制台
//!                           install <名> 按 SCHEMA 生成，enable <名> 启用。
//!                           原始值注入为全局 config（dict），
//!                           enabled: false 可停用插件（宿主保留键，不进 SCHEMA）
//!   SCHEMA                  可选：配置声明 list[dict]，配合
//!                           import loquat_cfglib（extensions/_lib/config.py）
//!                           做校验/默认值/install 的模板生成
//!   同目录其他 .py           可被 plugin.py import（勿与标准库/已装包重名），
//!                           改动同样触发热重载
//!   .disabled               运行时启停标记（enable/disable 与 install 写入/删除）
//!   全局 core                宿主注入的能力桥（pybridge 注入），重载后自动重挂
//!
//! 路由语义：按 (PRIORITY, 目录名) 排序逐个调用，第一个返回非空的插件接管。

use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::{log_error, log_info};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::pluginload;
use crate::pybridge;
use crate::sharedlib;

use crate::pluginload::{file_mtime, has_signature};

const ENTRY_FILE: &str = "plugin.py";
pub(crate) const CONFIG_FILE: &str = "config.yaml";
const DISABLE_MARKER: &str = ".disabled";

/// 传给插件 handle(ctx) 的消息上下文。
pub struct MsgCtx<'a> {
    pub text: &'a str,
    pub message_type: &'a str,
    pub chat_id: i64,
    pub user_id: i64,
    pub self_id: i64,
}

struct LoadedPlugin {
    dir_name: String,
    path: PathBuf, // plugin.py
    priority: i64,
    config_enabled: bool,  // config.yaml 声明
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
    /// 共享库 mtime（毫秒），检测 _lib/config.py 变更并强制全量重载
    lib_mtime: AtomicU64,
    /// 已提示过"可安装"的未安装插件，跳过后续扫描的重复加载检测
    installable: Mutex<Vec<String>>,
}

impl Business {
    pub fn load(dir: &str) -> Result<Self, Box<dyn Error>> {
        let business = Self {
            dir: dir.into(),
            plugins: Mutex::new(Vec::new()),
            reloads: AtomicU64::new(0),
            lib_mtime: AtomicU64::new(0),
            installable: Mutex::new(Vec::new()),
        };
        business.sync()?;
        Ok(business)
    }

    /// 重新扫描插件目录：发现新插件、重载有改动的（plugin.py 或 config.yaml）、
    /// 移除已删除的。单个插件加载失败只跳过自身，不拖垮整条链。
    fn sync(&self) -> Result<(), Box<dyn Error>> {
        let mut plugins = self.plugins.lock().expect("业务锁中毒");

        // 共享库变更：重注册后把所有插件标记为待重载，重新绑定 import
        let mt = sharedlib::lib_mtime_millis(&self.dir);
        if mt != self.lib_mtime.load(Ordering::Relaxed) {
            match sharedlib::load(&self.dir) {
                Ok(_) => log_info!("共享库已加载/更新（_lib 全量重注册）"),
                Err(e) => log_error!("共享库加载失败，import 共享库的插件将不可用: {e}"),
            }
            self.lib_mtime.store(mt, Ordering::Relaxed);
            for p in plugins.iter_mut() {
                p.mtime = SystemTime::UNIX_EPOCH;
            }
        }

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
            // 签名只挡新目录；已加载插件不因事后抹掉签名而凭空卸载
            if !plugins.iter().any(|p| p.dir_name == name) && !has_signature(&entry_py) {
                continue;
            }
            let mut mtime = file_mtime(&entry_py).unwrap_or(SystemTime::UNIX_EPOCH);
            let config_path = dir_path.join(CONFIG_FILE);
            if let Ok(mt) = file_mtime(&config_path) {
                if mt > mtime {
                    mtime = mt;
                }
            }
            // 同目录其余 .py 也是插件的一部分，改动即重载
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p != entry_py && p.extension().is_some_and(|x| x == "py") {
                        if let Ok(mt) = file_mtime(&p) {
                            if mt > mtime {
                                mtime = mt;
                            }
                        }
                    }
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

            let Some(idx) = plugins.iter().position(|p| p.dir_name == name) else {
                // 未安装插件：提示一次后跳过加载，直到 install 或手动放好 config.yaml
                if self.gated_as_installable(&name) {
                    continue;
                }
                match pluginload::load_module(&path) {
                    // 可安装：模块只为探测 SCHEMA 而加载，用完即弃
                    Ok((_, _, _, true)) => self.note_installable(&name),
                    Ok((module, priority, config_enabled, _)) => {
                        let enabled = config_enabled && !marker_now;
                        log_info!(
                            "插件已加载: {name}（优先级 {priority}，{}）",
                            if enabled { "启用" } else { "禁用" }
                        );
                        plugins.push(LoadedPlugin {
                            dir_name: name,
                            path,
                            priority,
                            config_enabled,
                            marker_disabled: marker_now,
                            module,
                            mtime,
                        });
                    }
                    Err(e) => log_error!("插件 {name} 加载失败，已跳过: {e}"),
                }
                continue;
            };

            let (config_changed, toggled) = {
                let existing = &plugins[idx];
                (
                    mtime > existing.mtime,
                    marker_now != existing.marker_disabled,
                )
            };
            if !config_changed && !toggled {
                continue;
            }
            if config_changed {
                match pluginload::load_module(&path) {
                    Ok((module, priority, config_enabled, installable)) => {
                        if installable {
                            // config.yaml 被删：回到未安装，卸载等 install
                            plugins.remove(idx);
                            self.note_installable(&name);
                            continue;
                        }
                        let existing = &mut plugins[idx];
                        existing.module = module;
                        existing.priority = priority;
                        existing.config_enabled = config_enabled;
                        existing.mtime = mtime;
                        self.reloads.fetch_add(1, Ordering::Relaxed);
                        pybridge::collect_garbage();
                        log_info!(
                            "插件已热重载: {name}（第 {} 次）",
                            self.reloads.load(Ordering::Relaxed)
                        );
                    }
                    Err(e) => {
                        log_error!("插件 {name} 重载失败，沿用旧版本: {e}");
                        continue;
                    }
                }
            }
            if toggled {
                let existing = &mut plugins[idx];
                existing.marker_disabled = marker_now;
                log_info!("插件 {name} 已{}", if marker_now { "禁用" } else { "启用" });
            }
        }

        plugins.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.dir_name.cmp(&b.dir_name))
        });
        Ok(())
    }

    /// 责任链路由：按优先级逐个调用，第一个返回非空的插件接管。
    /// 每条消息前惰性同步一次插件目录（扫描开销可忽略）。
    pub fn handle(&self, ctx: &MsgCtx) -> Result<Vec<String>, Box<dyn Error>> {
        if let Err(e) = self.sync() {
            log_error!("插件目录扫描失败（沿用已加载插件）: {e}");
        }
        let plugins = self.plugins.lock().expect("业务锁中毒");
        for plugin in plugins.iter().filter(|p| p.enabled()) {
            match Self::call(&plugin.module, ctx) {
                Ok(replies) if !replies.is_empty() => return Ok(replies),
                Ok(_) => continue,
                Err(e) => log_error!("插件 {} 处理出错: {e}", plugin.dir_name),
            }
        }
        Ok(Vec::new())
    }

    fn call(module: &Py<PyModule>, ctx: &MsgCtx) -> Result<Vec<String>, Box<dyn Error>> {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("text", ctx.text)?;
            dict.set_item("message_type", ctx.message_type)?;
            dict.set_item("chat_id", ctx.chat_id)?;
            dict.set_item("user_id", ctx.user_id)?;
            dict.set_item("self_id", ctx.self_id)?;
            let replies = module.bind(py).call_method1("handle", (dict,))?;
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
            log_error!("写启停标记失败: {e}");
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

    /// 未安装插件是否跳过加载：提示过且 config.yaml 仍缺失则返回 true；
    /// config.yaml 出现（手动安装）则移出提示集合，放行正常加载。
    fn gated_as_installable(&self, name: &str) -> bool {
        let mut hinted = self.installable.lock().expect("业务锁中毒");
        if !hinted.iter().any(|n| n == name) {
            return false;
        }
        if self.dir.join(name).join(CONFIG_FILE).is_file() {
            hinted.retain(|n| n != name);
            return false;
        }
        true
    }

    /// 可安装插件首次发现时提示一次，后续扫描不再重复。
    fn note_installable(&self, name: &str) {
        self.installable
            .lock()
            .expect("业务锁中毒")
            .push(name.to_string());
        log_info!("检测到可安装插件: {name}，运行 install {name} 完成安装");
    }

    /// 安装插件：按 SCHEMA 生成 config.yaml，写 .disabled 保持未启用，
    /// 由 enable 激活。返回给控制台打印的消息。
    pub fn install(&self, name: &str) -> Result<String, String> {
        let dir_path = self.dir.join(name);
        let entry_py = dir_path.join(ENTRY_FILE);
        if !entry_py.is_file() {
            return Err(format!("未找到插件: extensions/{name}/plugin.py"));
        }
        if dir_path.join(CONFIG_FILE).is_file() {
            return Err(format!("{name} 已安装（config.yaml 已存在）"));
        }
        // 加载只为探测 SCHEMA；模块用完即弃，下轮扫描会正式加载
        let loaded =
            pluginload::load_module(&entry_py).map_err(|e| format!("加载插件失败: {e}"))?;
        if !loaded.3 {
            return Err(format!("{name} 未声明 SCHEMA，无需安装"));
        }
        Python::attach(|py| -> Result<(), String> {
            sharedlib::ensure_config_file(&dir_path, loaded.0.bind(py))
                .map(|_| ())
                .map_err(|e| format!("生成 config.yaml 失败: {e}"))
        })?;
        if let Err(e) = std::fs::write(dir_path.join(DISABLE_MARKER), "") {
            return Err(format!("写未启用标记失败: {e}"));
        }
        self.installable
            .lock()
            .expect("业务锁中毒")
            .retain(|n| n != name);
        // 立即加载，让 plugins/enable 马上可见可用，不等下一条消息触发扫描
        let _ = self.sync();
        Ok(format!(
            "{name} 安装成功（config.yaml 已生成，当前未启用），运行 enable {name} 启用"
        ))
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_agent;
