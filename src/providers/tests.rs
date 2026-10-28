//! providers 的单元测试（纯函数：normalize/render/key 语义/迁移）。

use super::*;


use crate::business::serde_yamlish;

#[test]
fn normalize_validates_active_and_fills_defaults() {
    let body = serde_json::json!({
        "llm": { "active": "ollama", "providers": {
            "ollama": { "api_base": "http://127.0.0.1:11434/v1",
                        "model": "qwen2.5:14b", "api_key": "k1" } } },
        "search": { "active": "bocha", "providers": {
            "deepseek": {}, "duckduckgo": {}, "bocha": { "api_key": "bk" } } },
    });
    let n = normalize(&body).unwrap();
    let p = &n["llm"]["providers"]["ollama"];
    assert_eq!(p["api_base"], "http://127.0.0.1:11434/v1");
    assert_eq!(p["api_key"], "k1");
    assert_eq!(p["model"], "qwen2.5:14b");
    assert_eq!(p["temperature"], 0.8, "缺省字段应回填");
    assert_eq!(p["vision"], false);

    assert!(normalize(&serde_json::json!({
        "llm": { "active": "ghost", "providers": { "deepseek": {} } },
        "search": { "active": "deepseek", "providers": { "deepseek": {} } },
    }))
    .is_err(), "active 指向不存在的 provider 应拒绝");
    assert!(normalize(&serde_json::json!({
        "llm": { "active": "deepseek", "providers": {} },
        "search": { "active": "deepseek", "providers": { "deepseek": {} } },
    }))
    .is_err(), "空 providers 应拒绝");

    let round = serde_yamlish(&render(&n)).unwrap();
    assert_eq!(round["llm"]["active"], "ollama", "render 应可回读");
    assert_eq!(round["search"]["providers"]["bocha"]["api_key"], "bk");
}

#[test]
fn key_semantics_keep_clear_mask() {
    let old = serde_json::json!({
        "llm": { "active": "deepseek", "providers": {
            "deepseek": { "api_key": "sk-secret", "model": "m1" } } },
        "search": { "active": "deepseek", "providers": {
            "deepseek": {}, "duckduckgo": {}, "bocha": {} } },
    });
    let edit = |api_key: serde_json::Value, model: &str| {
        serde_json::json!({
            "llm": { "active": "deepseek", "providers": {
                "deepseek": { "api_key": api_key, "model": model } } },
            "search": { "active": "deepseek", "providers": {
                "deepseek": {}, "duckduckgo": {}, "bocha": {} } },
        })
    };

    // 空串 = 保持：模型可改，key 不丢
    let kept = prepare(&edit(serde_json::json!(""), "m2"), &old).unwrap();
    assert_eq!(kept["llm"]["providers"]["deepseek"]["api_key"], "sk-secret");
    assert_eq!(kept["llm"]["providers"]["deepseek"]["model"], "m2");

    // null = 显式清除
    let cleared = prepare(&edit(serde_json::Value::Null, "m2"), &old).unwrap();
    assert_eq!(cleared["llm"]["providers"]["deepseek"]["api_key"], "");

    // 视图打码：明文不出门，has_api_key 标记在
    let v = view_of(old.clone());
    let dp = &v["llm"]["providers"]["deepseek"];
    assert_eq!(dp["api_key"], "");
    assert_eq!(dp["has_api_key"], true);
    assert_eq!(resolved_of(&old).contains("sk-secret"), true);
    assert_eq!(resolved_of(&view_of(old.clone())).contains("sk-secret"), false,
        "打码视图不能再喂回解析层");
}

use std::path::PathBuf;

#[test]
fn migration_moves_credentials_and_slims_chat_config() {
    let dir = std::env::temp_dir().join("loquat_providers_migrate_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let chat: PathBuf = dir.join("config.yaml");
    std::fs::write(
        &chat,
        "enabled: true\napi_base: \"https://api.example.com/v1\"\napi_key: \"sk-test123\"\n\
         model: \"test-model\"\ntriggers: [\"A\"]\nsearch_provider: \"bocha\"\n\
         search_api_key: \"bk9\"\nmax_turns: 4\n",
    )
    .unwrap();

    let out = migrate(&chat);

    // 凭据上移：llm 收编到 deepseek 项，search 激活 bocha 且 key 跟随
    let llm = &out["llm"]["providers"]["deepseek"];
    assert_eq!(llm["api_key"], "sk-test123");
    assert_eq!(llm["model"], "test-model");
    assert_eq!(out["search"]["active"], "bocha");
    assert_eq!(out["search"]["providers"]["bocha"]["api_key"], "bk9");

    // chat 配置瘦身：凭据移除、业务字段保留、enabled 保留
    let slim = std::fs::read_to_string(&chat).unwrap();
    assert!(!slim.contains("sk-test123"), "key 不应驻留插件配置");
    assert!(!slim.contains("bk9"));
    let parsed = serde_yamlish(&slim).unwrap();
    assert_eq!(parsed["enabled"], true);
    assert_eq!(parsed["max_turns"], 4);
    assert_eq!(parsed["triggers"], serde_json::json!(["A"]));
    let _ = std::fs::remove_dir_all(&dir);
}