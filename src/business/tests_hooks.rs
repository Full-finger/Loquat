//! 建议 6：三阶段管线。on_observe 全部执行（抢答不丢）、before_send 逐级 fold、
//! on_unload 在热重载/移除前触发、纯观察插件合法且不参与路由、info() 即钩子表。

use std::path::Path;

use super::reply::text_message;
use super::tests::{ctx, plugin_dir, write_plugin};
use super::*;

/// v2 插件：显式版本签名 + API 声明（各测试文件自带一份，保持解耦）。
fn write_v2(dir: &Path, folder: &str, code: &str) {
    let d = dir.join(folder);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join(ENTRY_FILE),
        format!("# loquat-plugin v2\nAPI = 2\n{code}"),
    )
    .unwrap();
}

fn py_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[test]
fn observe_runs_even_when_stealer_answers() {
    let dir = plugin_dir("hooks_observe");
    let log = dir.join("observed.log");
    write_v2(
        &dir,
        "aa_steal",
        "PRIORITY = 1\ndef on_message(ctx):\n    return 'stolen'\n",
    );
    // 低优先级插件：路由永远轮不到它，但 on_observe 每条都要执行
    write_v2(
        &dir,
        "zz_obs",
        &format!(
            "PRIORITY = 9\nLOG = '{}'\n\
             def on_observe(ctx):\n    open(LOG, 'a', encoding='utf-8').write(ctx['text'] + '\\n')\n\
             def on_message(ctx):\n    return 'never'\n",
            py_path(&log)
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.texts(&ctx("one")).unwrap(),
        vec!["stolen".to_string()]
    );
    assert_eq!(
        business.texts(&ctx("two")).unwrap(),
        vec!["stolen".to_string()]
    );
    // 按行比较：Windows 下 Python 文本模式写出 \r\n
    let seen: Vec<String> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(seen, vec!["one", "two"], "被抢答的消息也进了 on_observe");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn before_send_folds_winner_in_priority_order() {
    let dir = plugin_dir("hooks_fold");
    write_v2(
        &dir,
        "aa_win",
        "PRIORITY = 1\ndef on_message(ctx):\n    return 'hi'\n",
    );
    // None = 不改
    write_v2(
        &dir,
        "bb_pass",
        "PRIORITY = 2\ndef before_send(ctx, segments):\n    return None\n",
    );
    // list = 替换
    write_v2(
        &dir,
        "cc_tag",
        "PRIORITY = 3\ndef before_send(ctx, segments):\n    return segments + [{'text': {'text': '!'}}]\n",
    );
    // str = 单 text 段（与 on_message 的 v2 归一化同一套规则）
    write_v2(
        &dir,
        "dd_upper",
        "PRIORITY = 4\ndef before_send(ctx, segments):\n    return ''.join(s['text']['text'] for s in segments).upper()\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("HI!")]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn before_send_error_keeps_original_and_empty_drops() {
    let dir = plugin_dir("hooks_drop");
    write_v2(
        &dir,
        "aa_win",
        "PRIORITY = 1\ndef on_message(ctx):\n    return 'secret'\n",
    );
    // 出错：保留原段继续往下 fold
    write_v2(
        &dir,
        "bb_err",
        "PRIORITY = 2\ndef before_send(ctx, segments):\n    raise RuntimeError('boom')\n",
    );
    // 只有原段完好到达这里才会命中并丢弃
    write_v2(
        &dir,
        "cc_block",
        "PRIORITY = 3\ndef before_send(ctx, segments):\n\
         \x20   return [] if any('secret' in s.get('text', {}).get('text', '') for s in segments) else None\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert!(
        business.handle(&ctx("x")).unwrap().is_empty(),
        "bb 出错不改段，cc 看到 secret 后丢弃整条"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn on_unload_fires_on_reload_and_removal() {
    let dir = plugin_dir("hooks_unload");
    let marker = dir.join("unload.log");
    let code = format!(
        "LOG = '{}'\ndef on_unload():\n    open(LOG, 'a').write('x')\n\
         def on_message(ctx):\n    return 'ok'\n",
        py_path(&marker)
    );
    write_v2(&dir, "p", &code);
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.texts(&ctx("a")).unwrap(), vec!["ok".to_string()]);
    assert!(!marker.exists(), "加载不触发 on_unload");

    // 热重载：旧模块先 on_unload 再被替换
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_v2(&dir, "p", &code);
    assert_eq!(business.texts(&ctx("b")).unwrap(), vec!["ok".to_string()]);
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "x");
    assert_eq!(business.reloads.load(Ordering::Relaxed), 1);

    // 目录被删：移除前 on_unload 再执行一次
    std::fs::remove_dir_all(dir.join("p")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    assert!(business.texts(&ctx("c")).unwrap().is_empty());
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "xx");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn observe_only_plugin_loads_and_info_lists_hooks() {
    let dir = plugin_dir("hooks_info");
    // 既无 on_message 也无 handle：合法，不参与路由（今天会每条消息报 AttributeError）
    write_v2(
        &dir,
        "aa_watch",
        "PRIORITY = 1\ndef on_observe(ctx):\n    pass\n",
    );
    write_v2(
        &dir,
        "bb_fold",
        "PRIORITY = 2\ndef before_send(ctx, segments):\n    return None\n",
    );
    write_plugin(&dir, "zz_v1", "def handle(ctx):\n    return ['v1']\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.texts(&ctx("x")).unwrap(), vec!["v1".to_string()]);
    let info = business.info();
    assert!(info.contains("aa_watch[观察]"), "{info}");
    assert!(info.contains("bb_fold[变换]"), "{info}");
    assert!(
        info.contains("zz_v1[消息]"),
        "v1 handle 垫片为路由入口: {info}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- 建议 6 的收益：历史记录与回复分离（仿 chat 插件 v2 形状）----

/// 拷真 _lib（log/config/agent）：同 tests_agent 的 agent_test_dir，各持一份保持解耦。
fn with_agent_lib(dir: &Path) {
    for f in ["log.py", "config.py", "agent.py"] {
        let lib = dir.join("_lib");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::copy(Path::new("extensions/_lib").join(f), lib.join(f)).unwrap();
    }
}

/// 独立 chat_id（108）：sessions 是全局锚点，与其他 agent 测试错开。
fn ctx_108(text: &'static str) -> MsgCtx<'static> {
    MsgCtx {
        text,
        chat: Chat::Private {
            id: "108".to_string(),
        },
        sender_id: "1",
        self_id: "100",
        message_id: "",
        mentions_bot: false,
        images: &[],
        segments: &[],
        platform: "napcat",
        started: std::time::Instant::now(),
    }
}

#[test]
fn history_survives_stealer_via_observe_and_recorded_respond() {
    crate::pybridge::ensure_state_module();
    let dir = plugin_dir("hooks_history");
    with_agent_lib(&dir);
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    // 高优先级抢答者：除了 'chat' 一律接管
    write_v2(
        &dir,
        "aa_steal",
        "PRIORITY = 1\ndef on_message(ctx):\n    return None if ctx['text'] == 'chat' else 'stolen'\n",
    );
    // 仿 chat 插件：on_observe 无条件记历史，on_message 只对 'chat' 回复且 recorded=True；
    // fake_chat 把看到的历史原样编进回复，测试据此断言
    write_v2(
        &dir,
        "zz_chat",
        concat!(
            "PRIORITY = 9\n",
            "import loquat_agent\n",
            "CFG = {'history_limit': 20}\n",
            "class FakeResp:\n",
            "    def __init__(self, text):\n",
            "        self.text = text\n",
            "        self.tool_calls = []\n",
            "def fake_chat(cfg, history, tools):\n",
            "    return FakeResp('|'.join(m['content'] for m in history))\n",
            "def on_observe(ctx):\n",
            "    loquat_agent.observe(ctx, loquat_agent.clean_text(ctx), CFG)\n",
            "def on_message(ctx):\n",
            "    if ctx['text'] != 'chat':\n",
            "        return None\n",
            "    reply = loquat_agent.respond(ctx, text='chat', cfg=CFG, chat_fn=fake_chat,\n",
            "                                 recorded=True)\n",
            "    return reply[0] if reply else None\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    // 两条都被抢答：老实现里 chat 根本轮不到、历史全丢
    assert_eq!(
        business.texts(&ctx_108("hi")).unwrap(),
        vec!["stolen".to_string()]
    );
    assert_eq!(
        business.texts(&ctx_108("again")).unwrap(),
        vec!["stolen".to_string()]
    );
    // 第三条触发回复：模型看到的历史应含前两条，且本条用户消息只出现一次
    assert_eq!(
        business.texts(&ctx_108("chat")).unwrap(),
        vec!["hi|again|chat".to_string()],
        "on_observe 记下被抢答的消息，recorded=True 不重复追加"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
