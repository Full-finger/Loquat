// loquat 宿主（Rust）：进程入口。
// 加载 Go 核心 dll（配置中心 / 插件监督 / 消息队列都在里面），
// 收事件，调 Python 业务（pyo3 嵌入，脚本改动自动热重载），发动作。

mod business;
mod console;
mod event;
mod ffi;
mod log;
mod pluginload;
mod providers;
mod pybridge;
mod sharedlib;
mod stats;
mod webui;

use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::ffi::c_char;
use std::hash::{Hash, Hasher};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::OnceLock;

use business::Business;
use event::{Chat, Incoming, parse_incoming};
use ffi::CoreDll;

const EXTENSIONS_DIR: &str = "extensions";

static EVENT_TX: OnceLock<mpsc::Sender<HostMsg>> = OnceLock::new();

// 宿主主循环的两类输入：Go 核心的事件、控制台命令。
enum HostMsg {
    Event(Vec<u8>),
    Command(String),
}

// worker 池：事件处理挪出主循环，慢回复（LLM 最长 60s）不再阻塞
// 其他聊天与控制台命令（清单 26101801 #1）。worker 数与队列上限、
// 管线统计在 src/stats.rs。
use stats::{WORKER_COUNT, WORKER_QUEUE_CAP};

// Go 核心的事件回调：只拷贝字节进通道就返回，别在回调里干重活。
// 禁止 unwrap/expect——panic 跨 C 边界会直接炸宿主进程（见 R3 规则）。
extern "C" fn on_event(data: *const c_char, len: i32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) };
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(HostMsg::Event(bytes.to_vec()));
    }
}

// worker 内：跑责任链并直接发送（CoreDll 是 Send+Sync，Go 侧导出并发安全）。
fn handle_message(incoming: Incoming, dll: &CoreDll, business: &Business) {
    let ctx = business::MsgCtx {
        text: &incoming.text,
        chat: incoming.chat.clone(),
        sender_id: &incoming.sender_id,
        self_id: &incoming.self_id,
        message_id: &incoming.message_id,
        mentions_bot: incoming.mentions_bot,
        images: &incoming.images,
        segments: &incoming.segments,
        platform: &incoming.platform,
    };
    let replies = match business.handle(&ctx) {
        Ok(r) => r,
        Err(e) => {
            log_error!("业务处理出错: {e}");
            return;
        }
    };

    // 回复走统一发送：纯文本包成 text 段，平台翻译在适配器
    let (chat_kind, chat_id) = match &incoming.chat {
        Chat::Private { id } => ("private", id.as_str()),
        Chat::Group { id } => ("group", id.as_str()),
    };
    for reply in replies {
        let segments = serde_json::json!([{ "text": { "text": reply } }]).to_string();
        match dll.send_message(&incoming.platform, chat_kind, chat_id, &segments, 15_000) {
            Ok(resp) => {
                // Go 侧约定：失败也返回 Ok，错误装在 {"error": ...} 里。
                // 解出来走失败分支，成功统计与消息流才不被污染（26102701#5）
                if ffi_resp_failed(&resp) {
                    log_error!("发送失败: {}", console::brief(&resp, 200));
                    continue;
                }
                stats::bump_reply();
                webui::publish_reply(&incoming.platform, chat_kind, chat_id, &reply);
                log_info!("已回复: {} resp={resp}", console::brief(&reply, 60));
            }
            Err(e) => log_error!("发送失败: {e}"),
        }
    }
}

// 应答是否为 Go 侧包装的错误（{"error": ...}）。非 JSON 视为成功——
// 成功应答是 protojson（messageId），解析不出 error 字段就别拦。
fn ffi_resp_failed(resp: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(resp)
        .ok()
        .is_some_and(|v| v.get("error").is_some())
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

    pybridge::ensure_state_module();
    // 会话历史回填：磁盘上的历史在锚点就位后立即装回，业务加载前完成
    let (loaded, skipped) = pybridge::load_sessions();
    if skipped > 0 {
        log_error!("会话历史回填: {loaded} 个会话恢复，{skipped} 个文件损坏已跳过");
    } else if loaded > 0 {
        log_info!("会话历史已回填: {loaded} 个会话");
    }
    // provider 层先于业务加载：插件的 providers 注入要用
    providers::load();
    let business = Arc::new(Business::load(EXTENSIONS_DIR)?);
    pybridge::suppress_sigint();
    log_info!("业务已加载: {}", business.info());

    // WebUI：REST + 静态页 + 消息流 WS，独立线程，起不来只记日志。
    // 选项卡经通用 tab 注册机制挂上；注册顺序 = 侧栏顺序
    webui::register_tab("providers", "服务", "/providers");
    webui::register_tab("adapters", "适配器", "/adapters");
    webui::register_tab("sandbox", "沙箱", "/sandbox");
    webui::spawn(Arc::clone(&dll), Arc::clone(&business));

    // worker 池：见顶部常量注释。catch_unwind 兜一条消息的 panic 只损失
    // 一次迭代——线程死亡会让该 chat 后续消息静默失去去处。
    let workers: Vec<mpsc::SyncSender<Incoming>> = (0..WORKER_COUNT)
        .map(|idx| {
            let (tx, rx) = mpsc::sync_channel::<Incoming>(WORKER_QUEUE_CAP);
            let worker_dll = Arc::clone(&dll);
            let worker_business = Arc::clone(&business);
            std::thread::spawn(move || {
                for incoming in rx {
                    let chat_id = incoming.chat.id().to_string();
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handle_message(incoming, &worker_dll, &worker_business);
                    }));
                    // 无论 panic 与否都归还计数，统计口径才守得住（agents 命令）
                    stats::worker_settle(idx, &chat_id);
                }
            });
            tx
        })
        .collect();

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
                stats::bump_event();
                let Some(incoming) = parse_incoming(&payload) else {
                    continue;
                };
                log_info!(
                    "消息: [{} {}] {}",
                    incoming.chat.message_type(),
                    incoming.chat.id(),
                    incoming.text
                );
                webui::publish_incoming(
                    &incoming.platform,
                    incoming.chat.message_type(),
                    incoming.chat.id(),
                    &incoming.text,
                );

                // chat ID 哈希定 worker：同一会话恒在同一线程串行（插件模块级状态免锁）
                let idx = {
                    let mut h = DefaultHasher::new();
                    incoming.chat.id().hash(&mut h);
                    (h.finish() % WORKER_COUNT as u64) as usize
                };
                let chat_id = incoming.chat.id().to_string();
                match workers[idx].try_send(incoming) {
                    Ok(()) => stats::worker_enqueue(idx, &chat_id),
                    Err(mpsc::TrySendError::Full(dropped)) => {
                        log_error!(
                            "worker {idx} 队列已满，丢弃消息（chat={}）",
                            dropped.chat.id()
                        );
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        log_error!("worker {idx} 已停止，丢弃消息");
                    }
                }
            }
            HostMsg::Command(cmd) => console::run_command(&cmd, &dll, &business),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_error_envelope_is_detected() {
        assert!(ffi_resp_failed(r#"{"error":"发送被 NapCat 拒绝: retcode=1200"}"#));
        assert!(!ffi_resp_failed(r#"{"messageId":"123"}"#));
        assert!(!ffi_resp_failed("not json"), "非 JSON 不拦");
    }

    // 集成测试：验 FFI 触发的 reload 能让适配器子进程重生。
    // 依赖 bin/ 产物与本机配置（真实 token）：缺依赖时打印原因后 return
    // （skip 语义，26102202#11）——外部环境 cargo test 不再红在环境检查上，
    // 全套验证走本地 just test
    #[test]
    fn reload_plugin_respawns_adapter() {
        for required in [
            "bin/core.dll",
            "bin/adapter-napcat.exe",
            "config/loquat.yaml",
        ] {
            if !std::path::Path::new(required).exists() {
                println!("跳过集成测试：缺 {required}（先跑 just build 并配置 token）");
                return;
            }
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
            if last.contains("\"alive\":true") {
                println!("[test] {what}（耗时 {}ms）", started.elapsed().as_millis());
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("{what} 超时，最后状态: {last}");
    }
}

