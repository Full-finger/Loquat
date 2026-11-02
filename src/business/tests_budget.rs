//! 建议 4：执行预算与隔离。看门狗注入 TimeoutError 中断字节码死循环、
//! 预算耗尽跳过后续插件、注入无效（卡在不返回的 C 调用）标 stuck 并退避。
//!
//! 死循环测试用"直到 FLAG 文件出现"而非 `while True`：注入失效时测试
//! 仍能让循环退出，不往测试进程里泄漏一个自旋线程。

use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::tests::{ctx, plugin_dir, write_file, write_plugin};
use super::*;
use crate::{stats, watchdog};

#[test]
fn deadline_ms_reaches_plugin_as_remaining_budget() {
    let dir = plugin_dir("budget_ctx");
    write_plugin(
        &dir,
        "p",
        "def handle(ctx):\n    return [str(ctx['deadline_ms'])]\n",
    );
    let business = Business::load(dir.to_str().unwrap())
        .unwrap()
        .with_budget(Duration::from_millis(5_000));
    let v: u64 = business.texts(&ctx("x")).unwrap()[0].parse().unwrap();
    assert!(v > 0 && v <= 5_000, "deadline_ms={v}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bytecode_loop_is_interrupted_and_other_sessions_keep_working() {
    let dir = plugin_dir("budget_loop");
    let flag = dir.join("stop.flag");
    let flag_py = flag.to_string_lossy().replace('\\', "/");
    // 只对 'spin' 死循环（纯字节码：每圈回到边界，注入的 TimeoutError 立刻抛出）；
    // 其他文本放行，让另一条消息能走到 zz_next
    write_plugin(
        &dir,
        "aa_spin",
        &format!(
            "import os\nPRIORITY = 1\nFLAG = '{flag_py}'\n\
             def handle(ctx):\n\
             \x20   if ctx['text'] != 'spin':\n        return []\n\
             \x20   while not os.path.exists(FLAG):\n        pass\n\
             \x20   return ['escaped']\n"
        ),
    );
    write_plugin(
        &dir,
        "zz_next",
        "PRIORITY = 9\ndef handle(ctx):\n    return ['next']\n",
    );
    let business = Arc::new(
        Business::load(dir.to_str().unwrap())
            .unwrap()
            .with_budget(Duration::from_millis(300)),
    );
    let t0 = stats::plugin_timeouts();
    let (tx, rx) = mpsc::channel();
    let b = Arc::clone(&business);
    let started = Instant::now();
    std::thread::spawn(move || {
        // Box<dyn Error> 不是 Send，过通道前压成字符串
        let _ = tx.send(b.texts(&ctx("spin")).map_err(|e| e.to_string()));
    });

    // 验收标准：一个插件死循环时，其他会话不冻结。
    // 自旋期间从本线程发另一条消息，它应在远小于预算的时间内拿到回复
    std::thread::sleep(Duration::from_millis(50));
    let other = Instant::now();
    assert_eq!(
        business.texts(&ctx("hello")).unwrap(),
        vec!["next".to_string()]
    );
    assert!(
        other.elapsed() < Duration::from_millis(250),
        "另一会话被死循环拖慢: {:?}",
        other.elapsed()
    );

    let got = rx.recv_timeout(Duration::from_secs(10));
    // 收尾在断言之前：注入失效时也要让循环退出
    std::fs::write(&flag, "").unwrap();
    let replies = got.expect("看门狗未能在 10s 内中断死循环").unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "耗时 {:?}",
        started.elapsed()
    );
    // 预算按消息计：被打断的那条已无预算给余下插件，整条消息无回复
    assert!(replies.is_empty(), "实际: {replies:?}");
    assert!(stats::plugin_timeouts() > t0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exhausted_budget_skips_remaining_plugins() {
    let dir = plugin_dir("budget_exhaust");
    // aa 用 sleep 耗光预算（sleep 放 GIL，返回瞬间 TimeoutError 抛出）；
    // 轮到 bb 时剩余为 0，直接跳过不调用
    write_plugin(
        &dir,
        "aa_slow",
        "import time\nPRIORITY = 1\ndef handle(ctx):\n    time.sleep(0.4)\n    return ['slow']\n",
    );
    write_plugin(
        &dir,
        "bb_never",
        "PRIORITY = 2\ndef handle(ctx):\n    return ['never']\n",
    );
    let business = Business::load(dir.to_str().unwrap())
        .unwrap()
        .with_budget(Duration::from_millis(100));
    let t0 = stats::plugin_timeouts();
    let replies = business.texts(&ctx("x")).unwrap();
    assert!(replies.is_empty(), "实际: {replies:?}");
    // 计数全局，其他并行测试也会加：只断言 aa 的注入超时至少记了一次
    // （bb 因预算耗尽被跳过是后果，不单独计数）
    assert!(stats::plugin_timeouts() > t0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stuck_plugin_is_marked_and_skipped() {
    watchdog::set_stuck_grace_for_test(500);
    let dir = plugin_dir("budget_stuck");
    // 长 sleep 模拟"不返回的 C 调用"：期间没有字节码边界，注入落不下来
    write_plugin(
        &dir,
        "aa_sleep",
        "import time\nPRIORITY = 1\ndef handle(ctx):\n    time.sleep(4)\n    return ['late']\n",
    );
    write_plugin(
        &dir,
        "zz_next",
        "PRIORITY = 9\ndef handle(ctx):\n    return ['next']\n",
    );
    let business = Arc::new(
        Business::load(dir.to_str().unwrap())
            .unwrap()
            .with_budget(Duration::from_millis(200)),
    );
    let s0 = stats::plugin_stuck();
    let b = Arc::clone(&business);
    // 不 join：该线程要等 sleep 返回才结束（届时 TimeoutError 抛出、链路记一次超时）
    std::thread::spawn(move || {
        let _ = b.texts(&ctx("x"));
    });
    // 200ms 预算 + 500ms 宽限 + 扫描余量
    std::thread::sleep(Duration::from_millis(1_500));
    assert!(watchdog::is_stuck("aa_sleep"), "宽限后应标 stuck");
    assert!(stats::plugin_stuck() > s0);

    // 第二条消息：stuck 插件被跳过，立刻落到下一个，不再等 4s
    let started = Instant::now();
    assert_eq!(business.texts(&ctx("y")).unwrap(), vec!["next".to_string()]);
    assert!(started.elapsed() < Duration::from_millis(500));

    watchdog::clear_stuck("aa_sleep");
    watchdog::set_stuck_grace_for_test(watchdog::STUCK_GRACE_MS);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- agent 预算收敛：模型永远只发 tool_call，预算不足一轮即收尾 ----

/// 拷真 _lib（log/config/agent）进临时目录：同 tests_agent 的 agent_test_dir，
/// 各持一份保持模块解耦。
fn agent_test_dir(tag: &str) -> std::path::PathBuf {
    let dir = plugin_dir(tag);
    for f in ["log.py", "config.py", "agent.py"] {
        write_file(
            &dir.join("_lib"),
            f,
            &std::fs::read_to_string(std::path::Path::new("extensions/_lib").join(f)).unwrap(),
        );
    }
    dir
}

/// 独立 chat_id（107）：sessions 是全局锚点，与 tests_agent 101~105、images 106 错开。
fn ctx_budget(text: &'static str) -> MsgCtx<'static> {
    MsgCtx {
        text,
        chat: Chat::Private {
            id: "107".to_string(),
        },
        sender_id: "1",
        self_id: "100",
        message_id: "",
        mentions_bot: false,
        images: &[],
        segments: &[],
        platform: "napcat",
        started: Instant::now(),
    }
}

#[test]
fn agent_cuts_turns_when_budget_runs_out() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_budget");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    // 模型永远只发 tool_call、每轮 0.3s：没有预算会跑满 max_turns=20（6s）；
    // 有预算则剩余不足一轮即收尾，且每轮 HTTP 超时被压到剩余量以内
    write_plugin(
        &dir,
        "p",
        concat!(
            "import time\n",
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text, tool_calls):\n",
            "        self.text = text\n",
            "        self.tool_calls = tool_calls\n",
            "\n",
            "TIMEOUTS = []\n",
            "def fake_chat(cfg, history, tools):\n",
            "    TIMEOUTS.append(cfg['timeout_seconds'])\n",
            "    time.sleep(0.3)\n",
            "    return FakeResp('', [{'id': 'x', 'type': 'function',\n",
            "                          'function': {'name': 'loop', 'arguments': '{}'}}])\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg={'timeout_seconds': 60},\n",
            "                                 chat_fn=fake_chat, tools=[], max_turns=20)\n",
            "    return [f'{reply[0]}|{len(TIMEOUTS)}|{max(TIMEOUTS):.1f}']\n",
        ),
    );
    // 预算 2.6s：首轮剩 2.6s 可起，每轮 0.3s，两三轮后剩余 < 2s（MIN_TURN_SECONDS）收尾
    let business = Business::load(dir.to_str().unwrap())
        .unwrap()
        .with_budget(std::time::Duration::from_millis(2_600));
    let started = std::time::Instant::now();
    let out = business.texts(&ctx_budget("hi")).unwrap();
    let parts: Vec<&str> = out[0].split('|').collect();
    assert!(
        parts[0].contains("超时"),
        "无回复时给兜底文案，实际: {}",
        parts[0]
    );
    let turns: usize = parts[1].parse().unwrap();
    assert!(
        (1..=3).contains(&turns),
        "预算内只应跑 1~3 轮，实际 {turns}"
    );
    let max_timeout: f64 = parts[2].parse().unwrap();
    assert!(
        max_timeout <= 2.7,
        "HTTP 超时应压到剩余预算内，实际 {max_timeout}"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "没有跑满 max_turns"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
