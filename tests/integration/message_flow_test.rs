//! Integration tests for message processing flow

use loquat::events::{MessageEvent, EventEnum, Package, EventMetadata, Block, BlockType};
use loquat::events::traits::Event;

#[tokio::test]
async fn test_complete_message_flow() {
    // 1. 创建测试引擎
    let mut engine = common::create_test_engine().await;
    
    // 2. 启动引擎
    let start_result = engine.start().await;
    assert!(start_result.is_ok(), "Engine should start successfully");
    
    // 3. 验证引擎运行状态
    assert!(engine.is_running(), "Engine should be running");
    
    // 4. 创建测试消息事件
    let message_event = MessageEvent::Text {
        text: "Hello, Loquat!".to_string(),
        metadata: EventMetadata::default(),
    };
    
    // 5. 包装为 EventEnum
    let event_enum = EventEnum::Message(message_event);
    
    // 6. 创建 Block
    let block = Block::new(BlockType::Default)
        .with_events(vec![event_enum]);
    
    // 7. 创建 Package
    let package = Package::new()
        .with_package_id("test_package".to_string())
        .with_block(block);
    
    // 8. 处理 Package
    let result = engine.process(package).await;
    
    // 9. 验证处理结果
    assert!(result.is_ok(), "Package processing should succeed");
    
    let processed_package = result.unwrap();
    assert_eq!(processed_package.package_id, "test_package");
    assert!(!processed_package.blocks.is_empty(), "Package should have blocks");
    
    // 10. 停止引擎
    let stop_result = engine.stop().await;
    assert!(stop_result.is_ok(), "Engine should stop successfully");
}

#[tokio::test]
async fn test_nine_stage_workflow() {
    // 测试九阶段工作流完整性
    let engine = common::create_test_engine().await;
    
    // 启动引擎
    engine.start().await.expect("Engine should start");
    
    // 获取或创建通道
    use loquat::channels::ChannelType;
    let channel_type = ChannelType::group("test_group");
    
    // 通过引擎获取通道
    let channel_result = engine.get_channel(&channel_type).await;
    
    // 验证通道创建
    assert!(channel_result.is_ok(), "Should be able to get channel");
    
    if let Some(channel) = channel_result.unwrap() {
        // 验证流 ID 格式
        assert!(channel.stream_id().starts_with("stream_"), 
                "Stream ID should start with 'stream_'");
        assert_eq!(channel.channel_id(), "group:test_group");
    }
}

#[tokio::test]
async fn test_different_message_types() {
    // 测试不同类型消息的处理
    let mut engine = common::create_test_engine().await;
    engine.start().await.expect("Engine should start");
    
    // 测试文本消息
    let text_event = MessageEvent::Text {
        text: "Text message".to_string(),
        metadata: EventMetadata::default(),
    };
    
    let text_enum = EventEnum::Message(text_event);
    let text_block = Block::new(BlockType::Default)
        .with_events(vec![text_enum]);
    let text_package = Package::new()
        .with_package_id("text_test".to_string())
        .with_block(text_block);
    
    let text_result = engine.process(text_package).await;
    assert!(text_result.is_ok(), "Text message should be processed");
    
    // 测试图片消息
    let image_event = MessageEvent::Image {
        url: "https://example.com/image.jpg".to_string(),
        caption: Some("Test image".to_string()),
        metadata: EventMetadata::default(),
    };
    
    let image_enum = EventEnum::Message(image_event);
    let image_block = Block::new(BlockType::Default)
        .with_events(vec![image_enum]);
    let image_package = Package::new()
        .with_package_id("image_test".to_string())
        .with_block(image_block);
    
    let image_result = engine.process(image_package).await;
    assert!(image_result.is_ok(), "Image message should be processed");
}

#[tokio::test]
async fn test_multiple_blocks_in_package() {
    // 测试一个 Package 包含多个 Block
    let mut engine = common::create_test_engine().await;
    engine.start().await.expect("Engine should start");
    
    // 创建多个事件
    let event1 = MessageEvent::Text {
        text: "First message".to_string(),
        metadata: EventMetadata::default(),
    };
    let event2 = MessageEvent::Text {
        text: "Second message".to_string(),
        metadata: EventMetadata::default(),
    };
    
    // 创建多个 Block
    let block1 = Block::new(BlockType::Default)
        .with_events(vec![EventEnum::Message(event1)]);
    let block2 = Block::new(BlockType::Default)
        .with_events(vec![EventEnum::Message(event2)]);
    
    // 创建包含多个 Block 的 Package
    let package = Package::new()
        .with_package_id("multi_block_test".to_string())
        .with_blocks(vec![block1, block2]);
    
    // 处理并验证
    let result = engine.process(package).await;
    assert!(result.is_ok());
    
    let processed = result.unwrap();
    assert_eq!(processed.blocks.len(), 2, "Package should have 2 blocks");
}

#[tokio::test]
async fn test_engine_state_transitions() {
    // 测试引擎状态转换
    let mut engine = common::create_test_engine().await;
    
    // 初始状态应该是停止的
    assert!(!engine.is_running(), "Engine should not be running initially");
    
    // 启动引擎
    engine.start().await.expect("Engine should start");
    assert!(engine.is_running(), "Engine should be running after start");
    
    // 检查状态
    let state = engine.state().await;
    assert!(state.status.is_running(), "State should show running");
    
    // 停止引擎
    engine.stop().await.expect("Engine should stop");
    assert!(!engine.is_running(), "Engine should not be running after stop");
}
