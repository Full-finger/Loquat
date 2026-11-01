//! 单个插件的装载：from_code 编译、环境注入、import 上下文管理。
//! 与 Business 的目录扫描/路由解耦，专注"把一个 plugin.py 变成已注入的模块"。
//!
//! import 上下文：插件目录临时挂上 sys.path（挂末尾——本地文件不遮蔽
//! 标准库，但同名已装包会抢先，本地模块起名别太通用），加载完摘掉，
//! 并弹掉本次 import 引入的本地模块——防跨插件串台与热重载吃缓存。

use std::error::Error;
use std::ffi::CString;
use std::path::Path;
use std::time::SystemTime;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

use crate::business::CONFIG_FILE;
use crate::pybridge;

const DEFAULT_PRIORITY: i64 = 100;

/// plugin.py 首行签名：宿主识别插件的依据。
pub(crate) const SIGNATURE: &str = "# loquat-plugin";

/// 宿主支持的插件 API 版本（评审建议 5）：
///   v1  handle 返回 list[str]，每个字符串一条消息（存量）
///   v2  handle 可返回 str / list[段] / None，段为 protojson 形态与入站同构
pub(crate) const SUPPORTED_API: [u8; 2] = [1, 2];

/// 文件修改时间；失败（不存在等）由调用方处理。
pub(crate) fn file_mtime(path: &Path) -> Result<SystemTime, std::io::Error> {
    std::fs::metadata(path)?.modified()
}

/// 签名检查只读文件头几十字节，不整文件读。
pub(crate) fn has_signature(entry_py: &Path) -> bool {
    let mut buf = [0u8; 64];
    match std::fs::File::open(entry_py).and_then(|mut f| std::io::Read::read(&mut f, &mut buf)) {
        Ok(n) => String::from_utf8_lossy(&buf[..n])
            .trim_start()
            .starts_with(SIGNATURE),
        Err(_) => false,
    }
}

/// load_module 的产物：模块、优先级、config.enabled 声明、是否可安装
/// （声明了 SCHEMA 且 config.yaml 缺失；此时模块不应生效，等 install 命令）、
/// 插件 API 版本（决定 handle 返回值怎么解释）。
pub struct Loaded {
    pub module: Py<PyModule>,
    pub priority: i64,
    pub config_enabled: bool,
    pub installable: bool,
    pub api_version: u8,
}

/// 签名行的 API 版本：`# loquat-plugin` = v1（存量），`# loquat-plugin v<N>` 显式。
/// 不支持的版本明确拒载（进失败退避），不静默尝试（评审建议 9）。
/// 首行不是签名也按 v1：已加载插件不因事后抹掉签名而凭空卸载。
pub(crate) fn parse_api_version(first_line: &str) -> Result<u8, String> {
    let Some(rest) = first_line.trim_start().strip_prefix(SIGNATURE) else {
        return Ok(1);
    };
    let ver = rest.trim();
    if ver.is_empty() {
        return Ok(1);
    }
    let supported = || {
        SUPPORTED_API
            .iter()
            .map(|v| format!("v{v}"))
            .collect::<Vec<_>>()
            .join("/")
    };
    let n = ver
        .strip_prefix('v')
        .and_then(|s| s.parse::<u8>().ok())
        .ok_or_else(|| format!("插件 API 版本无法识别: {ver}（宿主支持 {}）", supported()))?;
    if !SUPPORTED_API.contains(&n) {
        return Err(format!(
            "插件 API 版本不支持: {ver}（宿主支持 {}）",
            supported()
        ));
    }
    Ok(n)
}

/// 加载插件入口，注入 config / core 环境。返回模块、优先级、配置内
/// enabled 声明、是否可安装与 API 版本。
pub fn load_module(path: &Path) -> Result<Loaded, Box<dyn Error>> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("读插件失败 {path:?}: {e}"))?;
    let api_version = parse_api_version(source.lines().next().unwrap_or(""))?;
    let config_path = path.parent().unwrap_or(Path::new(".")).join(CONFIG_FILE);
    // 没有配置文件 = 空配置，不算错误
    let config_yaml = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
        Err(e) => return Err(format!("读配置失败 {config_path:?}: {e}").into()),
    };

    Python::attach(|py| {
        let dir = path.parent().unwrap_or(Path::new("."));
        let file = CString::new(path.to_string_lossy().as_bytes())?;
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("plugin");
        let name = pybridge::next_module_name(&format!("ext_{dir_name}"))?;
        let code = CString::new(source)?;

        let sys_mod = py.import("sys")?;
        let sys_path = sys_mod.getattr("path")?;
        let modules = sys_mod.getattr("modules")?;

        let dir_str = dir.to_string_lossy().into_owned();
        let _ = sys_path.call_method1("append", (&dir_str,));
        let before = module_keys(&modules);

        let built = (|| -> Result<Bound<'_, PyModule>, Box<dyn Error>> {
            let module = PyModule::from_code(py, &code, &file, &name)?;
            pybridge::attach_plugin_env(&module, &config_yaml)?;
            Ok(module)
        })();

        // 无论成败都先摘掉 sys.path，再清理 import 进来的本地模块。
        // 共享库是宿主注册的稳定名，不在插件目录下，不受影响。
        let _ = sys_path.call_method1("remove", (&dir_str,));
        pop_local_modules(py, &modules, &before, dir);

        let module = built?;
        // from_code 会把模块注册进 sys.modules；我们用唯一名防串台，
        // 代价是每次加载/热重载都留一个条目。用完即弃，防长期缓涨。
        let key = name.to_string_lossy().into_owned();
        let _ = modules.call_method1("pop", (key, py.None()));

        // v2 起模块可声明 API = <N>，须与签名一致（提案 §6）；缺省以签名为准。
        // 不一致硬拒：AI 生成插件的场景下静默降级最难排查。
        if api_version >= 2 {
            if let Ok(declared) = module.getattr("API").and_then(|v| v.extract::<u8>()) {
                if declared != api_version {
                    return Err(
                        format!("插件签名 v{api_version} 但声明 API = {declared}，拒载").into(),
                    );
                }
            }
        }

        // 可安装 = 声明了 SCHEMA 且 config.yaml 缺失。宿主不自动生成模板，
        // 由控制台 install 命令显式安装（生成 config.yaml + .disabled 标记）
        let installable = module.getattr("SCHEMA").is_ok() && !dir.join(CONFIG_FILE).is_file();

        let priority = module
            .getattr("PRIORITY")
            .and_then(|v| v.extract::<i64>())
            .unwrap_or(DEFAULT_PRIORITY);
        let config_enabled = module
            .getattr("config")
            .and_then(|c| c.get_item("enabled"))
            .and_then(|v| v.extract::<bool>())
            .unwrap_or(true);
        Ok(Loaded {
            module: module.into(),
            priority,
            config_enabled,
            installable,
            api_version,
        })
    })
}

/// sys.modules 的字符串键快照。
fn module_keys(modules: &Bound<'_, PyAny>) -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(iter) = modules.try_iter() {
        for key in iter.flatten() {
            if let Ok(k) = key.extract::<String>() {
                keys.push(k);
            }
        }
    }
    keys
}

/// 弹掉本次加载引入的本地模块：__file__ 落在插件目录下的才算本地。
/// 标准库/第三方包保留缓存，避免同类对象跨副本的身份分裂。
fn pop_local_modules(py: Python<'_>, modules: &Bound<'_, PyAny>, before: &[String], dir: &Path) {
    let dir_norm = dir.to_string_lossy().replace('\\', "/");
    let Ok(iter) = modules.try_iter() else {
        return;
    };
    let mut new_keys: Vec<String> = Vec::new();
    for key in iter.flatten() {
        if let Ok(k) = key.extract::<String>() {
            if !before.contains(&k) {
                new_keys.push(k);
            }
        }
    }
    for k in new_keys {
        let Ok(m) = modules.get_item(&k) else {
            continue;
        };
        let file: Option<String> = m
            .getattr("__file__")
            .ok()
            .and_then(|f| f.extract::<String>().ok());
        let in_dir = file.is_some_and(|f| f.replace('\\', "/").starts_with(&dir_norm));
        if in_dir {
            let _ = modules.call_method1("pop", (k, py.None()));
        }
    }
}
