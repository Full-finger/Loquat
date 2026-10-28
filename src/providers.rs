//! Provider 层：LLM / 搜索等外部服务的接入点与凭据，宿主级统一管理。
//!
//! 配置落 config/providers.yaml（运行时状态，不入库，模板 providers.yaml.example），
//! WebUI「服务」选项卡读写；解析后的激活项经 pybridge 注入插件全局
//! `providers`（{"llm": {...}, "search": {"provider", "api_key"}}），
//! 插件对「多套命名 provider 可切换」零感知。改动经 Business::refresh_providers
//! 热生效，无需重启。
//!
//! 首次运行迁移：providers.yaml 不存在而 chat/config.yaml 还存着旧版
//! 内联凭据时，把 llm/search 字段整体上移，chat 配置瘦身重写——
//! api_key 从插件目录（易被 git add -f / 截图误带）收编到 config/ 一处。

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, PoisonError, RwLock};

use crate::business::serde_yamlish;
use crate::{log_error, log_info, log_warn};

const PROVIDERS_FILE: &str = "config/providers.yaml";
const CHAT_CONFIG: &str = "extensions/chat/config.yaml";

/// 毒锁恢复（26102701#8）：providers 锁中毒若 expect，panic 会顺着
/// sync 的 plugins 写锁级联（attach_plugin_env → resolved_json），
/// 把业务锁一起毒掉。恢复继续 + warn 留痕。
fn recover_poison<T>(poisoned: PoisonError<T>) -> T {
    log_warn!("providers 锁中毒，已恢复继续（曾有 panic 持锁）");
    poisoned.into_inner()
}

// llm provider 的字段与缺省（与 chat 插件迁移前的 SCHEMA 对齐）
fn llm_field_defaults() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("api_base", serde_json::json!("https://api.deepseek.com/v1")),
        ("api_key", serde_json::json!("")),
        ("model", serde_json::json!("deepseek-chat")),
        ("temperature", serde_json::json!(0.8)),
        ("max_tokens", serde_json::json!(1024)),
        ("timeout_seconds", serde_json::json!(60)),
        ("vision", serde_json::json!(false)),
    ]
}

fn llm_defaults() -> serde_json::Value {
    let mut m = serde_json::Map::new();
    for (k, v) in llm_field_defaults() {
        m.insert(k.to_string(), v);
    }
    serde_json::Value::Object(m)
}

static PROVIDERS: OnceLock<RwLock<serde_json::Value>> = OnceLock::new();

fn cell() -> &'static RwLock<serde_json::Value> {
    PROVIDERS.get_or_init(|| RwLock::new(defaults()))
}

// 无配置文件时的初始形态：deepseek 单套 + 三搜索通道占位
fn defaults() -> serde_json::Value {
    serde_json::json!({
        "llm": {
            "active": "deepseek",
            "providers": { "deepseek": llm_defaults() },
        },
        "search": {
            "active": "deepseek",
            "providers": {
                "deepseek": { "api_key": "" },
                "duckduckgo": { "api_key": "" },
                "bocha": { "api_key": "" },
            },
        },
    })
}

/// 启动时加载：读 providers.yaml；缺失则尝试从 chat 旧配置迁移。
/// 失败只记日志用缺省值（宁可带病运行，不挡启动）。
pub fn load() {
    load_from(Path::new(PROVIDERS_FILE), Path::new(CHAT_CONFIG));
}

fn load_from(path: &Path, chat_cfg: &Path) {
    let loaded = match std::fs::read_to_string(path) {
        Ok(text) => serde_yamlish(&text).ok().filter(|v| v.is_object()),
        Err(_) => None,
    };
    match loaded {
        Some(v) => {
            if let Ok(norm) = normalize(&v) {
                *cell().write().unwrap_or_else(recover_poison) = norm;
            } else {
                log_error!("providers.yaml 形态不合法，用缺省值（可在 WebUI 重新保存）");
            }
        }
        None => {
            let migrated = migrate(chat_cfg);
            *cell().write().unwrap_or_else(recover_poison) = migrated.clone();
            if let Err(e) = std::fs::write(path, render(&migrated)) {
                log_error!("写 providers.yaml 失败（本次运行用内存配置）: {e}");
            } else {
                log_info!("已生成 {}（模板 providers.yaml.example）", path.display());
            }
        }
    }
}

// 首运迁移：chat/config.yaml 还以内联键存 LLM/搜索凭据时整体上移，
// 并把 chat 配置瘦身重写（原子替换，写坏不伤原文件）。
fn migrate(chat_cfg: &Path) -> serde_json::Value {
    let mut out = defaults();
    let Ok(text) = std::fs::read_to_string(chat_cfg) else {
        return out;
    };
    let Ok(old) = serde_yamlish(&text) else {
        log_error!("chat/config.yaml 解析失败，跳过 provider 迁移");
        return out;
    };

    // llm：任一接入字段非空即视为配置过，整体收编到 "deepseek" 命名项
    let has_llm = ["api_base", "api_key", "model"]
        .iter()
        .any(|&k| old.get(k).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()));
    if has_llm {
        let mut p = llm_defaults();
        if let serde_json::Value::Object(m) = &mut p {
            for (k, _) in llm_field_defaults() {
                if let Some(v) = old.get(k) {
                    if !v.is_null() {
                        m.insert(k.to_string(), v.clone());
                    }
                }
            }
        }
        out["llm"]["providers"]["deepseek"] = p;
    }
    // search：激活项沿旧 search_provider；key 只对 bocha 类有意义，原样带上
    let sp = old.get("search_provider").and_then(|v| v.as_str()).unwrap_or("deepseek");
    if matches!(sp, "deepseek" | "duckduckgo" | "bocha") {
        out["search"]["active"] = serde_json::json!(sp);
    }
    if let Some(skey) = old.get("search_api_key").and_then(|v| v.as_str()) {
        if !skey.is_empty() {
            out["search"]["providers"][sp]["api_key"] = serde_json::json!(skey);
        }
    }

    // chat 配置瘦身：保留宿主保留键与剩余 SCHEMA 字段，凭据不再驻留插件目录
    let mut slim = String::from(
        "# chat 插件配置（provider 迁移时自动瘦身；LLM/搜索凭据已上移 config/providers.yaml）\nenabled: true\n",
    );
    for k in ["triggers", "system_prompt", "history_limit", "max_turns"] {
        if let Some(v) = old.get(k) {
            slim.push_str(&format!("{k}: {}\n", json_scalar(v)));
        }
    }
    let tmp: PathBuf = chat_cfg.with_extension("yaml.tmp");
    if std::fs::write(&tmp, slim)
        .and_then(|_| std::fs::rename(&tmp, chat_cfg))
        .is_err()
    {
        log_error!("chat/config.yaml 瘦身失败（保留原文件，凭据仍在插件目录）");
    } else {
        log_info!("chat/config.yaml 已瘦身：LLM/搜索凭据上移 config/providers.yaml");
    }
    out
}

// 标量按 YAML 兼容形态输出（JSON 字符串/数组文本是合法的 YAML 流式标量）
fn json_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into())
        }
        other => other.to_string(),
    }
}

// providers.yaml 文本渲染（结构固定两层，手写 emitter；注释约定见模板）
fn render(v: &serde_json::Value) -> String {
    let mut s = String::from(
        "# 外部服务接入点与凭据（运行时状态，不入库；模板 providers.yaml.example）\n\
         # llm/search 各维护若干命名 provider，active 指向生效项；\n\
         # WebUI「服务」选项卡保存即热生效，无需重启。\n",
    );
    for (section, keys) in [
        ("llm", None::<Vec<&str>>),
        ("search", Some(vec!["api_key"])),
    ] {
        s.push_str(&format!("{section}:\n"));
        let active = v[section]["active"].as_str().unwrap_or("");
        s.push_str(&format!("  active: {}\n", json_scalar(&serde_json::json!(active))));
        s.push_str("  providers:\n");
        if let Some(map) = v[section]["providers"].as_object() {
            for (name, p) in map {
                s.push_str(&format!("    {}:\n", json_scalar(&serde_json::json!(name))));
                let fields: Vec<(&str, serde_json::Value)> = match &keys {
                    None => llm_field_defaults(),
                    Some(ks) => ks.iter().map(|k| (*k, serde_json::json!(""))).collect(),
                };
                for (k, d) in fields {
                    let val = p.get(k).unwrap_or(&d);
                    s.push_str(&format!("      {k}: {}\n", json_scalar(val)));
                }
            }
        }
    }
    s
}

// 表单/外部输入 → 内存形态：只认已知字段，缺省回填，active 必须存在
fn normalize(body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut out = defaults();
    for (section, is_llm) in [("llm", true), ("search", false)] {
        let sec = body.get(section).ok_or_else(|| format!("缺少 {section} 段"))?;
        let map = sec
            .get("providers")
            .and_then(|p| p.as_object())
            .ok_or_else(|| format!("{section}.providers 缺失或不是对象"))?;
        if map.is_empty() {
            return Err(format!("{section} 至少保留一个 provider"));
        }
        let active = sec
            .get("active")
            .and_then(|a| a.as_str())
            .ok_or_else(|| format!("{section}.active 缺失"))?;
        if !map.contains_key(active) {
            return Err(format!("{section}.active 指向不存在的 provider {active:?}"));
        }
        out[section]["active"] = serde_json::json!(active);
        if let Some(out_map) = out[section]["providers"].as_object_mut() {
            out_map.clear();
        }
        for (name, p) in map {
            if name.is_empty() {
                return Err(format!("{section} 有空的 provider 名"));
            }
            let mut entry = if is_llm { llm_defaults() } else { serde_json::json!({"api_key": ""}) };
            if let Some(em) = entry.as_object_mut() {
                for k in em.keys().cloned().collect::<Vec<_>>() {
                    if let Some(v) = p.get(&k) {
                        if !v.is_null() {
                            em.insert(k, v.clone());
                        }
                    }
                }
            }
            out[section]["providers"][name.as_str()] = entry;
        }
    }
    Ok(out)
}

// 保存前的准备：api_key 传 null 清除、缺省/空串保持已有值（打码回传的
// 配套约定），再走 normalize 校验。
fn prepare(body: &serde_json::Value, old: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut body = body.clone();
    for section in ["llm", "search"] {
        let Some(map) = body[section]["providers"].as_object_mut() else {
            continue;
        };
        for (name, p) in map.iter_mut() {
            let Some(pm) = p.as_object_mut() else { continue };
            match pm.get("api_key") {
                Some(serde_json::Value::Null) => {
                    pm.insert("api_key".into(), serde_json::json!(""));
                }
                Some(serde_json::Value::String(s)) if s.is_empty() => {
                    let had = old[section]["providers"][name.as_str()]["api_key"].clone();
                    pm.insert("api_key".into(), had);
                }
                _ => {}
            }
            pm.remove("has_api_key");
        }
    }
    normalize(&body)
}

/// 保存：校验、写盘、更新内存。调用方应随后触发
/// Business::refresh_providers 让插件重注入。
pub fn save(body: &serde_json::Value) -> Result<(), String> {
    let norm = prepare(body, &cell().read().unwrap_or_else(recover_poison))?;
    std::fs::write(PROVIDERS_FILE, render(&norm))
        .map_err(|e| format!("写 providers.yaml 失败: {e}"))?;
    *cell().write().unwrap_or_else(recover_poison) = norm;
    Ok(())
}

/// 注入插件全局的解析结果：激活项拍平，多 provider 机制对插件透明。
/// 未加载（测试等场景）为缺省形态。
pub fn resolved_json() -> String {
    let v = cell().read().unwrap_or_else(recover_poison);
    resolved_of(&v)
}

fn resolved_of(v: &serde_json::Value) -> String {
    let llm = &v["llm"]["providers"][v["llm"]["active"].as_str().unwrap_or("")];
    let sp = v["search"]["active"].as_str().unwrap_or("");
    let search = &v["search"]["providers"][sp];
    serde_json::json!({
        "llm": llm,
        "search": { "provider": sp, "api_key": search["api_key"] },
    })
    .to_string()
}

/// WebUI 视图：api_key 不回传明文，用 has_api_key 标记是否已配置。
pub fn view() -> serde_json::Value {
    view_of(cell().read().unwrap_or_else(recover_poison).clone())
}

fn view_of(v: serde_json::Value) -> serde_json::Value {
    let mut out = v;
    for section in ["llm", "search"] {
        if let Some(map) = out[section]["providers"].as_object_mut() {
            for p in map.values_mut() {
                let has = p.get("api_key").and_then(|k| k.as_str()).is_some_and(|k| !k.is_empty());
                if let Some(pm) = p.as_object_mut() {
                    pm.insert("has_api_key".into(), serde_json::json!(has));
                    pm.insert("api_key".into(), serde_json::json!(""));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
