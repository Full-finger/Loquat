//! loquat_webtools（内核联网库）的测试：DDG 结果页解析与 HTML 抽正文。
//! fixture 内联，不打真实网络；网络路径（真实搜索/抓页）由真机人工复测覆盖。
//! helpers 与 tests.rs / tests_agent.rs 各持一份，保持模块解耦。

use std::path::Path;
use std::path::PathBuf;

use super::*;
use crate::pluginload::SIGNATURE;

// 布置临时插件目录：只拷 log（webtools 依赖它）与 webtools 两个 _lib 库。
fn web_test_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loquat_biz_web_test_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["log.py", "webtools.py"] {
        let lib_dir = dir.join("_lib");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(
            lib_dir.join(f),
            std::fs::read_to_string(Path::new("extensions/_lib").join(f)).unwrap(),
        )
        .unwrap();
    }
    dir
}

fn write_plugin(dir: &Path, folder: &str, code: &str) {
    let d = dir.join(folder);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(ENTRY_FILE), format!("{SIGNATURE}\n{code}")).unwrap();
}

// spy 插件：调用 webtools 的纯解析函数，结果 JSON 化回传（不触网）。
const SPY_SRC: &str = r#"
import json

import loquat_webtools

DDG_HTML = (
    '<div class="result"><a class="result__a" '
    'href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fa.example%2Fx&amp;rut=abc">标题 甲</a>'
    '<a class="result__snippet" href="">摘要 <b>甲</b> 正文</a>'
    '<a class="result__a" href="https://b.example/y">标题乙</a></div>'
)

DS_JSON = json.loads('{"results": ['
    '{"title": "甲", "url": "https://a.example/1", "snippet": "短摘要",'
    ' "summary": "长摘要甲", "rerank_score": 2.0},'
    '{"title": "乙", "url": "https://b.example/2", "snippet": "只有短摘要",'
    ' "summary": "", "rerank_score": 1.0}]}')

PAGE_HTML = (
    '<html><head><title>T</title><style>.x{}</style>'
    '<script>var s = "</div>"; bad();</script></head>'
    '<body><p>第一段</p><div>第二 <b>段</b></div>'
    '<script>also_bad()</script></body></html>'
)


def handle(ctx):
    t = ctx.get("text") or ""
    if t == "search":
        rs = loquat_webtools._parse_ddg(DDG_HTML, 5)
        return [json.dumps(rs, ensure_ascii=False)]
    if t == "ds":
        rs = loquat_webtools._parse_deepseek(DS_JSON, 5)
        return [json.dumps(rs, ensure_ascii=False)]
    if t == "dsurl":
        a = loquat_webtools._tools_url("https://api.deepseek.com")
        b = loquat_webtools._tools_url("https://api.deepseek.com/v1/")
        return [f"{a}|{b}"]
    if t == "page":
        return [loquat_webtools._html_to_text(PAGE_HTML)]
    return []
"#;

#[test]
fn webtools_parses_ddg_and_extracts_text() {
    let dir = web_test_dir("parse");
    write_plugin(&dir, "spy", SPY_SRC);
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let ctx = |t: &'static str| MsgCtx {
        text: t,
        message_type: "private",
        chat_id: 901,
        user_id: 1,
        self_id: 100,
        message_id: 0,
        is_at_bot: false,
        images: &[],
    };

    // DDG 解析：uddg 跳转还原成真实链接，标题/摘要按文档顺序配对
    let replies = business.handle(&ctx("search")).unwrap();
    assert_eq!(replies.len(), 1);
    let out = &replies[0];
    assert!(out.contains("https://a.example/x"), "uddg 应还原: {out}");
    assert!(out.contains("标题 甲"), "实际: {out}");
    assert!(out.contains("摘要 甲 正文"), "嵌套标签应拼进摘要: {out}");
    assert!(out.contains("https://b.example/y"), "直链应原样保留: {out}");
    assert!(!out.contains("uddg"), "跳转包装不应直出: {out}");

    // DeepSeek 解析：摘要优先 summary（给模型的长摘要），缺失退化 snippet
    let replies = business.handle(&ctx("ds")).unwrap();
    assert_eq!(replies.len(), 1);
    let out = &replies[0];
    assert!(out.contains("长摘要甲"), "summary 应优先: {out}");
    assert!(out.contains("只有短摘要"), "无 summary 应退化 snippet: {out}");
    assert!(out.contains("https://a.example/1"), "实际: {out}");

    // api_base 规整：带不带 /v1 都收敛到 …/v1
    let replies = business.handle(&ctx("dsurl")).unwrap();
    assert_eq!(
        replies[0],
        "https://api.deepseek.com/v1|https://api.deepseek.com/v1"
    );

    // HTML 抽正文：script/style 剔除，块级断行，空白与空行归并
    let replies = business.handle(&ctx("page")).unwrap();
    assert_eq!(replies.len(), 1);
    let out = &replies[0];
    assert!(out.contains("第一段"), "实际: {out}");
    assert!(out.contains("第二 段"), "内联标签不应断词: {out}");
    assert!(!out.contains("bad()"), "script 应剔除: {out}");
    assert!(!out.contains(".x{}"), "style 应剔除: {out}");
    assert!(!out.contains("\n\n"), "空行应合并: {out}");
    let _ = std::fs::remove_dir_all(&dir);
}
