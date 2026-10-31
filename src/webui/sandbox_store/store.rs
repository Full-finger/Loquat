//! Store：沙箱会话的状态与落盘（index.json + <chat_id>.jsonl）。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::log_error;

use super::util::{
    clamp_name, hash_hex, b64_decode, media_ext, now_ms, read_lines, short_id, split_data_url,
    text_of,
};
use super::{Session, DEFAULT_GROUP, DEFAULT_PRIVATE, INDEX_FILE, PLACEHOLDER, RECORD_VERSION};

/// 一个会话库。base 是数据目录（生产 `data`，测试用临时目录）。
pub struct Store {
    base: PathBuf,
    sessions: Vec<Session>,
    keep: usize,
    max_sessions: usize,
}

impl Store {
    pub fn new(base: impl Into<PathBuf>, keep: usize, max_sessions: usize) -> Store {
        let mut s = Store {
            base: base.into(),
            sessions: Vec::new(),
            keep: keep.max(1),
            max_sessions: max_sessions.max(1),
        };
        if let Err(e) = s.load() {
            log_error!("沙箱记录索引加载失败（从空开始）: {e}");
        }
        s
    }

    pub(super) fn dir(&self) -> PathBuf {
        self.base.join("sandbox")
    }

    fn index_path(&self) -> PathBuf {
        self.dir().join(INDEX_FILE)
    }

    fn record_path(&self, chat_id: &str) -> PathBuf {
        self.dir().join(format!("{chat_id}.jsonl"))
    }

    pub(super) fn media_dir(&self) -> PathBuf {
        self.dir().join("media")
    }

    /// 机器人上下文文件（插件侧维护），删会话时一并清掉
    fn agent_path(&self, s: &Session) -> PathBuf {
        self.base.join("sessions").join(format!("{}_{}.json", s.mode, s.chat_id))
    }

    fn load(&mut self) -> Result<(), String> {
        fs::create_dir_all(self.dir()).map_err(|e| e.to_string())?;
        let text = match fs::read_to_string(self.index_path()) {
            Ok(t) => t,
            Err(_) => {
                self.seed(); // 首次运行：把两个缺省会话登记进来
                return self.save();
            }
        };
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("index.json 解析失败: {e}"))?;
        self.sessions = v
            .get("sessions")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Session::from_json).collect())
            .unwrap_or_default();
        if self.sessions.is_empty() {
            self.seed();
            return self.save();
        }
        Ok(())
    }

    fn seed(&mut self) {
        let now = now_ms();
        let mk = |mode: &str, chat_id: &str, name: &str| Session {
            chat_id: chat_id.to_string(),
            mode: mode.to_string(),
            name: name.to_string(),
            created: now,
            updated: now,
            count: 0,
            auto_named: false,
        };
        self.sessions = vec![
            mk("private", DEFAULT_PRIVATE, "默认私聊"),
            mk("group", DEFAULT_GROUP, "默认群聊"),
        ];
    }

    fn save(&self) -> Result<(), String> {
        let payload = json!({
            "version": 1,
            "sessions": self.sessions.iter().map(Session::to_json).collect::<Vec<_>>(),
        });
        let path = self.index_path();
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    pub fn list(&self) -> Vec<Session> {
        let mut v = self.sessions.clone();
        v.sort_by(|a, b| b.updated.cmp(&a.updated));
        v
    }

    pub fn get(&self, chat_id: &str) -> Option<Session> {
        self.sessions.iter().find(|s| s.chat_id == chat_id).cloned()
    }

    pub fn exists(&self, chat_id: &str) -> bool {
        self.get(chat_id).is_some()
    }

    pub fn create(&mut self, mode: &str) -> Result<Session, String> {
        if mode != "private" && mode != "group" {
            return Err(format!("未知 mode {mode}"));
        }
        let base = if mode == "group" { DEFAULT_GROUP } else { DEFAULT_PRIVATE };
        // 同毫秒重复创建也要拿到不同 id：时间戳 + 自增序号
        let mut attempt = 0;
        let chat_id = loop {
            let candidate = format!("{base}.{}", short_id(attempt));
            if !self.sessions.iter().any(|s| s.chat_id == candidate) {
                break candidate;
            }
            attempt += 1;
            if attempt > 64 {
                return Err("会话 ID 生成失败".to_string());
            }
        };
        let now = now_ms();
        let s = Session {
            chat_id,
            mode: mode.to_string(),
            name: PLACEHOLDER.to_string(),
            created: now,
            updated: now,
            count: 0,
            auto_named: true,
        };
        self.sessions.push(s.clone());
        self.enforce_max();
        self.save()?;
        Ok(s)
    }

    pub fn rename(&mut self, chat_id: &str, name: &str) -> Result<Session, String> {
        let name = clamp_name(name);
        if name.is_empty() {
            return Err("名字不能为空".to_string());
        }
        let Some(s) = self.sessions.iter_mut().find(|s| s.chat_id == chat_id) else {
            return Err("会话不存在".to_string());
        };
        s.name = name;
        s.auto_named = false;
        let out = s.clone();
        self.save()?;
        Ok(out)
    }

    pub fn remove(&mut self, chat_id: &str) -> Result<(), String> {
        let Some(pos) = self.sessions.iter().position(|s| s.chat_id == chat_id) else {
            return Err("会话不存在".to_string());
        };
        let s = self.sessions.remove(pos);
        // 记录、机器人上下文都清掉；媒体文件按内容哈希共享，不删（可能被别的会话引用）
        let _ = fs::remove_file(self.record_path(&s.chat_id));
        let _ = fs::remove_file(self.agent_path(&s));
        self.save()
    }

    /// 追加一条「我发的」消息：段结构与平台消息 ID 都留着，
    /// 图片的 dataURL 落盘换成引用，避免把 base64 塞进 JSONL。
    pub fn record_in(&mut self, chat_id: &str, msg_id: &str, segments: &Value) -> Result<Session, String> {
        let mut stored = segments.clone();
        self.persist_media(&mut stored);
        self.append(
            chat_id,
            json!({
                "v": RECORD_VERSION,
                "ts": now_ms(),
                "dir": "in",
                "id": msg_id,
                "segments": stored,
            }),
            text_of(segments),
        )
    }

    /// 追加一条机器人回复（WS 帧只有纯文本）。
    pub fn record_out(&mut self, chat_id: &str, mode: &str, text: &str) -> Result<Session, String> {
        self.ensure(chat_id, mode)?;
        self.append(
            chat_id,
            json!({
                "v": RECORD_VERSION,
                "ts": now_ms(),
                "dir": "out",
                "text": text,
            }),
            "",
        )
    }

    /// 老 chat_id（未登记）也要能记账：按 mode 补一条缺省名会话。
    fn ensure(&mut self, chat_id: &str, mode: &str) -> Result<(), String> {
        if self.exists(chat_id) {
            return Ok(());
        }
        if mode != "private" && mode != "group" {
            return Err(format!("未知 mode {mode}"));
        }
        let now = now_ms();
        self.sessions.push(Session {
            chat_id: chat_id.to_string(),
            mode: mode.to_string(),
            name: if mode == "group" { "默认群聊" } else { "默认私聊" }.to_string(),
            created: now,
            updated: now,
            count: 0,
            auto_named: false,
        });
        self.save()
    }

    fn append(&mut self, chat_id: &str, line: Value, name_hint: &str) -> Result<Session, String> {
        let Some(idx) = self.sessions.iter().position(|s| s.chat_id == chat_id) else {
            return Err("会话不存在".to_string());
        };
        // 首条消息把占位名换掉：默认取前几个字（需求里定的规则）
        if self.sessions[idx].auto_named && !name_hint.is_empty() {
            self.sessions[idx].name = clamp_name(name_hint);
            self.sessions[idx].auto_named = false;
        }
        self.sessions[idx].updated = now_ms();
        self.sessions[idx].count += 1;

        fs::create_dir_all(self.dir()).map_err(|e| e.to_string())?;
        let path = self.record_path(chat_id);
        let mut text = line.to_string();
        text.push('\n');
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;

        // 超过两倍上限才压缩，摊薄重写成本
        if self.sessions[idx].count as usize > self.keep * 2 {
            if let Ok(lines) = read_lines(&path) {
                let kept: Vec<String> = lines.iter().rev().take(self.keep).rev().cloned().collect();
                let mut body = kept.join("\n");
                body.push('\n');
                if fs::write(&path, body).is_ok() {
                    self.sessions[idx].count = kept.len() as u64;
                }
            }
        }
        let out = self.sessions[idx].clone();
        self.save()?;
        Ok(out)
    }

    /// 读历史：返回最近的 limit 条（正序），坏行跳过。
    pub fn history(&self, chat_id: &str, limit: usize) -> Vec<Value> {
        let Ok(lines) = read_lines(&self.record_path(chat_id)) else {
            return Vec::new();
        };
        let limit = limit.clamp(1, 2000);
        let start = lines.len().saturating_sub(limit);
        lines[start..]
            .iter()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter(|v| v.get("v").and_then(Value::as_u64) == Some(RECORD_VERSION))
            .collect()
    }

    /// dataURL 图片落盘，段里的 url 换成可访问的相对路径。
    fn persist_media(&self, segments: &mut Value) {
        let Some(arr) = segments.as_array_mut() else {
            return;
        };
        for seg in arr.iter_mut() {
            let Some(url) = seg
                .get_mut("image")
                .and_then(|i| i.get_mut("url"))
                .and_then(|u| u.as_str().map(String::from))
            else {
                continue;
            };
            if !url.starts_with("data:") {
                continue;
            }
            let Some((mime, b64)) = split_data_url(&url) else {
                continue;
            };
            let Some(bytes) = b64_decode(b64) else {
                continue;
            };
            let ext = media_ext(&mime);
            let name = format!("{}.{ext}", hash_hex(&bytes));
            let path = self.media_dir().join(&name);
            if !path.exists() && fs::create_dir_all(self.media_dir()).is_ok() {
                let _ = fs::write(&path, &bytes);
            }
            if let Some(obj) = seg.get_mut("image").and_then(Value::as_object_mut) {
                obj.insert(
                    "url".to_string(),
                    Value::String(format!("/api/sandbox/media/{name}")),
                );
            }
        }
    }

    fn enforce_max(&mut self) {
        if self.sessions.len() <= self.max_sessions {
            return;
        }
        let mut by_age: Vec<(usize, u64)> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.updated))
            .collect();
        by_age.sort_by_key(|(_, t)| *t);
        let drop_n = self.sessions.len() - self.max_sessions;
        // 从最旧开始删；倒序删除保证下标不失效
        let mut victims: Vec<usize> = by_age.into_iter().take(drop_n).map(|(i, _)| i).collect();
        victims.sort_unstable_by(|a, b| b.cmp(a));
        for i in victims {
            let s = self.sessions.remove(i);
            let _ = fs::remove_file(self.record_path(&s.chat_id));
            let _ = fs::remove_file(self.agent_path(&s));
        }
    }
}
