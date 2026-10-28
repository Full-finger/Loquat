//! 插件配置管理面：WebUI/控制台的配置读写、安装，以及支撑它们的
//! Python↔JSON↔YAML 转换助手。路由与责任链在 mod.rs。
//!
//! 转换约定：Python 对象一律经 json.dumps/loads 中转（避开逐类型
//! IntoPyObject）；config.yaml 只读扁平键值（serde_yamlish），
//! 复杂结构由前端编辑器序列化成标准 YAML、cfglib.render 写回。

use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use pyo3::types::PyAny;

use super::{Business, LoadedPlugin, CONFIG_FILE, ENTRY_FILE, UI_FILE};
use crate::log_error;
use crate::pluginload;
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
fn py_obj_to_json(obj: &Bound<'_, PyAny>) -> String {
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
) -> Result<Bound<'py, PyAny>, String> {
    let text = serde_json::to_string(v).map_err(|e| e.to_string())?;
    py.import("json")
        .and_then(|j| j.call_method1("loads", (text,)))
        .map_err(|e| e.to_string())
}

// 极简 YAML → JSON（标量/一层嵌套）：插件 config.yaml 都是扁平键值。
// 复杂结构交给前端的 list/dict 编辑器序列化成标准 YAML，这里只读。
pub(crate) fn serde_yamlish(text: &str) -> Result<serde_json::Value, String> {
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

impl Business {
    /// Provider 配置变更后调用：全部插件标记待重载并立即同步，
    /// 重编的模块经 attach_plugin_env 拿到新的 providers 注入。
    pub fn refresh_providers(&self) {
        {
            let mut plugins = self.plugins.write().expect("业务锁中毒");
            for p in plugins.iter_mut() {
                p.mtime = std::time::SystemTime::UNIX_EPOCH;
            }
        }
        if let Err(e) = self.sync() {
            crate::log_error!("provider 变更后重载插件失败: {e}");
        }
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
            let plugins = self.plugins.read().expect("业务锁中毒");
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
            if let Ok(serde_json::Value::Object(map)) = serde_yamlish(&text) {
                values = map;
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
        if let Err(e) = std::fs::write(dir_path.join(super::DISABLE_MARKER), "") {
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

    /// WebUI 扩展 tab 聚合：已启用插件声明的 TABS（模块属性，SCHEMA 同款
    /// 约定）→ 前端导航项。ui.js 缺失或条目不合规（缺 id/title）直接丢弃——
    /// 导航指向打不开的页面没有意义。v = ui.js 的 mtime 毫秒，热更后变化，
    /// 前端用它击穿浏览器 module 缓存重新 import。
    /// 先扫一遍插件目录（同 handle 的惰性同步）：手改 plugin.py 声明、
    /// 增删插件目录，不刷新页面也能在下一次轮询里出现。
    pub fn ui_tabs(&self) -> Vec<serde_json::Value> {
        if let Err(e) = self.sync() {
            log_error!("插件目录扫描失败（tab 聚合沿用已加载插件）: {e}");
        }
        let plugins = self.plugins.read().expect("业务锁中毒");
        let mut tabs = Vec::new();
        for p in plugins.iter().filter(|p| p.enabled()) {
            let Some(v) = ui_js_version(&self.dir.join(&p.dir_name)) else {
                continue;
            };
            let Some(declared) = Python::attach(|py| {
                let attr = p.module.bind(py).getattr("TABS").ok()?;
                serde_json::from_str::<serde_json::Value>(&py_obj_to_json(&attr)).ok()
            }) else {
                continue;
            };
            let Some(entries) = declared.as_array() else { continue };
            for entry in entries {
                let (Some(id), Some(title)) = (
                    entry.get("id").and_then(|x| x.as_str()),
                    entry.get("title").and_then(|x| x.as_str()),
                ) else {
                    continue;
                };
                if !valid_ext_id(id) || title.is_empty() {
                    continue;
                }
                tabs.push(serde_json::json!({
                    "id": format!("{}-{}", p.dir_name, id),
                    "title": title,
                    "path": format!("/ext/{}/{}", p.dir_name, id),
                    "entry": format!("/ext/{}/{}", p.dir_name, UI_FILE),
                    "icon": entry.get("icon").and_then(|x| x.as_str()).unwrap_or("Link"),
                    "v": v,
                }));
            }
        }
        tabs
    }

    /// /ext/{name}/ui.js 的落盘路径。name 只允许字母数字_-，且目录必须是
    /// 真实插件（有 plugin.py）：两者共同挡住路径穿越。
    pub fn plugin_ui_js(&self, name: &str) -> Option<PathBuf> {
        if !valid_ext_id(name) {
            return None;
        }
        let dir = self.dir.join(name);
        if !dir.join(ENTRY_FILE).is_file() {
            return None;
        }
        let ui = dir.join(UI_FILE);
        ui.is_file().then_some(ui)
    }
}

// tab id / 插件名进 URL 与自定义元素名（loquat-page-<插件>-<tab>），收紧字符集
fn valid_ext_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// ui.js 的 mtime 毫秒（前端缓存指纹）；文件缺失返回 None
fn ui_js_version(plugin_dir: &Path) -> Option<u64> {
    let mtime = std::fs::metadata(plugin_dir.join(UI_FILE))
        .ok()?
        .modified()
        .ok()?;
    Some(
        mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    )
}
