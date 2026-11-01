//! 插件目录扫描与热重载（Business::sync）：发现新插件、重载有改动的、
//! 移除已删除的。从 mod.rs 迁出：R4 规则限 400 行。

use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::SystemTime;

use super::recover_poison;
use super::Business;
use super::LoadedPlugin;
use super::{CONFIG_FILE, DISABLE_MARKER, ENTRY_FILE};
use crate::pluginload::{self, file_mtime, has_signature, Loaded};
use crate::{log_error, log_info, pybridge, sharedlib};

impl Business {
    /// 重新扫描插件目录：发现新插件、重载有改动的（plugin.py 或 config.yaml）、
    /// 移除已删除的。单个插件加载失败只跳过自身，不拖垮整条链。
    pub(super) fn sync(&self) -> Result<(), Box<dyn Error>> {
        let mut plugins = self.plugins.write().unwrap_or_else(recover_poison);

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
                // 加载失败的插件在退避期内不再重试（建议 1：避免每轮扫描重编译）
                let now = super::now_ms();
                if self.in_backoff(&name, now) {
                    continue;
                }
                // 未安装插件：提示一次后跳过加载，直到 install 或手动放好 config.yaml
                if self.gated_as_installable(&name) {
                    continue;
                }
                match pluginload::load_module(&path) {
                    // 可安装：模块只为探测 SCHEMA 而加载，用完即弃
                    Ok(Loaded {
                        installable: true, ..
                    }) => {
                        self.clear_failed(&name);
                        self.note_installable(&name);
                    }
                    Ok(Loaded {
                        module,
                        priority,
                        config_enabled,
                        api_version,
                        ..
                    }) => {
                        self.clear_failed(&name);
                        let enabled = config_enabled && !marker_now;
                        log_info!(
                            "插件已加载: {name}（优先级 {priority}，API v{api_version}，{}）",
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
                            api_version,
                        });
                    }
                    Err(e) => {
                        self.note_failed(&name, now);
                        log_error!("插件 {name} 加载失败，已跳过（退避后重试）: {e}");
                    }
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
                    Ok(Loaded {
                        module,
                        priority,
                        config_enabled,
                        installable,
                        api_version,
                    }) => {
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
                        existing.api_version = api_version;
                        self.reloads.fetch_add(1, Ordering::Relaxed);
                        // 新模块重新获得被调用的机会（建议 4 的卡死退避到此为止）
                        crate::watchdog::clear_stuck(&name);
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
}
