//! agent 多模态（图片）测试。从 tests_agent.rs 拆出（R4 限 400 行）。
//! helpers 与 tests_agent.rs 各持一份，保持模块解耦。

use std::path::{Path, PathBuf};

use super::*;
use crate::pluginload::SIGNATURE;

fn plugin_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loquat_biz_agent_img_test_{tag}"));
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
            chat: Chat::Private {
                id: chat_id.to_string(),
            },
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
        .handle_text(&msg_ctx(IMAGES, "VISION", &imgs))
        .unwrap();
    assert!(replies[0].contains("\"image_url\""), "实际: {replies:?}");
    assert!(
        replies[0].contains("https://qq.com/a.jpg"),
        "实际: {replies:?}"
    );
    assert!(
        replies[0].contains("\"type\": \"text\""),
        "实际: {replies:?}"
    );

    // vision 关闭：同图降级为 [图片] 占位文本，URL 不进模型
    let replies = business
        .handle_text(&msg_ctx(IMAGES_OFF, "hi", &imgs))
        .unwrap();
    assert!(replies[0].contains("[图片]"), "实际: {replies:?}");
    assert!(!replies[0].contains("image_url"), "实际: {replies:?}");

    // 无文本无图片：不进模型，无回复
    let replies = business
        .handle_text(&msg_ctx(IMAGES_OFF + 1, "", &[]))
        .unwrap();
    assert!(replies.is_empty(), "实际: {replies:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// 各测试用独立 chat_id：sessions 是全局锚点，串行测试下 key 撞车会互相污染。
const IMAGES: i64 = 106;
const IMAGES_OFF: i64 = 107;
