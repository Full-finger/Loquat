//! 沙箱会话记录：WebUI 沙箱选项卡的「对话历史 + 会话列表」。
//!
//! 和机器人上下文是两套数据，用同一个 chat_id 对齐：
//!   - `data/sessions/<mode>_<chat_id>.json`：插件写给模型看的上下文
//!     （含 tool_calls、会被裁剪），由 extensions/_lib/agent.py 维护；
//!   - `data/sandbox/<chat_id>.jsonl`：本模块维护的「展示用」记录，
//!     带段结构、图片引用与时间戳，只给 WebUI 渲染。
//!
//! WebUI 的「新建会话」= 新 chat_id → 新记忆文件 + 新记录文件，两边天然隔离，
//! 所以切回旧会话时机器人的上下文也是旧的，不会串味。
//!
//! 之所以不用 SQLite：单机、单用户、每会话几百条，JSONL 追加写足够，
//! 且坏一行只丢一条（读取时跳过），不需要迁移工具。
//!
//! 文件划分：store.rs 是 Store（状态与落盘）、util.rs 是编解码小工具、
//! tests.rs 是测试。R4 规则限单文件 400 行。

mod store;
#[cfg(test)]
mod tests;
mod util;

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

use crate::log_error;

use store::Store;

/// 缺省会话的 chat_id：与 adapter-sandbox 的固定值一致。
/// 保留它们是为了让老数据（已存在的机器人记忆）继续可见可用。
pub const DEFAULT_PRIVATE: &str = "10000";
pub const DEFAULT_GROUP: &str = "dev-group";

const INDEX_FILE: &str = "index.json";
const RECORD_VERSION: u64 = 1;
/// 名字还没被首条消息定下来（新建会话的占位名）
const PLACEHOLDER: &str = "新会话";
const NAME_MAX_CHARS: usize = 40;

#[derive(Clone, Debug)]
pub struct Session {
    pub chat_id: String,
    pub mode: String,
    pub name: String,
    pub created: u64,
    pub updated: u64,
    /// 条数缓存：只在追加时自增，避免每次列表都扫文件
    pub count: u64,
    /// 名字还是占位名，首条消息到来时自动改成前几个字
    pub auto_named: bool,
}

impl Session {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "chatId": self.chat_id,
            "mode": self.mode,
            "name": self.name,
            "created": self.created,
            "updated": self.updated,
            "count": self.count,
            // 名字是否还是占位名：前端据此刻「新会话」标记，重载后不能丢
            "autoNamed": self.auto_named,
        })
    }

    pub(super) fn from_json(v: &Value) -> Option<Session> {
        Some(Session {
            chat_id: v.get("chatId")?.as_str()?.to_string(),
            mode: v.get("mode")?.as_str()?.to_string(),
            name: v.get("name")?.as_str()?.to_string(),
            created: v.get("created").and_then(Value::as_u64).unwrap_or(0),
            updated: v.get("updated").and_then(Value::as_u64).unwrap_or(0),
            count: v.get("count").and_then(Value::as_u64).unwrap_or(0),
            auto_named: v.get("autoNamed").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

// ---- 全局单例：REST handler 与 worker 线程共用 ----

static GLOBAL: OnceLock<Mutex<Store>> = OnceLock::new();

/// 启动时调用一次；未调用时首个访问者用缺省参数惰性初始化。
pub fn init(keep: usize, max_sessions: usize) {
    GLOBAL.get_or_init(|| Mutex::new(Store::new("data", keep, max_sessions)));
}

fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let m = GLOBAL.get_or_init(|| Mutex::new(Store::new("data", 500, 50)));
    let mut guard = match m.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(), // 记录损坏不该连带整个 UI 挂掉
    };
    f(&mut guard)
}

pub fn list() -> Vec<Value> {
    with(|s| s.list().iter().map(Session::to_json).collect())
}

pub fn create(mode: &str) -> Result<Value, String> {
    with(|s| s.create(mode)).map(|s| s.to_json())
}

pub fn rename(chat_id: &str, name: &str) -> Result<Value, String> {
    with(|s| s.rename(chat_id, name)).map(|s| s.to_json())
}

pub fn remove(chat_id: &str) -> Result<(), String> {
    with(|s| s.remove(chat_id))
}

pub fn exists(chat_id: &str) -> bool {
    with(|s| s.exists(chat_id))
}

pub fn history(chat_id: &str, limit: usize) -> Vec<Value> {
    with(|s| s.history(chat_id, limit))
}

pub fn record_in(chat_id: &str, msg_id: &str, segments: &Value) {
    if let Err(e) = with(|s| s.record_in(chat_id, msg_id, segments)) {
        log_error!("沙箱记录写入失败 {chat_id}: {e}");
    }
}

pub fn record_out(chat_id: &str, mode: &str, text: &str) {
    if let Err(e) = with(|s| s.record_out(chat_id, mode, text)) {
        log_error!("沙箱回复记录写入失败 {chat_id}: {e}");
    }
}

/// 媒体文件绝对路径（防目录穿越：只认裸文件名）。
pub fn media_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    let p = with(|s| s.media_dir().join(name));
    p.is_file().then_some(p)
}
