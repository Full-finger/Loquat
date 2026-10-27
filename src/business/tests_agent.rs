//! loquat_agent（内核 agent 库）的测试：历史锚点、QQ→LLM 文本转换、工具循环。
//! 从 tests.rs 拆出（R4 限 400 行）。helpers 与 tests.rs 各持一份，保持模块解耦。

use std::path::Path;
use std::path::PathBuf;

use super::*;
use crate::pluginload::SIGNATURE;

fn plugin_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loquat_biz_agent_test_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_plugin(dir: &Path, folder: &str, code: &str) {
    let d = dir.join(folder);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(ENTRY_FILE), format!("{SIGNATURE}\n{code}")).unwrap();
}

fn write_file(dir: &Path, name: &str, content: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(name), content).unwrap();
}

fn ctx_in(chat_id: i64, text: &'static str) -> MsgCtx<'static> {
    MsgCtx {
        text,
        chat: Chat::Private { id: chat_id.to_string() },
        sender_id: "1",
        self_id: "100",
        message_id: "",
        mentions_bot: false,
        images: &[],
        segments: &[],
        platform: "napcat",
    }
}

// 各测试用独立 chat_id：sessions 是全局锚点，串行测试下 key 撞车会互相污染。
mod chat_ids {
    pub const HISTORY: i64 = 101;
    pub const TOOLS: i64 = 102;
    pub const UNKNOWN: i64 = 103;
    pub const MAX_TURNS: i64 = 104;
    pub const PERSIST: i64 = 105;
    pub const IMAGES: i64 = 106;
    pub const IMAGES_OFF: i64 = 107;
}

// 布置临时插件目录：拷真 _lib（log/config/agent）+ 测试插件。
fn agent_test_dir(tag: &str) -> PathBuf {
    let dir = plugin_dir(tag);
    for f in ["log.py", "config.py", "agent.py"] {
        write_file(
            &dir.join("_lib"),
            f,
            &std::fs::read_to_string(Path::new("extensions/_lib").join(f)).unwrap(),
        );
    }
    dir
}

#[test]
fn agent_history_survives_plugin_reload() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_reload");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    let code = concat!(
        "import loquat_agent\n",
        "\n",
        "class FakeResp:\n",
        "    def __init__(self, text):\n",
        "        self.text = text\n",
        "        self.tool_calls = []\n",
        "\n",
        "def fake_chat(cfg, history, tools):\n",
        "    return FakeResp('H' + str(len(history)))\n",
        "\n",
        "def handle(ctx):\n",
        "    text = loquat_agent.clean_text(ctx)\n",
        "    reply = loquat_agent.respond(ctx, text=text, cfg={'history_limit': 3}, chat_fn=fake_chat)\n",
        "    return reply or []\n",
    );
    write_plugin(&dir, "p", code);
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let c = |t| ctx_in(chat_ids::HISTORY, t);
    // m1 → 历史 [u1]；m2 → [u1,a1,u2] → 回复后裁剪 [a1,u2,a2]
    assert_eq!(business.handle(&c("m1")).unwrap(), vec!["H1".to_string()]);
    assert_eq!(business.handle(&c("m2")).unwrap(), vec!["H3".to_string()]);

    // 重载插件：模块级状态全换，历史必须从 loquat_state 锚点回来。
    // 若锚点失效，m3 只会看到 [u3]，返回 H1 而非 H4
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_plugin(&dir, "p", code);
    assert_eq!(business.handle(&c("m3")).unwrap(), vec!["H4".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_strips_cq_and_prefixes_group_sender() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_group");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    // spy 插件：捕获 fake_chat 实际收到的 history，验证 CQ 码剥离
    // 与群聊发言人前缀
    write_plugin(
        &dir,
        "spy",
        concat!(
            "import json\n",
            "import loquat_agent\n",
            "\n",
            "captured = []\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text):\n",
            "        self.text = text\n",
            "        self.tool_calls = []\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    captured.append(json.dumps(history, ensure_ascii=False))\n",
            "    return FakeResp('ok')\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    loquat_agent.respond(ctx, text=text, cfg={}, chat_fn=fake_chat)\n",
            "    return [captured[-1]] if captured else []\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let group_ctx = MsgCtx {
        text: "hi",
        chat: Chat::Group { id: "9".to_string() },
        sender_id: "7",
        self_id: "100",
        message_id: "",
        mentions_bot: true,
        images: &[],
        segments: &[],
        platform: "napcat",
    };
    assert_eq!(
        business.handle(&group_ctx).unwrap(),
        vec![r#"[{"role": "user", "content": "<7> hi"}]"#.to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_tool_loop_executes_and_passes_schema() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_tools");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    write_plugin(
        &dir,
        "p",
        concat!(
            "import json\n",
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text, tool_calls):\n",
            "        self.text = text\n",
            "        self.tool_calls = tool_calls\n",
            "\n",
            "@loquat_agent.tool\n",
            "def add(ctx, a: int, b: int) -> str:\n",
            "    \x22\x22\x22两数相加\x22\x22\x22\n",
            "    return str(a + b)\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    tool_msgs = [m['content'] for m in history if m.get('role') == 'tool']\n",
            "    if tool_msgs:\n",
            "        return FakeResp('schema=%s answer=%s' % (json.dumps(tools), tool_msgs[-1]), [])\n",
            "    return FakeResp('', [{'id': 'c1', 'type': 'function',\n",
            "                          'function': {'name': 'add', 'arguments': '{\"a\": 2, \"b\": 3}'}}])\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg={'history_limit': 20},\n",
            "                                 chat_fn=fake_chat, tools=[add])\n",
            "    return reply or []\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let replies = business.handle(&ctx_in(chat_ids::TOOLS, "1+1=?")).unwrap();
    assert_eq!(replies.len(), 1);
    // 工具执行结果（2+3=5）被回填并在第二轮被模型消费
    assert!(replies[0].contains("answer=5"), "实际: {replies:?}");
    // OpenAI tools schema 随请求下发：函数名与注解推导的类型都在
    assert!(replies[0].contains("\"add\""), "实际: {replies:?}");
    assert!(replies[0].contains("\"integer\""), "实际: {replies:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unknown_tool_error_feeds_back_to_model() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_unknown_tool");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    write_plugin(
        &dir,
        "p",
        concat!(
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text, tool_calls):\n",
            "        self.text = text\n",
            "        self.tool_calls = tool_calls\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    if any(m.get('role') == 'tool' for m in history):\n",
            "        last = [m['content'] for m in history if m.get('role') == 'tool'][-1]\n",
            "        return FakeResp('saw=%s' % last, [])\n",
            "    return FakeResp('', [{'id': 'x', 'type': 'function',\n",
            "                          'function': {'name': 'nope', 'arguments': '{}'}}])\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg={}, chat_fn=fake_chat, tools=[])\n",
            "    return reply or []\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    // dispatch 对未知工具回填错误文本，模型第二轮看到并转述
    let replies = business.handle(&ctx_in(chat_ids::UNKNOWN, "hi")).unwrap();
    assert_eq!(replies, vec!["saw=错误：未知工具 'nope'".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_tool_loop_caps_at_max_turns() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_max_turns");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    write_plugin(
        &dir,
        "p",
        concat!(
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text, tool_calls):\n",
            "        self.text = text\n",
            "        self.tool_calls = tool_calls\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    return FakeResp('', [{'id': 'x', 'type': 'function',\n",
            "                          'function': {'name': 'loop', 'arguments': '{}'}}])\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg={}, chat_fn=fake_chat,\n",
            "                                 tools=[], max_turns=2)\n",
            "    return reply or ['EMPTY']\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    // 模型永远只发 tool_call：max_turns 用尽强制收尾，本轮无文本回复
    assert_eq!(
        business.handle(&ctx_in(chat_ids::MAX_TURNS, "hi")).unwrap(),
        vec!["EMPTY".to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_sessions_persist_to_disk_and_reload() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_persist");
    let sess_dir = dir.join("sess");
    crate::pybridge::set_persist_dir(&sess_dir.to_string_lossy());
    write_plugin(
        &dir,
        "p",
        concat!(
            "import sys\n",
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text):\n",
            "        self.text = text\n",
            "        self.tool_calls = []\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    return FakeResp('R' + str(len(history)))\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    if text == 'WIPE':\n",
            "        # 模拟重启：内存历史清空，只剩磁盘上的\n",
            "        sys.modules['loquat_state'].sessions.clear()\n",
            "        return ['wiped']\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg={}, chat_fn=fake_chat)\n",
            "    return reply or []\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let c = |t| ctx_in(chat_ids::PERSIST, t);

    // 第一轮：历史进内存，同时落盘
    assert_eq!(business.handle(&c("m1")).unwrap(), vec!["R1".to_string()]);
    let file = sess_dir.join(format!("private_{}.json", chat_ids::PERSIST));
    assert!(file.exists(), "落盘文件应存在");
    let raw = std::fs::read_to_string(&file).unwrap();
    assert!(raw.contains("\"version\": 1"));
    assert!(raw.contains("m1"), "落盘内容应含用户消息，实际: {raw}");

    // 模拟重启：内存清空，磁盘保留；回填后历史回来
    assert_eq!(business.handle(&c("WIPE")).unwrap(), vec!["wiped".to_string()]);
    let (loaded, skipped) = crate::pybridge::load_sessions();
    assert_eq!((loaded, skipped), (1, 0));
    // 回填后的历史 [u:m1, a:R1]，再发 m2 → 3 条 → fake_chat 返回 R3
    assert_eq!(business.handle(&c("m2")).unwrap(), vec!["R3".to_string()]);

    // 损坏文件跳过：写一个坏的进去，重建后只恢复好的
    std::fs::write(sess_dir.join("group_999.json"), "not json").unwrap();
    sys_sessions_clear();
    let (loaded, skipped) = crate::pybridge::load_sessions();
    assert_eq!((loaded, skipped), (1, 1), "坏文件应计数跳过");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_images_multimodal_or_placeholder() {
    crate::pybridge::ensure_state_module();
    let dir = agent_test_dir("agent_images");
    crate::pybridge::set_persist_dir(&dir.join("sess").to_string_lossy());
    // spy 插件：cfg 由文本切换（VISION=识图开），fake_chat 回传收到的历史
    write_plugin(
        &dir,
        "spy",
        concat!(
            "import json\n",
            "import loquat_agent\n",
            "\n",
            "class FakeResp:\n",
            "    def __init__(self, text):\n",
            "        self.text = text\n",
            "        self.tool_calls = []\n",
            "\n",
            "def fake_chat(cfg, history, tools):\n",
            "    return FakeResp(json.dumps(history, ensure_ascii=False))\n",
            "\n",
            "def handle(ctx):\n",
            "    text = loquat_agent.clean_text(ctx)\n",
            "    cfg = {'vision': True} if text == 'VISION' else {}\n",
            "    reply = loquat_agent.respond(ctx, text=text, cfg=cfg, chat_fn=fake_chat)\n",
            "    return reply or []\n",
        ),
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let imgs = vec![
        "https://qq.com/a.jpg".to_string(),
        "https://qq.com/b.png".to_string(),
    ];
    fn msg_ctx<'a>(chat_id: i64, text: &'static str, images: &'a [String]) -> MsgCtx<'a> {
        MsgCtx {
            text,
            chat: Chat::Private { id: chat_id.to_string() },
            sender_id: "1",
            self_id: "100",
            message_id: "",
            mentions_bot: false,
            images,
            segments: &[],
            platform: "napcat",
        }
    }

    // vision 开启：content 是多模态 parts（text + image_url）
    let replies = business
        .handle(&msg_ctx(chat_ids::IMAGES, "VISION", &imgs))
        .unwrap();
    assert!(replies[0].contains("\"image_url\""), "实际: {replies:?}");
    assert!(replies[0].contains("https://qq.com/a.jpg"), "实际: {replies:?}");
    assert!(replies[0].contains("\"type\": \"text\""), "实际: {replies:?}");

    // vision 关闭：同图降级为 [图片] 占位文本，URL 不进模型
    let replies = business
        .handle(&msg_ctx(chat_ids::IMAGES_OFF, "hi", &imgs))
        .unwrap();
    assert!(replies[0].contains("[图片]"), "实际: {replies:?}");
    assert!(!replies[0].contains("image_url"), "实际: {replies:?}");

    // 无文本无图片：不进模型，无回复
    let replies = business
        .handle(&msg_ctx(chat_ids::IMAGES_OFF + 1, "", &[]))
        .unwrap();
    assert!(replies.is_empty(), "实际: {replies:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// 测试辅助：清空锚点 sessions（模拟重启后空内存）。
fn sys_sessions_clear() {
    crate::pybridge::for_test_clear_sessions();
}
