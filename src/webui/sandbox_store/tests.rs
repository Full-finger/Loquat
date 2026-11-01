//! sandbox_store 的测试：种子会话、自动命名、顺序读写、压缩、
//! 删除联动、媒体落盘、base64 往返。

use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::store::Store;
use super::util::b64_decode;
use super::{DEFAULT_GROUP, DEFAULT_PRIVATE, PLACEHOLDER};

fn tmp_store(name: &str) -> (Store, PathBuf) {
    let dir = std::env::temp_dir().join(format!("loquat-sandbox-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    (Store::new(&dir, 5, 3), dir)
}

#[test]
fn seeds_default_sessions() {
    let (s, dir) = tmp_store("seed");
    let list = s.list();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|x| x.chat_id == DEFAULT_PRIVATE));
    assert!(list.iter().any(|x| x.chat_id == DEFAULT_GROUP));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn create_is_isolated_and_auto_names() {
    let (mut s, dir) = tmp_store("auto");
    let created = s.create("private").unwrap();
    assert!(created.chat_id.starts_with("10000."));
    assert_eq!(created.name, PLACEHOLDER);

    let segs = json!([{ "text": { "text": "帮我看下今天的排班" } }]);
    let after = s.record_in(&created.chat_id, "1", &segs).unwrap();
    assert_eq!(after.name, "帮我看下今天的排班");
    assert_eq!(after.count, 1);

    // 另一个会话互不影响
    let other = s.create("private").unwrap();
    assert!(s.history(&other.chat_id, 50).is_empty());
    assert_eq!(s.history(&created.chat_id, 50).len(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn appends_and_reads_back_in_order() {
    let (mut s, dir) = tmp_store("read");
    let c = s.create("group").unwrap();
    s.record_in(&c.chat_id, "1", &json!([{ "text": { "text": "第一句" } }]))
        .unwrap();
    s.record_out(&c.chat_id, "group", "收到").unwrap();
    let h = s.history(&c.chat_id, 10);
    assert_eq!(h.len(), 2);
    assert_eq!(h[0]["dir"], "in");
    assert_eq!(h[1]["dir"], "out");
    assert_eq!(h[1]["text"], "收到");
    assert!(h[0]["ts"].as_u64().unwrap() > 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn trims_to_keep_when_growing() {
    let (mut s, dir) = tmp_store("trim");
    let c = s.create("private").unwrap();
    // keep=5，写 11 条会触发压缩（count > 10），只留最近 5 条
    for i in 0..11 {
        s.record_in(
            &c.chat_id,
            &i.to_string(),
            &json!([{ "text": { "text": format!("m{i}") } }]),
        )
        .unwrap();
    }
    let h = s.history(&c.chat_id, 100);
    assert_eq!(h.len(), 5, "压缩后应只剩 keep 条");
    assert_eq!(h[0]["segments"][0]["text"]["text"], "m6");
    assert_eq!(h.last().unwrap()["segments"][0]["text"]["text"], "m10");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_and_remove_clean_both_files() {
    let (mut s, dir) = tmp_store("rename");
    let c = s.create("private").unwrap();
    s.record_in(&c.chat_id, "1", &json!([{ "text": { "text": "x" } }]))
        .unwrap();
    // 造一个机器人上下文文件，删除时应一并清掉
    let agent = dir.join("sessions");
    fs::create_dir_all(&agent).unwrap();
    fs::write(agent.join(format!("private_{}.json", c.chat_id)), "{}").unwrap();

    let renamed = s.rename(&c.chat_id, "  改名   测试 ").unwrap();
    assert_eq!(renamed.name, "改名 测试");

    s.remove(&c.chat_id).unwrap();
    assert!(s.get(&c.chat_id).is_none());
    assert!(!dir
        .join("sandbox")
        .join(format!("{}.jsonl", c.chat_id))
        .exists());
    assert!(!agent.join(format!("private_{}.json", c.chat_id)).exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn media_data_url_becomes_reference() {
    let (mut s, dir) = tmp_store("media");
    let c = s.create("private").unwrap();
    // 1x1 png
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let segs = json!([{ "image": { "url": format!("data:image/png;base64,{png}"), "filename": "a.png" } }]);
    s.record_in(&c.chat_id, "1", &segs).unwrap();
    let h = s.history(&c.chat_id, 10);
    let url = h[0]["segments"][0]["image"]["url"].as_str().unwrap();
    assert!(url.starts_with("/api/sandbox/media/"), "url={url}");
    let name = url.rsplit('/').next().unwrap();
    assert!(dir.join("sandbox").join("media").join(name).is_file());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn base64_roundtrip() {
    assert_eq!(b64_decode("aGVsbG8=").unwrap(), b"hello");
    assert_eq!(b64_decode("").unwrap(), b"");
    assert!(b64_decode("!!!").is_none());
}
