//! business 的插件加载与责任链路由测试。
//! 从 business.rs 内联迁出：R4 规则限 400 行，内联测试挤爆上限。

use std::path::Path;
use std::path::PathBuf;

use super::*;
use crate::pluginload::SIGNATURE;

fn plugin_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loquat_biz_test_{tag}"));
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

fn ctx(text: &'static str) -> MsgCtx {
    MsgCtx {
        text,
        chat: Chat::Private { id: "1".to_string() },
        sender_id: "1",
        self_id: "100",
        message_id: "",
        mentions_bot: false,
        images: &[],
        segments: &[],
        platform: "napcat",
    }
}

#[test]
fn ctx_fields_reach_plugin() {
    let dir = plugin_dir("ctx");
    write_plugin(
        &dir,
        "p",
        "def handle(ctx):\n    return ['|'.join(str(ctx[k]) for k in ('text', 'message_type', 'chat_id', 'user_id', 'self_id', 'message_id'))]\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    let c = MsgCtx {
        text: "hi",
        chat: Chat::Group { id: "42".to_string() },
        sender_id: "7",
        self_id: "100",
        message_id: "12345",
        mentions_bot: true,
        images: &[],
        segments: &[],
        platform: "napcat",
    };
    assert_eq!(
        business.handle(&c).unwrap(),
        vec!["hi|group|42|7|100|12345".to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn business_hot_reload() {
    let dir = plugin_dir("hot_reload");
    write_plugin(&dir, "p", "def handle(ctx):\n    return ['v1']\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.handle(&ctx("k")).unwrap(), vec!["v1".to_string()]);

    // mtime 有粒度，等一会让改动可观测
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_plugin(&dir, "p", "def handle(ctx):\n    return ['v2']\n");
    assert_eq!(business.handle(&ctx("k")).unwrap(), vec!["v2".to_string()]);
    assert_eq!(business.reloads.load(Ordering::Relaxed), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn chain_routes_by_priority_and_survives_removal() {
    let dir = plugin_dir("chain");
    write_plugin(
        &dir,
        "zz_low",
        "PRIORITY = 999\ndef handle(ctx):\n    return ['兜底']\n",
    );
    write_plugin(
        &dir,
        "aa_high",
        "PRIORITY = 1\ndef handle(ctx):\n    return ['高优先级']\n",
    );
    // 空回复的插件不接管，链继续
    write_plugin(&dir, "mm_none", "def handle(ctx):\n    return []\n");

    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec!["高优先级".to_string()]
    );

    // 删掉高优先级插件后，同步应落到兜底
    std::fs::remove_dir_all(dir.join("aa_high")).unwrap();
    assert_eq!(
        business.handle(&ctx("x")).unwrap(),
        vec!["兜底".to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plugin_can_be_disabled() {
    let dir = plugin_dir("disable");
    write_plugin(&dir, "p", "def handle(ctx):\n    return ['ok']\n");
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["ok".to_string()]);

    assert!(business.set_enabled("p", false));
    assert!(business.handle(&ctx("x")).unwrap().is_empty());
    assert!(business.set_enabled("p", true));
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["ok".to_string()]);
    assert!(!business.set_enabled("不存在", true));
    let _ = std::fs::remove_dir_all(&dir);
}

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
fn plugin_imports_local_module_and_hot_reloads_it() {
    let dir = plugin_dir("localmod");
    write_file(&dir.join("p"), "helper.py", "VALUE = 41\n");
    write_plugin(
        &dir,
        "p",
        "from helper import VALUE\ndef handle(ctx):\n    return [str(VALUE + 1)]\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["42".to_string()]);

    // 改本地模块也要触发热重载
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_file(&dir.join("p"), "helper.py", "VALUE = 10\n");
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["11".to_string()]);
    assert!(business.reloads.load(Ordering::Relaxed) >= 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn local_modules_isolated_between_plugins() {
    let dir = plugin_dir("isolation");
    // 两个插件各带同名 helper.py，值不同——import 不得串台
    write_file(&dir.join("aa"), "helper.py", "VALUE = 'A'\n");
    write_plugin(
        &dir,
        "aa",
        "from helper import VALUE\ndef handle(ctx):\n    return [VALUE]\n",
    );
    write_file(&dir.join("bb"), "helper.py", "VALUE = 'B'\n");
    write_plugin(
        &dir,
        "bb",
        "from helper import VALUE\ndef handle(ctx):\n    return [VALUE]\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["A".to_string()]);
    assert!(business.set_enabled("aa", false));
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["B".to_string()]);
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
fn unsigned_dirs_are_ignored() {
    let dir = plugin_dir("unsigned");
    write_file(
        &dir.join("p"),
        ENTRY_FILE,
        "def handle(ctx):\n    return ['ok']\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert!(business.handle(&ctx("x")).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lib_change_forces_plugin_reload() {
    let dir = plugin_dir("libreload");
    write_file(&dir.join("_lib"), "config.py", "TAG = 'v1'\n");
    write_plugin(
        &dir,
        "p",
        "import loquat_cfglib\ndef handle(ctx):\n    return [loquat_cfglib.TAG]\n",
    );
    let business = Business::load(dir.to_str().unwrap()).unwrap();
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["v1".to_string()]);

    // 改共享库：所有插件强制重载，重新绑定 import
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_file(&dir.join("_lib"), "config.py", "TAG = 'v2'\n");
    assert_eq!(business.handle(&ctx("x")).unwrap(), vec!["v2".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}
