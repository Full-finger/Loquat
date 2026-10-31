//! 插件资源（asset 配置字段的文件面）：上传、落盘路径解析、删除。
//! 从 config_api.rs 迁出：R4 规则限 400 行。
//!
//! 上传落 extensions/<名>/assets/，配置里存相对路径 assets/<文件名>。
//! 与插件 JS 同信任域（插件本在进程内跑任意代码），不做隔离，只挡
//! 路径穿越与超大文件。

use std::path::PathBuf;

use super::config_api::valid_ext_id;
use super::Business;

const ASSET_DIR: &str = "assets";
const ASSET_MAX_BYTES: usize = 10 * 1024 * 1024;

impl Business {
    /// 资源清单（assets/ 下的文件名）；目录不存在视为空。
    pub fn list_plugin_assets(&self, name: &str) -> Result<Vec<String>, String> {
        if !valid_ext_id(name) {
            return Err(format!("非法插件名: {name}"));
        }
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.dir.join(name).join(ASSET_DIR)) {
            for e in rd.flatten() {
                if e.path().is_file() {
                    if let Some(n) = e.file_name().to_str() {
                        out.push(n.to_string());
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }

    /// 保存上传：重名不覆盖（追加 -1/-2），返回落盘文件名。
    pub fn save_plugin_asset(
        &self,
        name: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, String> {
        if !valid_ext_id(name) {
            return Err(format!("非法插件名: {name}"));
        }
        if !valid_asset_name(filename) {
            return Err(format!("非法文件名: {filename}"));
        }
        if bytes.is_empty() {
            return Err("空文件".into());
        }
        if bytes.len() > ASSET_MAX_BYTES {
            return Err("文件超过 10MB 上限".into());
        }
        let assets = self.dir.join(name).join(ASSET_DIR);
        std::fs::create_dir_all(&assets).map_err(|e| format!("建 assets 目录失败: {e}"))?;
        let (stem, ext) = match filename.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
            _ => (filename.to_string(), String::new()),
        };
        let mut stored = filename.to_string();
        let mut n = 1;
        while assets.join(&stored).exists() {
            stored = format!("{stem}-{n}{ext}");
            n += 1;
        }
        std::fs::write(assets.join(&stored), bytes).map_err(|e| format!("写资源失败: {e}"))?;
        Ok(stored)
    }

    /// 配置值（assets/<文件名>）→ 落盘路径；穿越/不存在返回 None。
    pub fn plugin_asset_path(&self, name: &str, rel: &str) -> Option<PathBuf> {
        let file = rel.strip_prefix("assets/")?;
        if !valid_ext_id(name) || !valid_asset_name(file) {
            return None;
        }
        let path = self.dir.join(name).join(ASSET_DIR).join(file);
        path.is_file().then_some(path)
    }

    pub fn delete_plugin_asset(&self, name: &str, rel: &str) -> Result<(), String> {
        let path = self
            .plugin_asset_path(name, rel)
            .ok_or_else(|| "资源不存在或路径非法".to_string())?;
        std::fs::remove_file(path).map_err(|e| format!("删除失败: {e}"))
    }
}

// 资源文件名收紧：字母数字/-/_/. 与非 ASCII（中文文件名），拒隐藏
// 文件与连续点。只用于 assets/ 下的单段名字，不含分隔符即无穿越。
fn valid_asset_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && !s.starts_with('.')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || !c.is_ascii())
}
