//! Integration tests for configuration loading and reloading

use loquat::config::LoquatConfig;
use std::path::PathBuf;

#[tokio::test]
async fn test_default_config_loading() {
    // 测试加载默认配置
    let config = LoquatConfig::from_environment("config", "default").unwrap();
    
    // 验证默认值
    assert_eq!(config.general.environment, "dev", "Default environment should be dev");
    assert_eq!(config.logging.level, "Info", "Default log level should be Info");
    assert_eq!(config.logging.format, "text", "Default format should be text");
    assert_eq!(config.logging.output, "console", "Default output should be console");
    assert_eq!(config.plugins.enabled, true, "Plugins should be enabled by default");
    assert_eq!(config.adapters.enabled, true, "Adapters should be enabled by default");
}

#[tokio::test]
async fn test_environment_config_override() {
    // 测试环境配置覆盖
    let config = LoquatConfig::from_environment("config", "dev").unwrap();
    
    // 验证开发环境配置
    assert_eq!(config.general.environment, "dev");
    assert_eq!(config.plugins.enable_hot_reload, true, "Dev should enable plugin hot reload");
    assert_eq!(config.adapters.enable_hot_reload, true, "Dev should enable adapter hot reload");
    assert_eq!(config.web.enabled, true, "Dev should enable web service");
}

#[tokio::test]
async fn test_production_config() {
    // 测试生产环境配置
    let config = LoquatConfig::from_environment("config", "prod").unwrap();
    
    // 验证生产环境配置
    assert_eq!(config.general.environment, "prod");
    assert_eq!(config.logging.level, "Warn", "Prod should use Warn level");
    assert_eq!(config.logging.format, "json", "Prod should use JSON format");
    assert_eq!(config.logging.output, "combined", "Prod should use combined output");
    assert_eq!(config.plugins.enable_hot_reload, false, "Prod should disable hot reload");
    assert_eq!(config.adapters.enable_hot_reload, false, "Prod should disable hot reload");
}

#[tokio::test]
async fn test_test_environment_config() {
    // 测试测试环境配置
    let config = LoquatConfig::from_environment("config", "test").unwrap();
    
    // 验证测试环境配置
    assert_eq!(config.general.environment, "test");
    // 测试环境可能有不同的配置，这里只验证基本结构
    assert!(!config.general.name.is_empty());
}

#[tokio::test]
async fn test_config_validation() {
    // 测试无效配置文件的处理
    let result = LoquatConfig::from_environment("config", "non_existent");
    assert!(result.is_err(), "Loading non-existent config should fail");
    
    if let Err(e) = result {
        assert!(e.to_string().contains("Failed to load configuration") || 
                e.to_string().contains("not found"),
                "Error should indicate configuration loading failure");
    }
}

#[tokio::test]
async fn test_config_file_not_found() {
    // 测试配置文件不存在的情况
    let result = LoquatConfig::from_file("config/non_existent.toml");
    assert!(result.is_err(), "Non-existent config file should return error");
}

#[tokio::test]
async fn test_config_custom_file() {
    // 测试从自定义路径加载配置
    let temp_dir = common::create_test_dir();
    
    let custom_config = r#"
[general]
environment = "custom"
name = "Custom Test Config"

[logging]
level = "Debug"
format = "json"
output = "file"
file_path = "./logs/custom.log"

[plugins]
enabled = false

[adapters]
enabled = false

[web]
enabled = false
"#;
    
    let config_path = common::create_test_config_file(temp_dir.path(), custom_config);
    
    // 加载自定义配置
    let config = LoquatConfig::from_file(&config_path);
    
    assert!(config.is_ok(), "Custom config should load successfully");
    
    let config = config.unwrap();
    assert_eq!(config.general.environment, "custom");
    assert_eq!(config.general.name, "Custom Test Config");
    assert_eq!(config.logging.level, "Debug");
    assert_eq!(config.logging.format, "json");
    assert_eq!(config.plugins.enabled, false);
    assert_eq!(config.adapters.enabled, false);
}

#[tokio::test]
async fn test_config_whitelist_blacklist() {
    // 测试白名单和黑名单配置
    let temp_dir = common::create_test_dir();
    
    let config_content = r#"
[general]
environment = "test"
name = "Test Config"

[logging]
level = "Info"

[plugins]
enabled = true
whitelist = ["plugin1", "plugin2"]
blacklist = ["bad_plugin"]

[adapters]
enabled = true
whitelist = ["console", "echo"]
blacklist = ["dangerous_adapter"]

[web]
enabled = false
"#;
    
    let config_path = common::create_test_config_file(temp_dir.path(), config_content);
    let config = LoquatConfig::from_file(&config_path).unwrap();
    
    // 验证白名单
    assert_eq!(config.plugins.whitelist.len(), 2);
    assert!(config.plugins.whitelist.contains(&"plugin1".to_string()));
    assert!(config.plugins.whitelist.contains(&"plugin2".to_string()));
    
    assert_eq!(config.adapters.whitelist.len(), 2);
    assert!(config.adapters.whitelist.contains(&"console".to_string()));
    assert!(config.adapters.whitelist.contains(&"echo".to_string()));
    
    // 验证黑名单
    assert_eq!(config.plugins.blacklist.len(), 1);
    assert!(config.plugins.blacklist.contains(&"bad_plugin".to_string()));
    
    assert_eq!(config.adapters.blacklist.len(), 1);
    assert!(config.adapters.blacklist.contains(&"dangerous_adapter".to_string()));
}

#[tokio::test]
async fn test_config_hot_reload_intervals() {
    // 测试热重载间隔配置
    let temp_dir = common::create_test_dir();
    
    let config_content = r#"
[general]
environment = "test"
name = "Test Config"

[logging]
level = "Info"

[plugins]
enabled = true
enable_hot_reload = true
hot_reload_interval = 15

[adapters]
enabled = true
enable_hot_reload = true
hot_reload_interval = 20

[web]
enabled = false
"#;
    
    let config_path = common::create_test_config_file(temp_dir.path(), config_content);
    let config = LoquatConfig::from_file(&config_path).unwrap();
    
    assert_eq!(config.plugins.hot_reload_interval, 15);
    assert_eq!(config.adapters.hot_reload_interval, 20);
}

#[tokio::test]
async fn test_config_web_service() {
    // 测试 Web 服务配置
    let temp_dir = common::create_test_dir();
    
    let config_content = r#"
[general]
environment = "test"
name = "Test Config"

[logging]
level = "Info"

[plugins]
enabled = false

[adapters]
enabled = false

[web]
enabled = true
host = "0.0.0.0"
port = 9000
enable_cors = false
"#;
    
    let config_path = common::create_test_config_file(temp_dir.path(), config_content);
    let config = LoquatConfig::from_file(&config_path).unwrap();
    
    assert_eq!(config.web.enabled, true);
    assert_eq!(config.web.host, "0.0.0.0");
    assert_eq!(config.web.port, 9000);
    assert_eq!(config.web.enable_cors, false);
}

#[tokio::test]
async fn test_config_default_values() {
    // 测试配置的默认值
    let config = LoquatConfig::default();
    
    assert!(!config.general.name.is_empty());
    assert!(!config.general.environment.is_empty());
    assert_eq!(config.logging.level, "Info");
    assert_eq!(config.logging.format, "text");
    assert_eq!(config.logging.output, "console");
    assert_eq!(config.plugins.enabled, true);
    assert_eq!(config.adapters.enabled, true);
    assert_eq!(config.web.enabled, false);
}

#[tokio::test]
async fn test_config_paths() {
    // 测试路径配置
    let temp_dir = common::create_test_dir();
    
    let config_content = r#"
[general]
environment = "test"
name = "Test Config"

[logging]
level = "Info"
file_path = "./custom/logs/loquat.log"

[plugins]
enabled = true
plugin_dir = "./custom/plugins"

[adapters]
enabled = true
adapter_dir = "./custom/adapters"

[web]
enabled = false
"#;
    
    let config_path = common::create_test_config_file(temp_dir.path(), config_content);
    let config = LoquatConfig::from_file(&config_path).unwrap();
    
    assert_eq!(config.logging.file_path, "./custom/logs/loquat.log");
    assert_eq!(config.plugins.plugin_dir, "./custom/plugins");
    assert_eq!(config.adapters.adapter_dir, "./custom/adapters");
}
