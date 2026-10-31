//! 进程与硬件探测：宿主/适配器子进程的 CPU、内存，硬件静态信息。
//! 供仪表盘 /api/status 使用。sysinfo 的 CPU% 需要两次 refresh 之间
//! 隔开（>=200ms）才有意义，WebUI 10s 一拍，首拍为 0 属预期。

use std::sync::{Mutex, OnceLock};

use serde_json::json;
use sysinfo::{Pid, ProcessesToUpdate, System};

static SYS: OnceLock<Mutex<System>> = OnceLock::new();

fn sys() -> &'static Mutex<System> {
    SYS.get_or_init(|| Mutex::new(System::new()))
}

/// GPU 控制器名称，探测一次永久缓存（型号不会变；失败不重试，
/// 避免每拍 status 都拖 WMI 的慢路径）。非 Windows 返回空，前端隐藏该行。
#[cfg(windows)]
pub fn gpus() -> Vec<String> {
    static GPUS: OnceLock<Vec<String>> = OnceLock::new();
    GPUS.get_or_init(detect_gpus_windows).clone()
}

#[cfg(not(windows))]
pub fn gpus() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
fn detect_gpus_windows() -> Vec<String> {
    use std::collections::HashMap;

    use wmi::{COMLibrary, Variant, WMIConnection};

    (|| {
        let con = WMIConnection::new(COMLibrary::new().ok()?).ok()?;
        let rows: Vec<HashMap<String, Variant>> = con
            .raw_query("SELECT Name FROM Win32_VideoController")
            .ok()?;
        Some(
            rows.iter()
                .filter_map(|r| r.get("Name"))
                .filter_map(|v| match v {
                    Variant::String(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        )
    })()
    .unwrap_or_default()
}

/// 按进程号探测 CPU% 与内存（字节）。进程已退出返回 None。
pub fn probe(pid: u32) -> Option<(f32, u64)> {
    let mut sys = sys().lock().ok()?;
    probe_locked(&mut sys, pid)
}

/// 已持锁版：供 host() 复用，避免对非重入 Mutex 二次加锁死锁。
fn probe_locked(sys: &mut System, pid: u32) -> Option<(f32, u64)> {
    let p = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    let proc = sys.process(p)?;
    Some((proc.cpu_usage(), proc.memory()))
}

/// 宿主进程占用 + 硬件静态信息（CPU 型号/频带/总内存/GPU）。
pub fn host() -> serde_json::Value {
    let Ok(mut sys) = sys().lock() else {
        return json!({});
    };
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    let (host_cpu, host_mem) = probe_locked(&mut sys, std::process::id()).unwrap_or((0.0, 0));
    json!({
        "os_long": System::long_os_version(),
        "arch": std::env::consts::ARCH,
        "cpu_brand": sys.cpus().first().map(|c| c.brand().trim()).unwrap_or(""),
        "cpu_cores": sys.cpus().len(),
        "cpu_freq": sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
        "gpus": gpus(),
        "mem_total": sys.total_memory(),
        "host_cpu": host_cpu,
        "host_mem": host_mem,
        // 全局占用：仪表盘圆环上与宿主进程占用对照展示
        "sys_cpu": sys.global_cpu_usage(),
        "sys_mem_used": sys.total_memory().saturating_sub(sys.available_memory()),
    })
}

#[cfg(test)]
mod tests {
    use super::{host, probe};

    /// 死锁回归：host() 持锁时探测宿主自身（旧实现在此二次加锁卡死）。
    /// 卡死的测试会被 cargo test 超时杀掉，直接暴露。
    #[test]
    fn host_does_not_deadlock() {
        let v = host();
        assert!(v.get("cpu_cores").and_then(|c| c.as_u64()).unwrap_or(0) > 0);
        assert!(v.get("mem_total").and_then(|m| m.as_u64()).unwrap_or(0) > 0);
        // 顺手验证 probe 自身路径（含查不到的 pid）
        assert!(probe(u32::MAX).is_none());
    }
}
