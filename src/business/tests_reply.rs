//! 建议 5：回复类型升级。v1 存量语义（list[str]，每串一条消息）不变；
//! v2 可返回 str / 段列表 / None，段列表是**一条**消息，构造器 loquat_seg。

use std::path::Path;

use serde_json::json;

use super::reply::text_message;
use super::tests::{ctx, plugin_dir, write_file, write_plugin};
use super::*;

/// v2 插件：显式版本签名 + API 声明（write_plugin 固定写 v1 签名，这里自己写）。
fn write_v2(dir: &Path, folder: &str, code: &str) {
    let d = dir.join(folder);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join(ENTRY_FILE),
        format!("# loquat-plugin v2\nAPI = 2\n{code}"),
    )
    .unwrap();
}

/// 把真 _lib/seg.py 拷进临时目录，让插件能 import loquat_seg（同 tests_agent 拷 agent.py 的手法）。
fn with_seg_lib(dir: &Path) {
    write_file(
        &dir.join("_lib"),
        "seg.py",
        &std::fs::read_to_string("extensions/_lib/seg.py").unwrap(),
    );
}

#[test]
fn v1_strings_become_one_text_message_each() {
    let dir = plugin_dir("reply_v1");
    write_plugin(&dir, "p", "def handle(ctx):\n    return ['a', 'b']\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("a"), text_message("b")],
        "v1 每个字符串独立成一条消息"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_str_is_one_text_message() {
    let dir = plugin_dir("reply_v2_str");
    write_v2(&dir, "p", "def handle(ctx):\n    return 'hello'\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("hello")]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_segment_list_is_one_message_via_loquat_seg() {
    let dir = plugin_dir("reply_v2_segs");
    with_seg_lib(&dir);
    write_v2(
        &dir,
        "p",
        "import loquat_seg as seg\n\
         def handle(ctx):\n\
         \x20   return [seg.text('look'), seg.image('https://x/y.png'),\n\
         \x20           seg.reply(ctx['message_id']), seg.at(7),\n\
         \x20           seg.image(inline=b'\\x89PNG'), 'tail']\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let c = MsgCtx {
        text: "x",
        chat: Chat::Private {
            id: "1".to_string(),
        },
        sender_id: "1",
        self_id: "100",
        message_id: "m42",
        mentions_bot: false,
        images: &[],
        segments: &[],
        platform: "napcat",
        started: std::time::Instant::now(),
    };
    let replies = business.handle(&c).unwrap();
    assert_eq!(replies.len(), 1, "扁平段列表 = 一条消息");
    assert_eq!(
        replies[0],
        vec![
            json!({ "text": { "text": "look" } }),
            json!({ "image": { "url": "https://x/y.png" } }),
            json!({ "reply": { "messageId": "m42" } }),
            json!({ "at": { "target": "7" } }),
            json!({ "image": { "inline": "iVBORw==" } }), // b'\x89PNG' 的 base64
            json!({ "text": { "text": "tail" } }),        // str 元素宽容为 text 段
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_none_and_empty_pass_the_chain() {
    let dir = plugin_dir("reply_v2_empty");
    write_v2(
        &dir,
        "aa_none",
        "PRIORITY = 1\ndef handle(ctx):\n    return None\n",
    );
    write_v2(
        &dir,
        "bb_blank",
        "PRIORITY = 2\ndef handle(ctx):\n    return ''\n",
    );
    write_v2(
        &dir,
        "cc_list",
        "PRIORITY = 3\ndef handle(ctx):\n    return []\n",
    );
    write_v2(
        &dir,
        "zz_last",
        "PRIORITY = 9\ndef handle(ctx):\n    return 'last'\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("last")],
        "None / '' / [] 都不接管，链继续"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_bad_return_errors_and_chain_continues() {
    let dir = plugin_dir("reply_v2_bad");
    // 未知段键、非对象内容、非法返回类型：各自报错，但只跳过自身
    write_v2(
        &dir,
        "aa_badkey",
        "PRIORITY = 1\ndef handle(ctx):\n    return [{'bogus': {}}]\n",
    );
    write_v2(
        &dir,
        "bb_badbody",
        "PRIORITY = 2\ndef handle(ctx):\n    return [{'image': 'https://x'}]\n",
    );
    write_v2(
        &dir,
        "cc_int",
        "PRIORITY = 3\ndef handle(ctx):\n    return 42\n",
    );
    write_v2(
        &dir,
        "dd_nested",
        "PRIORITY = 4\ndef handle(ctx):\n    return [[{'text': {'text': 'no'}}]]\n",
    );
    write_v2(
        &dir,
        "zz_ok",
        "PRIORITY = 9\ndef handle(ctx):\n    return 'ok'\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("ok")]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_api_declaration_must_match_signature() {
    let dir = plugin_dir("reply_v2_api");
    // 签名 v2 但 API = 3：拒载（不静默降级）
    let bad = dir.join("mismatch");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(
        bad.join(ENTRY_FILE),
        "# loquat-plugin v2\nAPI = 3\ndef handle(ctx):\n    return 'no'\n",
    )
    .unwrap();
    // 签名 v2、未声明 API：以签名为准，正常加载
    let ok = dir.join("implicit");
    std::fs::create_dir_all(&ok).unwrap();
    std::fs::write(
        ok.join(ENTRY_FILE),
        "# loquat-plugin v2\ndef handle(ctx):\n    return 'implicit'\n",
    )
    .unwrap();
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec![text_message("implicit")]
    );
    assert!(business.info().contains("implicit"));
    assert!(!business.info().contains("mismatch"));
    let _ = std::fs::remove_dir_all(&dir);
}
