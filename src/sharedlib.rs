//! 共享插件库与配置模板生成。
//!
//! extensions/_lib/ 下的库由宿主编译并以稳定名注册进 sys.modules
//! （config.py -> loquat_cfglib，log.py -> loquat_log），
//! 插件 `import loquat_log` 即用。不走普通 sys.path：
//! 匿名模块加载没有导入上下文，共享缓存还会让热重载吃到旧模块——
//! 稳定名 + 宿主检测 mtime 替换条目，才保得住热重载语义。
//! 插件声明 SCHEMA 且 config.yaml 缺失时，由这里调库生成带注释模板。

use std::error::Error;
use std::ffi::CString;
use std::path::Path;
use std::time::UNIX_EPOCH;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

/// 库清单：(相对路径, 注册名)。loquat_log 必须在 loquat_cfglib 之前：
/// config.py 顶层 `import loquat_log`，靠 sys.modules 里已有条目解析。
const LIBS: [(&str, &str); 2] = [
    ("_lib/log.py", "loquat_log"),
    ("_lib/config.py", "loquat_cfglib"),
];
pub const LIB_MODULE: &str = "loquat_cfglib";

/// 库文件的 mtime（毫秒）取各库的较新者；缺失为 0。宿主用它检测变更并重载。
pub fn lib_mtime_millis(ext_dir: &Path) -> u64 {
    LIBS.iter()
        .map(|(rel, _)| {
            std::fs::metadata(ext_dir.join(rel))
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0)
}

/// 编译各库并按清单顺序注册/替换 sys.modules 条目。
/// 一个库都没找到返回 Ok(false)；存在但编译失败返回 Err（调用方告警）。
pub fn load(ext_dir: &Path) -> Result<bool, Box<dyn Error>> {
    let mut loaded = false;
    for (rel, name) in LIBS {
        let path = ext_dir.join(rel);
        if !path.is_file() {
            continue;
        }
        loaded = true;
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("读共享库失败 {path:?}: {e}"))?;
        Python::attach(|py| -> Result<(), Box<dyn Error>> {
            let file = CString::new(path.to_string_lossy().as_bytes())?;
            let cname = CString::new(name)?;
            let code = CString::new(source)?;
            let module = PyModule::from_code(py, &code, &file, &cname)?;
            let modules = py.import("sys")?.getattr("modules")?;
            modules.set_item(name, &module)?;
            Ok(())
        })?;
    }
    Ok(loaded)
}

/// 按插件的 SCHEMA 生成 config.yaml，install 命令调用。
/// 已存在则不动，返回是否写了文件。
pub fn ensure_config_file(
    plugin_dir: &Path,
    module: &Bound<'_, PyModule>,
) -> Result<bool, Box<dyn Error>> {
    let Ok(schema) = module.getattr("SCHEMA") else {
        return Ok(false); // 没声明 SCHEMA 的插件不管
    };
    if plugin_dir.join("config.yaml").is_file() {
        return Ok(false);
    }
    Python::attach(|py| {
        let modules = py.import("sys")?.getattr("modules")?;
        let dict = modules.downcast_into::<PyDict>().map_err(PyErr::from)?;
        let lib = dict
            .get_item(LIB_MODULE)?
            .ok_or("共享库 loquat_cfglib 未加载，无法生成配置模板")?;
        let raw = module
            .getattr("config")
            .unwrap_or_else(|_| PyDict::new(py).into_any());
        let written: bool = lib
            .call_method1("ensure_file", (schema, plugin_dir.to_string_lossy(), raw))?
            .extract()?;
        Ok(written)
    })
}
