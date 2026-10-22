//! 控制台命令：REPL 应答（status / agents / plugins / enable / disable /
//! install / reload napcat / exit）。数据来自 stats 与 pybridge / ffi。

use crate::business::Business;
use crate::ffi::CoreDll;
use crate::{pybridge, stats};

// 控制台命令的应答是 REPL 输出，不是日志：裸文本走 stdout，不带时间戳。
// 异步事件（消息/回复）与运行日志才走 stderr 的统一日志格式。
pub fn run_command(cmd: &str, dll: &CoreDll, business: &Business) {
    match cmd {
        "" => {}
        "status" => println!(
            "核心: {} | 事件 {} 条 | 已回复 {} 条 | 业务: {}",
            dll.status(),
            stats::event_count(),
            stats::reply_count(),
            business.info()
        ),
        "agents" => {
            println!(
                "worker 积压: {:?} | 累计处理: {:?}",
                stats::worker_backlogs(),
                stats::worker_done()
            );
            let active = stats::active_chats();
            if active.is_empty() {
                println!("活跃会话: 无");
            } else {
                println!("活跃会话: {active:?}");
            }
            match pybridge::agent_engine_stats() {
                Some(s) => println!(
                    "agent 引擎: 有历史会话 {} | respond {} 次 | LLM 轮次 {}",
                    s.sessions, s.responds, s.turns
                ),
                None => println!("agent 引擎: 未初始化"),
            }
            // 核心侧队列丢弃计数（适配器侧在子进程内，只有日志）
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&dll.status()) {
                if let Some(d) = v.get("events_dropped").and_then(|d| d.as_u64()) {
                    println!("核心丢弃事件: {d}");
                }
            }
        }
        "reload napcat" => {
            let rc = dll.reload_plugin("napcat");
            println!(
                "reload napcat -> {}",
                if rc == 0 {
                    "已触发，核心将退避重启适配器"
                } else {
                    "失败（核心未启动）"
                }
            );
        }
        "plugins" => println!("{}", business.info()),
        c if c.starts_with("disable ") => {
            let name = c["disable ".len()..].trim();
            let found = business.set_enabled(name, false);
            println!(
                "{name} -> {}",
                if found { "已禁用（重启后回到文件内 ENABLED 声明）" } else { "未找到该插件" }
            );
        }
        c if c.starts_with("install ") => {
            let name = c["install ".len()..].trim();
            match business.install(name) {
                Ok(msg) => println!("{msg}"),
                Err(e) => println!("{e}"),
            }
        }
        c if c.starts_with("enable ") => {
            let name = c["enable ".len()..].trim();
            let found = business.set_enabled(name, true);
            println!(
                "{name} -> {}",
                if found { "已启用" } else { "未找到该插件" }
            );
        }
        "help" => println!(
            "命令: status | agents | plugins | install <插件名> | enable/disable <插件名> | reload napcat | exit"
        ),
        "exit" | "quit" => {
            dll.stop();
            println!("已退出");
            std::process::exit(0);
        }
        other => println!("未知命令: {other}（help 查看命令）"),
    }
}

// 日志预览用：图片码等长内容只留开头，避免刷屏。
pub fn brief(s: &str, max_chars: usize) -> String {
    if s.starts_with("[CQ:image") {
        return "[图片]".to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push('…');
    }
    out
}
