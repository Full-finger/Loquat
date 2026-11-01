//! 业务插件加载器：扫描 extensions/<名>/plugin.py，责任链路由，
//! 逐插件惰性热重载。Python 注入层（能力桥/配置挂载/SIGINT 屏蔽）
//! 见 src/pybridge.rs。
//!
//! 插件约定（extensions/<名>/ 目录，自包含）：
//!   plugin.py               入口：首行必须是签名 `# loquat-plugin`（宿主
//!                           识别插件的依据，无签名目录忽略），可带 API 版本
//!                           `# loquat-plugin v2`（缺省 v1）。钩子（全部可选，
//!                           按优先级执行；v1 的 handle 由宿主垫片为 on_message）：
//!                           on_observe(ctx)        每条消息全部执行，返回值忽略
//!                                                  （历史、审计、埋点，抢答不丢）
//!                           on_message(ctx)        路由：首个非 None/空 接管
//!                           before_send(ctx, segs) 对胜者每条消息逐级 fold：
//!                                                  None 不改 / list 替换 / 空表丢弃
//!                           on_unload()            热重载、移除前清理；disable 不触发
//!                           ctx: dict，含 text/chat/segments/user_id/self_id/
//!                           message_id/is_at_bot/images/deadline_ms——剩余预算
//!                           毫秒数，LLM 类插件据此收敛轮数；到期宿主注入
//!                           TimeoutError 中断，不要用 except Exception 吞掉它
//!                           路由返回：v1 list[str] 每个字符串一条消息；v2
//!                           str | list[段] | None，list 为**一条**消息的扁平段
//!                           列表（protojson 形态，与 ctx["segments"] 同构，
//!                           构造器见 loquat_seg），可声明 API = 2 与签名交叉校验；
//!                           空（[] / None / ""）= 不归我管，链继续；
//!                           可声明 PRIORITY = <int>（越小越先，缺省 100）
//!   config.yaml             可选：插件配置（运行时状态，不入库）。声明了
//!                           SCHEMA 却缺失 = 未安装：不加载，控制台
//!                           install <名> 按 SCHEMA 生成，enable <名> 启用。
//!                           原始值注入为全局 config（dict），
//!                           enabled: false 可停用插件（宿主保留键，不进 SCHEMA）
//!   SCHEMA                  可选：配置声明 list[dict]，配合
//!                           import loquat_cfglib（extensions/_lib/config.py）
//!                           做校验/默认值/install 的模板生成
//!   TABS + ui.js            可选：WebUI 扩展页。TABS 声明侧栏入口
//!                           （{"id", "title", "icon"?}），ui.js 为每个
//!                           tab 注册自定义元素 loquat-page-<名>-<tab id>，
//!                           契约见 docs/ext-ui.md
//!   同目录其他 .py           可被 plugin.py import（勿与标准库/已装包重名），
//!                           改动同样触发热重载
//!   .disabled               运行时启停标记（enable/disable 与 install 写入/删除）
//!   全局 core                宿主注入的能力桥（pybridge 注入），重载后自动重挂
//!
//! 管线语义（评审建议 6，三阶段）：按 (PRIORITY, 目录名) 排序，
//! ① on_observe 全部执行 → ② on_message 首个非空接管 → ③ before_send 逐级 fold 胜者。
//! 实现见 chain.rs。

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant, SystemTime};

use crate::{log_error, log_info, log_warn};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::event::Chat;

mod assets_api;
mod chain;
mod config_api;
mod reply;
mod sync;

pub(crate) use config_api::serde_yamlish;

/// 一条出站消息：扁平段列表（protojson 形态，与入站 ctx["segments"] 同构，
/// 如 [{"text":{"text":"看这个"}}, {"image":{"url":"https://…"}}]）。
/// 出站原样序列化进 SendMessageRequest.segments（评审建议 5）。
pub type Message = Vec<serde_json::Value>;

const ENTRY_FILE: &str = "plugin.py";
pub(crate) const CONFIG_FILE: &str = "config.yaml";
/// WebUI 扩展页脚本（约定入口，存在即视为插件声明了页面）
pub(crate) const UI_FILE: &str = "ui.js";
const DISABLE_MARKER: &str = ".disabled";

/// 热路径节流：距上次目录扫描不足此时长则直接用快照（评审建议 1：
/// 全量扫描实测 2.3 ms/次，是吞吐上限的来源）。管理操作不受限。
const SYNC_INTERVAL_MS: u64 = 500;
/// 加载失败插件的退避：首次 1 s，翻倍到 60 s 封顶（建议 1：
/// 不再每条消息重试编译）。
const BACKOFF_INIT_MS: u64 = 1_000;
const BACKOFF_MAX_MS: u64 = 60_000;
/// 单条消息的插件链总预算（建议 4）：各插件串行消耗，到期由看门狗注入
/// TimeoutError；ctx["deadline_ms"] 给插件的是剩余量。
const PLUGIN_BUDGET_MS: u64 = 30_000;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 传给插件 handle(ctx) 的消息上下文。
/// 字段来自统一信封（event.rs 解析），跨平台通用；
/// 结构化段在 segments（protojson 形态），平台细节从那里取。
pub struct MsgCtx<'a> {
    pub text: &'a str,
    pub chat: Chat,
    pub sender_id: &'a str,
    pub self_id: &'a str,
    /// 触发消息的 ID；平台不回时为空串
    pub message_id: &'a str,
    /// 是否 at 了机器人（适配器判定）
    pub mentions_bot: bool,
    /// 消息里的图片 URL（event.rs 派生，上限 4 张）
    pub images: &'a [String],
    /// 结构化消息段（protojson 形态，如 {"text":{"text":"早"}}）
    pub segments: &'a [serde_json::Value],
    pub platform: &'a str,
    /// 进入插件链的时刻：执行预算从这里起算（建议 4）
    pub started: Instant,
}

struct LoadedPlugin {
    dir_name: String,
    path: PathBuf, // plugin.py
    priority: i64,
    config_enabled: bool,  // config.yaml 声明
    marker_disabled: bool, // .disabled 标记（控制台启停）
    module: Py<PyModule>,
    mtime: SystemTime,
    /// 插件 API 版本（签名行解析），决定 handle 返回值怎么解释
    api_version: u8,
    /// 钩子（on_message/on_observe/before_send/on_unload，v1 handle 垫片为路由入口）
    hooks: crate::pluginload::Hooks,
}

impl LoadedPlugin {
    fn enabled(&self) -> bool {
        self.config_enabled && !self.marker_disabled
    }
}

/// 毒锁恢复：中毒只说明曾有 panic 持锁，数据本身仍可用。
/// 本项目的用锁场景全是路由与统计，恢复继续跑好过砖死（26102701#8）；
/// warn 留痕，事故可观测。
fn recover_poison<T>(poisoned: PoisonError<T>) -> T {
    log_warn!("业务锁中毒，已恢复继续（曾有 panic 持锁）");
    poisoned.into_inner()
}

/// on_unload 的固定短预算：清理钩子不该做重活，卡住就注入打断。
const ON_UNLOAD_BUDGET: Duration = Duration::from_secs(2);

/// 旧模块丢弃前调用 on_unload（热重载、目录/配置被删导致的移除）。
/// disable 有意不触发：没有配对的 init 钩子，清掉的线程/句柄无人负责重建，
/// 半套生命周期比不清理更危险（提案 §4.2 在此偏离）。
/// 在 sync 的写锁内被调：套看门狗 Guard，插件写坏 on_unload 不会拖死整条链。
pub(super) fn fire_on_unload(hooks: &crate::pluginload::Hooks, name: &str) {
    let Some(hook) = &hooks.on_unload else {
        return;
    };
    Python::attach(|py| {
        let Ok(tid) = crate::watchdog::current_py_thread(py) else {
            return;
        };
        let result = {
            let _guard = crate::watchdog::guard(tid, name, ON_UNLOAD_BUDGET);
            hook.bind(py).call0()
        };
        crate::watchdog::clear_pending(py, tid);
        if let Err(e) = result {
            log_error!("插件 {name} on_unload 出错: {e}");
        }
    });
}

pub struct Business {
    dir: PathBuf,
    /// 读多写少：每条消息 handle 只读快照，sync/启停才写。handle 放锁后
    /// 执行插件链，LLM 等长等待不堵其他线程。
    plugins: RwLock<Vec<LoadedPlugin>>,
    reloads: AtomicU64,
    /// 共享库 mtime（毫秒），检测 _lib/config.py 变更并强制全量重载
    lib_mtime: AtomicU64,
    /// 已提示过"可安装"的未安装插件，跳过后续扫描的重复加载检测
    installable: Mutex<Vec<String>>,
    /// 上次目录扫描的毫秒时间戳（热路径节流）
    last_sync: AtomicU64,
    /// 加载失败插件 → 退避到期的毫秒时间戳（到期前不再重试编译）
    failed: Mutex<HashMap<String, u64>>,
    /// 单条消息的插件链总预算（缺省 PLUGIN_BUDGET_MS，测试可调小）
    budget: Duration,
}

impl Business {
    pub fn load(dir: &str) -> Result<Self, Box<dyn Error>> {
        let business = Self {
            dir: dir.into(),
            plugins: RwLock::new(Vec::new()),
            reloads: AtomicU64::new(0),
            lib_mtime: AtomicU64::new(0),
            installable: Mutex::new(Vec::new()),
            last_sync: AtomicU64::new(0),
            failed: Mutex::new(HashMap::new()),
            budget: Duration::from_millis(PLUGIN_BUDGET_MS),
        };
        business.sync()?;
        Ok(business)
    }

    /// 改执行预算（测试用：把 30s 压到几百毫秒让超时路径可测）。
    #[cfg(test)]
    pub(super) fn with_budget(mut self, budget: Duration) -> Self {
        self.budget = budget;
        self
    }

    /// 热路径节流：距上次扫描不足间隔则直接用快照。CAS 抢占保证并发
    /// worker 只放一个进扫描，其余走旧快照（下一条消息自然重试）。
    fn sync_due(&self) -> bool {
        let now = now_ms();
        let last = self.last_sync.load(Ordering::Relaxed);
        now >= last.saturating_add(SYNC_INTERVAL_MS)
            && self
                .last_sync
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
    }

    /// 插件退避未到期：true = 跳过本次加载尝试。
    pub(super) fn in_backoff(&self, name: &str, now: u64) -> bool {
        self.failed
            .lock()
            .unwrap_or_else(recover_poison)
            .get(name)
            .is_some_and(|&until| now < until)
    }

    /// 记录加载失败：退避时长翻倍（1s→2s→…→60s 封顶）。
    pub(super) fn note_failed(&self, name: &str, now: u64) {
        let mut m = self.failed.lock().unwrap_or_else(recover_poison);
        let backoff = m.get(name).map_or(BACKOFF_INIT_MS, |&until| {
            (until.saturating_sub(now) * 2).min(BACKOFF_MAX_MS)
        });
        m.insert(name.to_string(), now + backoff);
    }

    /// 清除退避记录：加载成功、install 完成时调用。
    pub(super) fn clear_failed(&self, name: &str) {
        self.failed
            .lock()
            .unwrap_or_else(recover_poison)
            .remove(name);
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

        let mut plugins = self.plugins.write().unwrap_or_else(recover_poison);
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
        let mut hinted = self.installable.lock().unwrap_or_else(recover_poison);
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
            .unwrap_or_else(recover_poison)
            .push(name.to_string());
        log_info!("检测到可安装插件: {name}，运行 install {name} 完成安装");
    }

    /// WebUI 插件列表：已加载插件 + 已发现未安装的目录。
    /// 返回 (名称, 已安装, 已启用, 优先级, 有WebUI页面)。未安装的优先级未知，
    /// 报缺省 100；has_ui 只看 ui.js 存在性，与安装状态无关。
    pub fn plugin_rows(&self) -> Vec<(String, bool, bool, i64, bool)> {
        let plugins = self.plugins.read().unwrap_or_else(recover_poison);
        let installable = self.installable.lock().unwrap_or_else(recover_poison);
        let mut rows: Vec<(String, bool, bool, i64, bool)> = plugins
            .iter()
            .map(|p| {
                let has_ui = self.dir.join(&p.dir_name).join(UI_FILE).is_file();
                (p.dir_name.clone(), true, p.enabled(), p.priority, has_ui)
            })
            .collect();
        for name in installable.iter() {
            if !rows.iter().any(|(n, ..)| n == name) {
                let has_ui = self.dir.join(name).join(UI_FILE).is_file();
                rows.push((name.clone(), false, false, 100, has_ui));
            }
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    }

    /// 链路概况（控制台 plugins 命令）：插件名带钩子标签，即当前生效的钩子表
    /// （提案验收 #5：不透明，但可枚举）。
    pub fn info(&self) -> String {
        let plugins = self.plugins.read().unwrap_or_else(recover_poison);
        let names: Vec<String> = plugins
            .iter()
            .map(|p| {
                let mut name = format!("{}{}", p.dir_name, p.hooks.tag());
                if !p.enabled() {
                    name.push_str("[禁用]");
                }
                name
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

#[cfg(test)]
mod tests_agent_images;

#[cfg(test)]
mod tests_config;

#[cfg(test)]
mod tests_webtools;

#[cfg(test)]
mod tests_reply;

#[cfg(test)]
mod tests_budget;

#[cfg(test)]
mod tests_llm;

#[cfg(test)]
mod tests_hooks;
