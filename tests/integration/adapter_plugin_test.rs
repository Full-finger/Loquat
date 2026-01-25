//! Integration tests for adapter and plugin interaction

use loquat::adapters::{AdapterManager, ConsoleAdapterFactory, EchoAdapterFactory, MockTestFactory};
use loquat::adapters::AdapterManagerConfig;
use loquat::plugins::PluginManager;
use loquat::plugins::PluginManagerConfig;
use loquat::errors::Result;

#[tokio::test]
async fn test_adapter_lifecycle() {
    // 1. 创建适配器管理器
    let logger = common::create_test_logger();
    let config = AdapterManagerConfig {
        adapter_dir: "adapters".to_string(),
        auto_load: false,
        enable_hot_reload: false,
        hot_reload_interval: 30,
        whitelist: vec![],
        blacklist: vec![],
        enabled: true,
    };
    
    let mut manager = AdapterManager::new(config, logger);
    
    // 2. 注册内置工厂
    manager.register_factory(Box::new(ConsoleAdapterFactory)).unwrap();
    manager.register_factory(Box::new(EchoAdapterFactory)).unwrap();
    manager.register_factory(Box::new(MockTestFactory)).unwrap();
    
    // 3. 创建 Console 适配器
    let adapter_result = manager.create_adapter("console", "test-console-001").await;
    assert!(adapter_result.is_ok(), "Console adapter creation should succeed");
    
    // 4. 验证适配器已创建
    let adapters = manager.list_adapter_infos().await;
    assert!(!adapters.is_empty(), "Should have at least one adapter");
    assert!(adapters.iter().any(|a| a.adapter_id == "test-console-001"), 
            "Console adapter should be in the list");
    
    // 5. 启动适配器
    let start_result = manager.start_adapter("test-console-001").await;
    assert!(start_result.is_ok(), "Console adapter should start");
    
    // 6. 验证适配器状态
    let adapter_info = manager.get_adapter_info("test-console-001").await;
    assert!(adapter_info.is_some(), "Should be able to get adapter info");
    
    // 7. 停止适配器
    let stop_result = manager.stop_adapter("test-console-001").await;
    assert!(stop_result.is_ok(), "Console adapter should stop");
    
    // 8. 卸载适配器
    let unload_result = manager.unload_adapter("test-console-001").await;
    assert!(unload_result.is_ok(), "Console adapter should unload");
    
    // 9. 验证适配器已移除
    let adapters_after = manager.list_adapter_infos().await;
    assert!(!adapters_after.iter().any(|a| a.adapter_id == "test-console-001"), 
            "Console adapter should be removed");
}

#[tokio::test]
async fn test_multiple_adapters() {
    // 测试管理多个适配器
    let logger = common::create_test_logger();
    let config = AdapterManagerConfig {
        adapter_dir: "adapters".to_string(),
        auto_load: false,
        enable_hot_reload: false,
        hot_reload_interval: 30,
        whitelist: vec![],
        blacklist: vec![],
        enabled: true,
    };
    
    let mut manager = AdapterManager::new(config, logger);
    
    // 注册所有内置适配器工厂
    manager.register_factory(Box::new(ConsoleAdapterFactory)).unwrap();
    manager.register_factory(Box::new(EchoAdapterFactory)).unwrap();
    manager.register_factory(Box::new(MockTestFactory)).unwrap();
    
    // 创建多个适配器
    let console_result = manager.create_adapter("console", "console-1").await;
    assert!(console_result.is_ok(), "Console adapter creation should succeed");
    
    let echo_result = manager.create_adapter("echo", "echo-1").await;
    assert!(echo_result.is_ok(), "Echo adapter creation should succeed");
    
    let mock_result = manager.create_adapter("mock_test", "mock-1").await;
    assert!(mock_result.is_ok(), "MockTest adapter creation should succeed");
    
    // 验证所有适配器都已创建
    let adapters = manager.list_adapter_infos().await;
    assert_eq!(adapters.len(), 3, "Should have 3 adapters");
    
    // 启动所有适配器
    let start_all_result = manager.start_all_adapters().await;
    assert!(!start_all_result.is_empty(), "Should have start results");
    
    let started_count = start_all_result.iter().filter(|r| r.success).count();
    assert!(started_count > 0, "At least one adapter should have started");
    
    // 停止所有适配器
    let stop_all_result = manager.stop_all_adapters().await;
    assert!(!stop_all_result.is_empty(), "Should have stop results");
}

#[tokio::test]
async fn test_plugin_lifecycle() {
    // 1. 创建插件管理器
    let config = PluginManagerConfig {
        plugin_dir: "plugins".to_string(),
        enabled: true,
        auto_load: false,
        enable_hot_reload: false,
        hot_reload_interval: 30,
        whitelist: vec![],
        blacklist: vec![],
    };
    
    let mut manager = PluginManager::new(config);
    
    // 2. 测试插件加载（使用内置测试插件 "12"）
    let load_result = manager.load_plugin("plugins/12").await;
    
    // 注意：这个测试依赖于 plugins/12 目录存在
    // 如果插件不存在，我们跳过这个测试
    if load_result.is_ok() {
        let result = load_result.unwrap();
        assert!(result.success, "Plugin should load successfully");
        
        // 3. 验证插件已加载
        let plugins = manager.list_plugin_infos();
        assert!(!plugins.is_empty(), "Should have loaded plugins");
        
        // 4. 获取插件信息
        let plugin_info = manager.get_plugin_info("12");
        assert!(plugin_info.is_some(), "Should be able to get plugin info");
        
        // 5. 重载插件
        let reload_result = manager.reload_plugin("12").await;
        assert!(reload_result.is_ok(), "Plugin should reload");
        
        // 6. 卸载插件
        let unload_result = manager.unload_plugin("12").await;
        assert!(unload_result.is_ok(), "Plugin should unload");
        
        // 7. 验证插件已移除
        let plugins_after = manager.list_plugin_infos();
        assert!(plugins_after.is_empty(), "All plugins should be unloaded");
    } else {
        // 插件目录不存在，跳过测试
        println!("Skipping plugin test - plugin directory not found");
    }
}

#[tokio::test]
async fn test_adapter_auto_load() {
    // 测试适配器自动加载功能
    let logger = common::create_test_logger();
    let mut config = AdapterManagerConfig {
        adapter_dir: "adapters".to_string(),
        auto_load: true,
        enable_hot_reload: false,
        hot_reload_interval: 30,
        whitelist: vec![],
        blacklist: vec![],
        enabled: true,
    };
    
    let mut manager = AdapterManager::new(config, logger);
    
    // 注册适配器工厂
    manager.register_factory(Box::new(ConsoleAdapterFactory)).unwrap();
    manager.register_factory(Box::new(EchoAdapterFactory)).unwrap();
    manager.register_factory(Box::new(MockTestFactory)).unwrap();
    
    // 自动加载适配器
    let auto_load_results = manager.auto_load_adapters().await;
    assert!(!auto_load_results.is_empty(), "Should have auto-load results");
    
    // 验证加载结果
    let loaded_count = auto_load_results.iter().filter(|r| r.success).count();
    assert!(loaded_count > 0, "At least one adapter should auto-load");
    
    // 列出所有适配器
    let adapters = manager.list_adapter_infos().await;
    assert!(!adapters.is_empty(), "Should have loaded adapters");
}

#[tokio::test]
async fn test_adapter_statistics() {
    // 测试适配器统计信息
    let logger = common::create_test_logger();
    let config = AdapterManagerConfig {
        adapter_dir: "adapters".to_string(),
        auto_load: false,
        enable_hot_reload: false,
        hot_reload_interval: 30,
        whitelist: vec![],
        blacklist: vec![],
        enabled: true,
    };
    
    let mut manager = AdapterManager::new(config, logger);
    manager.register_factory(Box::new(EchoAdapterFactory)).unwrap();
    
    // 创建并启动适配器
    manager.create_adapter("echo", "echo-stats").await.unwrap();
    manager.start_adapter("echo-stats").await.unwrap();
    
    // 获取统计信息
    let stats = manager.get_adapter_statistics("echo-stats").await;
    assert!(stats.is_some(), "Should be able to get adapter statistics");
    
    let stats = stats.unwrap();
    assert_eq!(stats.events_received, 0, "Initially should have 0 events received");
    assert_eq!(stats.events_sent, 0, "Initially should have 0 events sent");
    assert_eq!(stats.errors, 0, "Initially should have 0 errors");
    
    // 停止并卸载适配器
    manager.stop_adapter("echo-stats").await.unwrap();
    manager.unload_adapter("echo-stats").await.unwrap();
}

#[tokio::test]
async fn test_plugin_manager_config() {
    // 测试插件管理器配置
    let config = PluginManagerConfig {
        plugin_dir: "test_plugins".to_string(),
        enabled: true,
        auto_load: false,
        enable_hot_reload: true,
        hot_reload_interval: 10,
        whitelist: vec!["plugin1".to_string(), "plugin2".to_string()],
        blacklist: vec!["bad_plugin".to_string()],
    };
    
    let manager = PluginManager::new(config);
    
    // 验证配置
    assert_eq!(manager.config().plugin_dir, "test_plugins");
    assert_eq!(manager.config().auto_load, false);
    assert_eq!(manager.config().enable_hot_reload, true);
    assert_eq!(manager.config().hot_reload_interval, 10);
    assert_eq!(manager.config().whitelist.len(), 2);
    assert_eq!(manager.config().blacklist.len(), 1);
}
