// loquat 宿主（Rust）：进程入口。
// 加载 Go 核心 dll（配置中心 / 插件监督 / 消息队列都在里面），
// 收事件，调 Python 业务（pyo3 嵌入，脚本改动自动热重载），发动作。

mod business;
mod console;
mod dedupe;
mod event;
mod ffi;
mod log;
mod outbound;
mod pluginload;
mod providers;
mod pybridge;
mod sharedlib;
mod stats;
mod webui;

use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::ffi::c_char;
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::OnceLock;

use business::Business;
use event::{parse_incoming, Chat, Incoming};
use ffi::CoreDll;

const EXTENSIONS_DIR: &str = "extensions";

static EVENT_TX: OnceLock<mpsc::SyncSender<HostMsg>> = OnceLock::new();

// 宿主主循环的三类输入：Go 核心的事件、控制台命令、退出信号。
// 有界队列（评审建议 3）：满时 Go 侧 send 阻塞形成反压，事件改由
// 核心队列承载并计数（events_dropped），不再是无界内存隐患。
const MAIN_QUEUE_CAP: usize = 256;
enum HostMsg {
    Event(Vec<u8>),
    Command(String),
    /// Ctrl+C / console exit：主循环排空 worker 后正常退出（评审建议 9：
    /// 不再 process::exit 硬退，正在处理的会话历史可能丢最后一次写入）
    Shutdown,
}

// worker 池：事件处理挪出主循环，慢回复（LLM 最长 60s）不再阻塞
// 其他聊天与控制台命令（清单 26101801 #1）。worker 数与队列上限、
// 管线统计在 src/stats.rs。
use stats::{WORKER_COUNT, WORKER_QUEUE_CAP};

// Go 核心的事件回调：拷贝字节进通道就返回；队列满则阻塞——这正是
// 反压（建议 3），Go 核心的 pump 暂停拉取，事件在其队列排队并计数。
// 禁止 unwrap/expect——panic 跨 C 边界会直接炸宿主进程（见 R3 规则）。
extern "C" fn on_event(data: *const c_char, len: i32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) };
    if let Some(tx) = EVENT_TX.get() {
        // 主队列占用观测：出入各记一笔
        stats::main_enqueue();
        if tx.send(HostMsg::Event(bytes.to_vec())).is_err() {
            stats::main_dequeue();
        }
    }
}

// worker 内：只跑责任链并产出回复（发送交出站队列，评审建议 2），
// 15s 发送超时不再占住 worker。
fn handle_message(incoming: Incoming, _dll: &CoreDll, business: &Business) {
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
    let chain_started = std::time::Instant::now();
    let replies = match business.handle(&ctx) {
        Ok(r) => r,
        Err(e) => {
            log_error!("业务处理出错: {e}");
            stats::record_chain(chain_started.elapsed());
            return;
        }
    };
    stats::record_chain(chain_started.elapsed());

    // 回复交出站队列：限流、重试、统计都在 sender 池（建议 2）；
    // 每条 Message 一次发送，段列表原样下发（建议 5）
    for msg in replies {
        outbound::enqueue(&incoming.platform, &incoming.chat, msg);
    }
}

/// worker 队列满的降级回复（建议 3）：只对私聊说一句，群聊不回
/// （刷屏 + 风控风险）。直接同步发送、不重试不入队——它本身就是
/// 队列满时的产物，再入队只会跟着一起丢。
fn busy_reply(dll: &CoreDll, incoming: &Incoming) {
    if !matches!(incoming.chat, Chat::Private { .. }) {
        return;
    }
    let text = "消息有点多，刚才那条没接住，稍后再试～";
    let segments = serde_json::json!([{ "text": { "text": text } }]).to_string();
    if let Err(e) = dll.send_message(
        &incoming.platform,
        "private",
        incoming.chat.id(),
        &segments,
        5_000,
    ) {
        log_error!("降级回复发送失败: {e}");
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let dll = Arc::new(unsafe { CoreDll::load("bin/core.dll")? });
    ffi::set_core_dll(dll.clone());
    // 接管核心日志泵：此后三边日志都在宿主 stderr 锁下单一写者打印。
    // 这是首次 FFI 调用，顺带完成 Go 运行时初始化（须在 stdin 线程前）。
    log::pump_from_handle(dll.log_pipe());
    log_info!("core.dll 已加载");

    let (tx, rx) = mpsc::sync_channel::<HostMsg>(MAIN_QUEUE_CAP);
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
    // 内置页面导航在前端 tabs.ts；插件页面（TABS/ui.js）由 /api/extensions/ui 聚合。
    webui::spawn(Arc::clone(&dll), Arc::clone(&business));

    // 出站 sender 池先于 worker 起：回复一产出就有去处（评审建议 2）
    outbound::spawn_senders(Arc::clone(&dll));

    // worker 池：见顶部常量注释。catch_unwind 兜一条消息的 panic 只损失
    // 一次迭代——线程死亡会让该 chat 后续消息静默失去去处。
    // workers 与线程句柄成对收集：句柄在退出时 join，排空在途消息
    let (workers, worker_handles): (Vec<mpsc::SyncSender<Incoming>>, Vec<_>) = (0..WORKER_COUNT)
        .map(|idx| {
            let (tx, rx) = mpsc::sync_channel::<Incoming>(WORKER_QUEUE_CAP);
            let worker_dll = Arc::clone(&dll);
            let worker_business = Arc::clone(&business);
            let h = std::thread::spawn(move || {
                for incoming in rx {
                    let chat_id = incoming.chat.id().to_string();
                    // panic 载荷必须留痕：静默吞掉会让会话砖化且查不到原因（26102701#8）
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handle_message(incoming, &worker_dll, &worker_business);
                    }));
                    if let Err(payload) = outcome {
                        log_error!("消息处理 panic（chat={chat_id}）：{}", panic_text(&payload));
                    }
                    // 无论 panic 与否都归还计数，统计口径才守得住（agents 命令）
                    stats::worker_settle(idx, &chat_id);
                }
            });
            (tx, h)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .unzip();

    // Ctrl+C：第一次触发主循环排空退出，第二次直接强退（用户不耐烦时
    // 的保底）。必须在 CPython 初始化之后注册：SetConsoleCtrlHandler 按
    // 注册逆序调用，后注册的先执行——让我们的处理器排到 CPython 前面，
    // 否则它会把 Ctrl+C 吞成 KeyboardInterrupt 或返回"已处理"截断调用链，
    // 导致退出失效。
    let shutdown_tx = tx.clone();
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag_for_signal = Arc::clone(&shutdown_flag);
    ctrlc::set_handler(move || {
        if flag_for_signal.swap(true, Ordering::SeqCst) {
            std::process::exit(0);
        }
        log_info!("收到 Ctrl+C，排空在途消息后退出（再按一次强制退出）");
        let _ = shutdown_tx.send(HostMsg::Shutdown);
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
                stats::main_dequeue();
                stats::bump_event();
                let parse_started = std::time::Instant::now();
                let Some(incoming) = parse_incoming(&payload) else {
                    stats::record_parse(parse_started.elapsed());
                    continue;
                };
                stats::record_parse(parse_started.elapsed());
                // 消息去重：重连重放/上游重试会重复投递，LLM 场景即重复计费
                if dedupe::duplicate(&incoming.platform, &incoming.message_id) {
                    stats::bump_duplicate();
                    continue;
                }
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
                        stats::bump_worker_dropped();
                        log_error!(
                            "worker {idx} 队列已满，丢弃消息（chat={}）",
                            dropped.chat.id()
                        );
                        busy_reply(&dll, &dropped); // 私聊降级回复（建议 3）
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        stats::bump_worker_dropped();
                        log_error!("worker {idx} 已停止，丢弃消息");
                    }
                }
            }
            HostMsg::Command(cmd) => {
                if console::run_command(&cmd, &dll, &business) {
                    break;
                }
            }
            HostMsg::Shutdown => break,
        }
    }

    // 优雅退出（评审建议 9）：关 worker 队列入口，等在途消息处理完
    // （会话落盘在 handle 内完成），再停核心（连带杀适配器子进程）
    drop(workers);
    for h in worker_handles {
        let _ = h.join();
    }
    log_info!("在途消息已排空，停止核心与适配器");
    dll.stop();
    Ok(())
}

/// panic 载荷转文本：`panic!("...")` 与 `panic!("{}", x)` 分别是
/// &str/String，其余类型报缺省文案兜底，保证事故可观测。
fn panic_text(payload: &Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "非文本 panic 载荷".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_error_envelope_is_detected() {
        assert!(ffi::resp_failed(
            r#"{"error":"发送被 NapCat 拒绝: retcode=1200"}"#
        ));
        assert!(!ffi::resp_failed(r#"{"messageId":"123"}"#));
        assert!(!ffi::resp_failed("not json"), "非 JSON 不拦");
    }

    #[test]
    fn panic_text_handles_common_payloads() {
        assert_eq!(
            panic_text(&(Box::new("boom") as Box<dyn Any + Send>)),
            "boom"
        );
        assert_eq!(
            panic_text(&(Box::new(String::from("code 7")) as Box<dyn Any + Send>)),
            "code 7"
        );
        assert_eq!(
            panic_text(&(Box::new(42usize) as Box<dyn Any + Send>)),
            "非文本 panic 载荷"
        );
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
        let (tx, rx) = mpsc::sync_channel::<HostMsg>(MAIN_QUEUE_CAP);
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
