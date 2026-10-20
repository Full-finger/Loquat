//! 宿主日志：格式对齐 hclog 文本输出（Go 核心侧的基准格式）。
//! 日志全部走 stderr，`2> run.log` 一把抓；日志源名用项目名 loquat。
//! 核心的 hclog 输出经管道由 pump_from_handle 接管，三边收敛到
//! 宿主的 stderr 锁下单一写者，避免多运行时并发写控制台绞行。

use std::fmt::Arguments;
use std::fs::File;
use std::io::{BufRead, Write};
use std::mem::ManuallyDrop;
use std::os::windows::io::{FromRawHandle, RawHandle};

/// hclog 把级别字段对齐到 8 列：[INFO] 后 2 空格，[ERROR] 后 1 空格。
pub fn emit(level: &str, args: Arguments<'_>) {
    let pad = " ".repeat(6 - level.len());
    eprintln!(
        "{} [{level}]{pad}loquat: {args}",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z")
    );
}

/// 从核心日志管道读出已格式化的 hclog 行，在宿主的 stderr 锁下打印。
/// 句柄归 Go 侧持有，这里 ManuallyDrop 只读不关；线程随进程存活。
pub fn pump_from_handle(raw: usize) {
    if raw == 0 {
        return;
    }
    let file = ManuallyDrop::new(unsafe {
        File::from_raw_handle(raw as RawHandle)
    });
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(&*file);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let stderr = std::io::stderr();
                    let _ = stderr.lock().write_all(line.as_bytes());
                }
            }
        }
    });
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::log::emit("INFO", format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log::emit("WARN", format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log::emit("ERROR", format_args!($($arg)*)) };
}
