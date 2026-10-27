//! 业务插件加载器：扫描 extensions/<名>/plugin.py，责任链路由，
//! 逐插件惰性热重载。Python 注入层（能力桥/配置挂载/SIGINT 屏蔽）
//! 见 src/pybridge.rs。
//!
//! 插件约定（extensions/<名>/ 目录，自包含）：
//!   plugin.py               入口：首行必须是签名 `# loquat-plugin`（宿主
//!                           识别插件的依据，无签名目录忽略）；必须实现
//!                           handle(ctx) -> list[str]
//!                           （ctx: dict，含 text/chat/segments/
//!                           user_id/self_id/message_id/is_at_bot/images；
//!                           返回空列表 = 不归我管，路由链继续）；
//!                           可声明 PRIORITY = <int>（越小越先，缺省 100）
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

use crate::event::Chat;
use crate::pluginload;
use crate::pybridge;
use crate::sharedlib;

// SCHEMA（python 对象）→ JSON 文本：经 json.dumps 中转，避开逐类型 IntoPyObject
fn schema_to_json(loaded: &LoadedPlugin) -> String {
    Python::attach(|py| {
        match loaded.module.bind(py).getattr("SCHEMA") {
            Ok(schema) => {
                let json_mod = match py.import("json") {
                    Ok(m) => m,
                    Err(_) => return "[]".to_string(),
                };
                match json_mod.call_method1("dumps", (schema,)) {
                    Ok(s) => s.extract::<String>().unwrap_or_else(|_| "[]".into()),
                    Err(_) => "[]".to_string(),
                }
            }
            Err(_) => "[]".to_string(),
        }
    })
}

// python 对象 → JSON 文本
fn py_obj_to_json(obj: &pyo3::Bound<'_, pyo3::types::PyAny>) -> String {
    let py = obj.py();
    match py.import("json").and_then(|j| j.call_method1("dumps", (obj,))) {
        Ok(s) => s.extract::<String>().unwrap_or_else(|_| "[]".into()),
        Err(_) => "[]".to_string(),
    }
}

// JSON → python 对象（json.loads）
fn json_to_py<'py>(
    py: pyo3::Python<'py>,
    v: &serde_json::Value,
) -> Result<pyo3::Bound<'py, pyo3::types::PyAny>, String> {
    let text = serde_json::to_string(v).map_err(|e| e.to_string())?;
    py.import("json")
        .and_then(|j| j.call_method1("loads", (text,)))
        .map_err(|e| e.to_string())
}

// 极简 YAML → JSON（标量/一层嵌套）：插件 config.yaml 都是扁平键值。
// 复杂结构交给前端的 list/dict 编辑器序列化成标准 YAML，这里只读。
fn serde_yamlish(text: &str) -> Result<serde_json::Value, String> {
    let docs = yaml_rust2::YamlLoader::load_from_str(text).map_err(|e| e.to_string())?;
    let doc = docs.into_iter().next().ok_or("空文档")?;
    Ok(yaml_to_json_value(&doc))
}

fn yaml_to_json_value(v: &yaml_rust2::Yaml) -> serde_json::Value {
    use yaml_rust2::Yaml;
    match v {
        Yaml::Real(s) => {
            serde_json::Value::Number(s.parse().unwrap_or(serde_json::Number::from(0)))
        }
        Yaml::Integer(i) => serde_json::json!(i),
        Yaml::Boolean(b) => serde_json::json!(b),
        Yaml::String(s) => serde_json::json!(s),
        Yaml::Array(a) => serde_json::Value::Array(a.iter().map(yaml_to_json_value).collect()),
        Yaml::Hash(h) => {
            let mut m = serde_json::Map::new();
            for (k, val) in h {
                let key = match k {
                    Yaml::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                m.insert(key, yaml_to_json_value(val));
            }
            serde_json::Value::Object(m)
        }
        Yaml::Null => serde_json::Value::Null,
        other => serde_json::json!(format!("{other:?}")),
    }
}

use crate::pluginload::{file_mtime, has_signature};

const ENTRY_FILE: &str = "plugin.py";
pub(crate) const CONFIG_FILE: &str = "config.yaml";
const DISABLE_MARKER: &str = ".disabled";

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
        // 登记"哪里来"：插件经能力桥发逃生舱动作时按此路由平台
        pybridge::set_current_platform(ctx.platform);
        let result = Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("text", ctx.text)?;
            dict.set_item("message_type", ctx.chat.message_type())?;
            dict.set_item("chat_id", ctx.chat.id())?;
            dict.set_item("user_id", ctx.sender_id)?;
            dict.set_item("self_id", ctx.self_id)?;
            dict.set_item("message_id", ctx.message_id)?;
            dict.set_item("is_at_bot", ctx.mentions_bot)?;
            dict.set_item("images", ctx.images.to_vec())?;
            // serde_json::Value 无 IntoPyObject，走 json 字符串中转（同 pybridge 的 config 注入）
            let segments_json = serde_json::to_string(ctx.segments)?;
            let json_mod = py.import("json")?;
            let segments = json_mod.call_method1("loads", (segments_json,))?;
            dict.set_item("segments", segments)?;
            dict.set_item("platform", ctx.platform)?;
            let replies = module.bind(py).call_method1("handle", (dict,))?;
            Ok(replies.extract()?)
        });
        pybridge::clear_current_platform();
        result
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

    /// 插件配置读取：SCHEMA（字段声明）+ 当前值（config.yaml，未安装给默认值）。
    /// 前端按 schema 渲染编辑表单。
    pub fn plugin_config(&self, name: &str) -> Result<serde_json::Value, String> {
        let dir_path = self.dir.join(name);
        if !dir_path.join(ENTRY_FILE).is_file() {
            return Err(format!("未找到插件: extensions/{name}/plugin.py"));
        }
        // SCHEMA：已加载的插件直接读内存模块，未加载临时编译一个
        let schema_json = {
            let plugins = self.plugins.lock().expect("业务锁中毒");
            match plugins.iter().find(|p| p.dir_name == name) {
                Some(p) => schema_to_json(p),
                None => {
                    drop(plugins);
                    let loaded = pluginload::load_module(&dir_path.join(ENTRY_FILE))
                        .map_err(|e| format!("加载插件失败: {e}"))?;
                    Python::attach(|py| -> Result<String, String> {
                        let schema_any = loaded.0.bind(py).getattr("SCHEMA");
                        if schema_any.is_err() {
                            return Err(format!("{name} 未声明 SCHEMA，无可编辑配置"));
                        }
                        Ok(py_obj_to_json(&schema_any.unwrap()))
                    })?
                }
            }
        };
        let schema: serde_json::Value =
            serde_json::from_str(&schema_json).map_err(|e| format!("SCHEMA 转换失败: {e}"))?;

        // 当前值：config.yaml 存在则解析，缺失（未安装）用 SCHEMA 默认值
        let cfg_path = dir_path.join(CONFIG_FILE);
        let mut values = serde_json::Map::new();
        if let Ok(text) = std::fs::read_to_string(&cfg_path) {
            if let Ok(v) = serde_yamlish(&text) {
                if let serde_json::Value::Object(map) = v {
                    values = map;
                }
            }
        } else if let serde_json::Value::Array(fields) = &schema {
            for f in fields {
                if let (Some(k), Some(d)) = (f.get("key"), f.get("default")) {
                    values.insert(k.as_str().unwrap_or("").to_string(), d.clone());
                }
            }
        }
        let installed = cfg_path.is_file();
        let enabled = values.get("enabled").and_then(|b| b.as_bool()).unwrap_or(true);
        values.remove("enabled"); // 宿主保留键，表单不显示
        Ok(serde_json::json!({
            "schema": schema,
            "values": values,
            "installed": installed,
            "enabled": enabled,
        }))
    }

    /// 插件配置保存：经 cfglib 校验渲染成带注释 YAML 写回。
    /// mtime 变化会让责任链热重载插件，改动即时生效。
    pub fn save_plugin_config(&self, name: &str, values: serde_json::Value) -> Result<(), String> {
        let dir_path = self.dir.join(name);
        let cfg_path = dir_path.join(CONFIG_FILE);
        let loaded = pluginload::load_module(&dir_path.join(ENTRY_FILE))
            .map_err(|e| format!("加载插件失败: {e}"))?;
        // 注意 loaded.3 是 installable（声明 SCHEMA 且 config.yaml 缺失），
        // 已安装的插件恒为 false，不能当“有无 SCHEMA”用——直接探测属性
        let has_schema = Python::attach(|py| loaded.0.bind(py).getattr("SCHEMA").is_ok());
        if !has_schema {
            return Err(format!("{name} 未声明 SCHEMA，无可保存配置"));
        }
        // enabled 是宿主保留键：不在 SCHEMA 里，原样保留
        let had_enabled = std::fs::read_to_string(&cfg_path)
            .ok()
            .and_then(|t| serde_yamlish(&t).ok())
            .and_then(|v| v.get("enabled").cloned())
            .and_then(|b| b.as_bool())
            .unwrap_or(true);
        let mut py_values = values;
        py_values["enabled"] = serde_json::Value::Bool(had_enabled);
        let yaml_text = Python::attach(|py| -> Result<String, String> {
            let schema = loaded.0.bind(py).getattr("SCHEMA").map_err(|e| e.to_string())?;
            let cfglib = py
                .import("loquat_cfglib")
                .map_err(|e| format!("cfglib 未注册: {e}"))?;
            let values_dict = json_to_py(py, &py_values)?;
            cfglib
                .call_method1("render", (schema, values_dict))
                .and_then(|t| t.extract::<String>())
                .map_err(|e| format!("渲染配置失败: {e}"))
        })?;
        std::fs::write(&cfg_path, yaml_text).map_err(|e| format!("写 config.yaml 失败: {e}"))?;
        let _ = self.sync(); // 触发热重载，改动即时生效
        Ok(())
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

    /// WebUI 插件列表：已加载插件 + 已发现未安装的目录。
    /// 返回 (名称, 已安装, 已启用, 优先级)。未安装的优先级未知，报缺省 100。
    pub fn plugin_rows(&self) -> Vec<(String, bool, bool, i64)> {
        let plugins = self.plugins.lock().expect("业务锁中毒");
        let installable = self.installable.lock().expect("业务锁中毒");
        let mut rows: Vec<(String, bool, bool, i64)> = plugins
            .iter()
            .map(|p| (p.dir_name.clone(), true, p.enabled(), p.priority))
            .collect();
        for name in installable.iter() {
            if !rows.iter().any(|(n, ..)| n == name) {
                rows.push((name.clone(), false, false, 100));
            }
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
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

#[cfg(test)]
mod tests_webtools;
