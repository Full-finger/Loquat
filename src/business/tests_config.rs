//! 插件配置面测试：config.yaml 注入/门控、install、配置热重载、secret
//! 打码与合并、cfglib 渲染。从 tests.rs 迁出：R4 规则限 400 行。

use std::path::Path;

use super::tests::{ctx, plugin_dir, write_file, write_plugin};
use super::*;

#[test]
fn config_yaml_is_injected_and_gates_enabled() {
    let dir = plugin_dir("config");
    write_plugin(
        &dir,
        "cfgtest",
        "def handle(ctx):\n    return [config.get('greeting', '缺失')]\n",
    );
    std::fs::write(
        dir.join("cfgtest").join(CONFIG_FILE),
        "enabled: true\ngreeting: 你好呀\n",
    )
    .unwrap();
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec!["你好呀".to_string()]
    );

    // enabled: false —— 路由链跳过
    std::fs::write(
        dir.join("cfgtest").join(CONFIG_FILE),
        "enabled: false\ngreeting: 你好呀\n",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    assert!(business.handle(&ctx("x")).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_generates_config_and_gates_loading() {
    let dir = plugin_dir("template");
    // 把真实的 _lib/config.py、log.py 拷进临时 extensions 目录（cargo test 工作目录 = 包根）
    std::fs::create_dir_all(dir.join("_lib")).unwrap();
    std::fs::copy(
        "extensions/_lib/config.py",
        dir.join("_lib").join("config.py"),
    )
    .unwrap();
    std::fs::copy("extensions/_lib/log.py", dir.join("_lib").join("log.py")).unwrap();
    write_plugin(
        &dir,
        "cfgtest",
        "import loquat_cfglib\nSCHEMA = [{'key': 'greeting', 'type': 'string', 'default': '你好呀', 'desc': '问候语'}]\ndef handle(ctx):\n    return [loquat_cfglib.resolve(SCHEMA, config)['greeting']]\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();

    // 未安装：不加载，消息路由不到
    assert!(business.handle(&ctx("x")).unwrap().is_empty());

    // 安装：按 SCHEMA 生成 config.yaml，保持未启用
    let msg = business.install("cfgtest").unwrap();
    assert!(msg.contains("enable cfgtest"), "提示应引导 enable: {msg}");
    let cfg_path = dir.join("cfgtest").join("config.yaml");
    assert!(cfg_path.is_file(), "config.yaml 应被生成");
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(text.contains("# 问候语"), "模板应带 SCHEMA 的 desc 注释");
    // 生成的文件必须是合法 YAML 且值取默认
    let docs = yaml_rust2::YamlLoader::load_from_str(&text).unwrap();
    assert_eq!(docs[0]["greeting"].as_str(), Some("你好呀"));
    assert_eq!(docs[0]["enabled"].as_bool(), Some(true));
    assert!(
        dir.join("cfgtest").join(".disabled").is_file(),
        "安装后应保持未启用"
    );

    // 未启用：路由不到；enable 后生效
    assert!(business.handle(&ctx("x")).unwrap().is_empty());
    assert!(business.set_enabled("cfgtest", true));
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec!["你好呀".to_string()]
    );

    // 重复安装报错
    assert!(business.install("cfgtest").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_yaml_change_reloads_and_takes_effect() {
    let dir = plugin_dir("cfg_reload");
    write_plugin(
        &dir,
        "p",
        concat!(
            "import loquat_cfglib\n",
            "\n",
            "SCHEMA = [{'key': 'rules', 'type': 'dict', 'default': {}}]\n",
            "\n",
            "def handle(ctx):\n",
            "    t = (ctx.get('text') or '').strip()\n",
            "    rules = loquat_cfglib.resolve(SCHEMA, config)['rules']\n",
            "    return [rules[t]] if t in rules else []\n",
        ),
    );
    // 插件依赖 loquat_cfglib：测试目录也要带 _lib，否则 lib_mtime 为 0
    // 会被当作"无变更"跳过共享库加载
    for f in ["log.py", "config.py"] {
        write_file(
            &dir.join("_lib"),
            f,
            &std::fs::read_to_string(Path::new("extensions/_lib").join(f)).unwrap(),
        );
    }
    write_file(
        &dir.join("p"),
        CONFIG_FILE,
        "enabled: true\nrules:\n  ping: pong\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("ping")).unwrap(),
        vec!["pong".to_string()]
    );

    // 改 config.yaml：下一条消息处理前惰性重载，新配置立即生效
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_file(
        &dir.join("p"),
        CONFIG_FILE,
        "enabled: true\nrules:\n  ping: pong2\n",
    );
    assert_eq!(
        business.handle(&ctx("ping")).unwrap(),
        vec!["pong2".to_string()]
    );
    assert_eq!(business.reloads.load(Ordering::Relaxed), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plugin_config_masks_secrets_and_merges_on_save() {
    // secret 语义：GET 打码（明文不出后端），保存 留空=保持、null=清除
    let dir = plugin_dir("secret_cfg");
    write_plugin(
        &dir,
        "p",
        "SCHEMA = [{'key': 'api_key', 'type': 'secret', 'default': ''}, \
                   {'key': 'name', 'type': 'string', 'default': 'x'}]\n\
         def handle(ctx):\n    return []\n",
    );
    write_file(
        &dir.join("p"),
        "config.yaml",
        "api_key: \"real-secret\"\nname: demo\nenabled: true\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();

    let cfg = business.plugin_config("p").unwrap();
    assert_eq!(cfg["values"]["api_key"], "");
    assert_eq!(cfg["secret_keys"][0], "api_key");
    assert_eq!(cfg["values"]["name"], "demo");

    // 留空 = 保持已有（明文仍写盘，打码只发生在接口层）
    business
        .save_plugin_config("p", serde_json::json!({ "api_key": "", "name": "v2" }))
        .unwrap();
    let on_disk = std::fs::read_to_string(dir.join("p").join("config.yaml")).unwrap();
    assert!(on_disk.contains("real-secret"), "留空应保持原值: {on_disk}");
    assert!(on_disk.contains("v2"));

    // null = 清除，清除后不再标记已配置
    business
        .save_plugin_config("p", serde_json::json!({ "api_key": null, "name": "v2" }))
        .unwrap();
    let cfg = business.plugin_config("p").unwrap();
    assert_eq!(cfg["secret_keys"].as_array().unwrap().len(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cfglib_render_handles_dict_fields() {
    // 回归：dict 字段曾被渲染成 `rules:    "a": 1`（块没换行），
    // 标准 YAML 直接拒绝（mapping values are not allowed in this context），
    // 插件下次启动解析失败即被跳过——qa 插件踩过这个坑。
    use pyo3::prelude::*;
    use pyo3::types::PyDict;

    // 参数是 extensions 目录（内部再拼 _lib/xxx.py）
    assert!(crate::sharedlib::load(Path::new("extensions")).unwrap());
    let text = Python::attach(|py| -> String {
        let locals = PyDict::new(py);
        py.run(
            &std::ffi::CString::new(
                r#"
import loquat_cfglib
schema = [
    {"key": "rules", "type": "dict", "default": {"ping": "pong"}, "desc": "关键词 -> 回复"},
    {"key": "prompt", "default": "第一行\n第二行: 带冒号"},
]
values = {"rules": {"ping": "pong", "在吗": "在"}, "prompt": "第一行\n第二行: 带冒号"}
text = "enabled: true\n\n" + loquat_cfglib.render(schema, values)
"#,
            )
            .unwrap(),
            None,
            Some(&locals),
        )
        .unwrap();
        locals
            .get_item("text")
            .unwrap()
            .unwrap()
            .extract::<String>()
            .unwrap()
    });

    // 能被标准 YAML 解析回原值，才算写对了
    let parsed = super::config_api::serde_yamlish(&text).expect("render 输出必须是合法 YAML");
    assert_eq!(parsed["rules"]["在吗"], "在", "dict 字段丢了: {text}");
    assert_eq!(parsed["rules"]["ping"], "pong");
    assert_eq!(parsed["prompt"], "第一行\n第二行: 带冒号");
}

#[test]
fn asset_upload_and_path_guard() {
    // asset：上传重名不覆盖；配置值取落盘路径时穿越/非法名一律拒绝
    let dir = plugin_dir("assets");
    write_plugin(&dir, "p", "def handle(ctx):\n    return []\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();

    let stored = business.save_plugin_asset("p", "logo.png", b"png").unwrap();
    assert_eq!(stored, "logo.png");
    let again = business.save_plugin_asset("p", "logo.png", b"png2").unwrap();
    assert_eq!(again, "logo-1.png");

    assert!(business.plugin_asset_path("p", "assets/logo.png").is_some());
    assert!(business.plugin_asset_path("p", "assets/../plugin.py").is_none());
    assert!(business.plugin_asset_path("p", "../../Cargo.toml").is_none());
    assert!(business.plugin_asset_path("../p", "assets/logo.png").is_none());
    assert!(business.save_plugin_asset("p", "../evil", b"x").is_err());
    assert!(business.save_plugin_asset("p", ".hidden", b"x").is_err());

    business.delete_plugin_asset("p", "assets/logo-1.png").unwrap();
    assert!(business.plugin_asset_path("p", "assets/logo-1.png").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
