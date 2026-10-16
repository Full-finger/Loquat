// loquat 宿主（Rust）：进程入口。
// 加载 Go 核心 dll（配置中心 / 插件监督 / 消息队列都在里面），
// 收事件，调 Python 业务（pyo3 嵌入，脚本改动自动热重载），发动作。

mod business;
mod ffi;

use std::error::Error;
use std::ffi::c_char;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, OnceLock, Arc};

use business::Business;
use ffi::CoreDll;

const QA_SCRIPT: &str = "extensions/qa.py";

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
    text: String,
}

fn parse_incoming(payload: &[u8]) -> Option<Incoming> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    if v.get("post_type")?.as_str()? != "message" {
        return None;
    }
    let message_type = v.get("message_type")?.as_str()?.to_string();
    let chat_id = match message_type.as_str() {
        "private" => v.get("user_id")?.as_i64()?,
        "group" => v.get("group_id")?.as_i64()?,
        _ => return None,
    };
    Some(Incoming {
        message_type,
        chat_id,
        text: v.get("raw_message")?.as_str()?.to_string(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let dll = Arc::new(unsafe { CoreDll::load("bin/core.dll")? });
    ffi::set_core_dll(dll.clone());
    println!("[rust] core.dll 已加载");

    let (tx, rx) = mpsc::channel::<HostMsg>();
    EVENT_TX.set(tx.clone()).expect("事件通道只能初始化一次");

    // 先起 Go 核心，再初始化 Python 与 stdin 线程：
    // 1) CPython 先于 Go 运行时初始化会在 Windows 上互相卡死；
    // 2) 阻塞读 stdin 的线程若先于首次 FFI 启动，Go 运行时初始化会直接冻死
    //   （实测：stdin 给 DEVNULL 则一切正常）。启动完成前命令也无意义。
    dll.start("config/loquat.yaml", on_event)?;
    println!("[rust] 核心已启动，等待事件（命令：help）");

    let business = Business::load(QA_SCRIPT)?;
    println!("[rust] 业务已加载: {QA_SCRIPT}");

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
                println!("[rust] 消息: [{}] {}", incoming.message_type, incoming.text);

                let replies = match business.handle(&incoming.text) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[rust] 业务处理出错: {e}");
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
                            println!("[rust] 已回复: {reply} resp={resp}");
                        }
                        Err(e) => eprintln!("[rust] 发送失败: {e}"),
                    }
                }
            }
            HostMsg::Command(cmd) => run_command(&cmd, &dll, &business),
        }
    }
    Ok(())
}

fn run_command(cmd: &str, dll: &CoreDll, business: &Business) {
    match cmd {
        "" => {}
        "status" => println!(
            "[rust] 核心: {} | 事件 {} 条 | 已回复 {} 条 | 业务: {}",
            dll.status(),
            EVENT_COUNT.load(Ordering::Relaxed),
            REPLY_COUNT.load(Ordering::Relaxed),
            business.info()
        ),
        "reload napcat" => {
            let rc = dll.reload_plugin("napcat");
            println!(
                "[rust] reload napcat -> {}",
                if rc == 0 {
                    "已触发，核心将退避重启适配器"
                } else {
                    "失败（核心未启动）"
                }
            );
        }
        "help" => println!("[rust] 命令: status | reload napcat | exit"),
        "exit" | "quit" => {
            dll.stop();
            println!("[rust] 已退出");
            std::process::exit(0);
        }
        other => println!("[rust] 未知命令: {other}（help 查看命令）"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 集成测试：验 FFI 触发的 reload 能让适配器子进程重生。
    // 依赖 make build 的产物与本机配置（真实 token），缺文件则提前报错。
    #[test]
    fn reload_plugin_respawns_adapter() {
        for required in ["bin/core.dll", "bin/adapter-napcat.exe", "config/loquat.yaml"] {
            assert!(
                std::path::Path::new(required).exists(),
                "缺少 {required}，先跑 make build"
            );
        }
        let dll = unsafe { CoreDll::load("bin/core.dll").expect("加载 dll") };
        let (tx, rx) = mpsc::channel::<HostMsg>();
        EVENT_TX.set(tx).expect("EVENT_TX 只能初始化一次");
        std::thread::spawn(move || {
            for _ in rx {}
        });

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
                println!(
                    "[test] {what}（耗时 {}ms）",
                    started.elapsed().as_millis()
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("{what} 超时，最后状态: {last}");
    }
}
