// loquat 宿主（Rust）：进程入口。
// 加载 Go 核心 dll（配置中心 / 插件监督 / 消息队列都在里面），
// 收事件，调 Python 业务（pyo3 嵌入，脚本改动自动热重载），发动作。

mod business;
mod ffi;
mod log;
mod pluginload;
mod pybridge;
mod sharedlib;

use std::error::Error;
use std::ffi::c_char;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, OnceLock};

use business::Business;
use ffi::CoreDll;

const EXTENSIONS_DIR: &str = "extensions";

static EVENT_TX: OnceLock<mpsc::Sender<HostMsg>> = OnceLock::new();
static EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static REPLY_COUNT: AtomicU64 = AtomicU64::new(0);

// 宿主主循环的两类输入：Go 核心的事件、控制台命令。
enum HostMsg {
    Event(Vec<u8>),
    Command(String),
}

// Go 核心的事件回调：只拷贝字节进通道就返回，别在回调里干重活。
// 禁止 unwrap/expect——panic 跨 C 边界会直接炸宿主进程（见 R3 规则）。
extern "C" fn on_event(data: *const c_char, len: i32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) };
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(HostMsg::Event(bytes.to_vec()));
    }
}

// 从事件 JSON 里取出 MVP 关心的字段。
struct Incoming {
    message_type: String,
    chat_id: i64,
    user_id: i64,
    self_id: i64,
    text: String,
}

fn parse_incoming(payload: &[u8]) -> Option<Incoming> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    if v.get("post_type")?.as_str()? != "message" {
        return None;
    }
    let message_type = v.get("message_type")?.as_str()?.to_string();
    let self_id = v.get("self_id").and_then(|s| s.as_i64()).unwrap_or(0);
    let user_id = match message_type.as_str() {
        "private" => v.get("user_id")?.as_i64()?,
        "group" => v
            .get("sender")
            .and_then(|s| s.get("user_id"))
            .and_then(|u| u.as_i64())?,
        _ => return None,
    };
    if user_id == self_id {
        return None; // 机器人自己的消息，丢弃防自问自答
    }
    let chat_id = match message_type.as_str() {
        "private" => user_id,
        "group" => v.get("group_id")?.as_i64()?,
        _ => return None,
    };
    Some(Incoming {
        message_type,
        chat_id,
        user_id,
        self_id,
        text: v.get("raw_message")?.as_str()?.to_string(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let dll = Arc::new(unsafe { CoreDll::load("bin/core.dll")? });
    ffi::set_core_dll(dll.clone());
    // 接管核心日志泵：此后三边日志都在宿主 stderr 锁下单一写者打印。
    // 这是首次 FFI 调用，顺带完成 Go 运行时初始化（须在 stdin 线程前）。
    log::pump_from_handle(dll.log_pipe());
    log_info!("core.dll 已加载");

    let (tx, rx) = mpsc::channel::<HostMsg>();
    EVENT_TX.set(tx.clone()).expect("事件通道只能初始化一次");

    // 先起 Go 核心，再初始化 Python 与 stdin 线程：
    // 1) CPython 先于 Go 运行时初始化会在 Windows 上互相卡死；
    // 2) 阻塞读 stdin 的线程若先于首次 FFI 启动，Go 运行时初始化会直接冻死
    //   （实测：stdin 给 DEVNULL 则一切正常）。启动完成前命令也无意义。
    dll.start("config/loquat.yaml", on_event)?;
    log_info!("核心已启动，等待事件（命令：help）");

    let business = Business::load(EXTENSIONS_DIR)?;
    pybridge::suppress_sigint();
    log_info!("业务已加载: {}", business.info());

    // Ctrl+C / 关闭窗口：先停核心（连带杀掉适配器子进程）再退出。
    // 必须在 CPython 初始化之后注册：SetConsoleCtrlHandler 按注册逆序调用，
    // 后注册的先执行——让我们的处理器排到 CPython 前面，否则它会把 Ctrl+C
    // 吞成 KeyboardInterrupt 或返回“已处理”截断调用链，导致退出失效。
    let dll_for_signal = Arc::clone(&dll);
    ctrlc::set_handler(move || {
        log_info!("收到 Ctrl+C，停止核心与适配器");
        dll_for_signal.stop();
        log_info!("已退出");
        std::process::exit(0);
    })
    .expect("设置 Ctrl+C 处理器失败");

    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if tx.send(HostMsg::Command(line.trim().to_string())).is_err() {
                break;
            }
        }
    });

    for msg in rx {
        match msg {
            HostMsg::Event(payload) => {
                EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
                let Some(incoming) = parse_incoming(&payload) else {
                    continue;
                };
                log_info!("消息: [{}] {}", incoming.message_type, incoming.text);

                let ctx = business::MsgCtx {
                    text: &incoming.text,
                    message_type: &incoming.message_type,
                    chat_id: incoming.chat_id,
                    user_id: incoming.user_id,
                    self_id: incoming.self_id,
                };
                let replies = match business.handle(&ctx) {
                    Ok(r) => r,
                    Err(e) => {
                        log_error!("业务处理出错: {e}");
                        continue;
                    }
                };

                let action = match incoming.message_type.as_str() {
                    "private" => "send_private_msg",
                    "group" => "send_group_msg",
                    _ => continue,
                };
                for reply in replies {
                    let params = if action == "send_private_msg" {
                        serde_json::json!({ "user_id": incoming.chat_id, "message": reply })
                    } else {
                        serde_json::json!({ "group_id": incoming.chat_id, "message": reply })
                    };
                    match dll.send_action(action, &params.to_string(), 15_000) {
                        Ok(resp) => {
                            REPLY_COUNT.fetch_add(1, Ordering::Relaxed);
                            log_info!("已回复: {} resp={resp}", brief(&reply, 60));
                        }
                        Err(e) => log_error!("发送失败: {e}"),
                    }
                }
            }
            HostMsg::Command(cmd) => run_command(&cmd, &dll, &business),
        }
    }
    Ok(())
}

// 日志预览用：图片码等长内容只留开头，避免刷屏。
fn brief(s: &str, max_chars: usize) -> String {
    if s.starts_with("[CQ:image") {
        return "[图片]".to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push('…');
    }
    out
}

// 控制台命令的应答是 REPL 输出，不是日志：裸文本走 stdout，不带时间戳。
// 异步事件（消息/回复）与运行日志才走 stderr 的统一日志格式。
fn run_command(cmd: &str, dll: &CoreDll, business: &Business) {
    match cmd {
        "" => {}
        "status" => println!(
            "核心: {} | 事件 {} 条 | 已回复 {} 条 | 业务: {}",
            dll.status(),
            EVENT_COUNT.load(Ordering::Relaxed),
            REPLY_COUNT.load(Ordering::Relaxed),
            business.info()
        ),
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
            "命令: status | plugins | install <插件名> | enable/disable <插件名> | reload napcat | exit"
        ),
        "exit" | "quit" => {
            dll.stop();
            println!("已退出");
            std::process::exit(0);
        }
        other => println!("未知命令: {other}（help 查看命令）"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 集成测试：验 FFI 触发的 reload 能让适配器子进程重生。
    // 依赖 make build 的产物与本机配置（真实 token），缺文件则提前报错。
    #[test]
    fn reload_plugin_respawns_adapter() {
        for required in [
            "bin/core.dll",
            "bin/adapter-napcat.exe",
            "config/loquat.yaml",
        ] {
            assert!(
                std::path::Path::new(required).exists(),
                "缺少 {required}，先跑 make build"
            );
        }
        let dll = unsafe { CoreDll::load("bin/core.dll").expect("加载 dll") };
        let (tx, rx) = mpsc::channel::<HostMsg>();
        EVENT_TX.set(tx).expect("EVENT_TX 只能初始化一次");
        std::thread::spawn(move || for _ in rx {});

        dll.start("config/loquat.yaml", on_event).expect("核心启动");
        wait_alive(&dll, 10_000, "初始就绪");

        assert_eq!(dll.reload_plugin("napcat"), 0, "reload 返回码非零");
        wait_alive(&dll, 10_000, "reload 后恢复");
        dll.stop();
    }

    fn wait_alive(dll: &CoreDll, timeout_ms: u64, what: &str) {
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_millis(timeout_ms);
        let mut last = String::new();
        while std::time::Instant::now() < deadline {
            last = dll.status();
            if last.contains("\"adapter_alive\":true") {
                println!("[test] {what}（耗时 {}ms）", started.elapsed().as_millis());
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("{what} 超时，最后状态: {last}");
    }
}
