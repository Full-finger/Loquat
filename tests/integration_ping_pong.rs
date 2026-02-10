//! Integration test for ping-pong flow (v2.0)
//!
//! This test demonstrates the complete flow:
//! 1. Create a package with "/ping" text
//! 2. Process through Input pool (CommandParser)
//! 3. Process through Process pool (optional ConversionWorker)
//! 4. Process through Output pool (PingPongWorker)
//! 5. Verify output is "pong"

use loquat::events::{Package, TargetSite};
use loquat::events::payloads::TextPayload;
use loquat::pools::StandardPool;
use loquat::pools::PoolType;
use loquat::workers::{CommandParser, PingPongWorker, WorkerRegistration, MatchingRule};
use loquat::logging::StructuredLogger;
use loquat::logging::formatters::JsonFormatter;
use loquat::logging::writers::ConsoleWriter;

fn create_logger() -> loquat::logging::Arc<dyn loquat::logging::Logger> {
    let formatter = loquat::Arc::new(JsonFormatter::new());
    let writer = loquat::Arc::new(ConsoleWriter::new());
    loquat::Arc::new(StructuredLogger::new(formatter, writer))
}

#[tokio::test]
async fn test_complete_ping_pong_flow() {
    // Create logger
    let logger = create_logger();
    
    // 1. Create Input pool with CommandParser
    let mut input_pool = StandardPool::new(PoolType::Input, logger.clone());
    let command_parser = Box::new(CommandParser::new());
    let cmd_reg = WorkerRegistration::new(command_parser, MatchingRule::All, 0);
    input_pool.register(cmd_reg).unwrap();
    
    // 2. Create Output pool with PingPongWorker
    let mut output_pool = StandardPool::new(PoolType::Output, logger.clone());
    let ping_pong = Box::new(PingPongWorker::new());
    let pong_reg = WorkerRegistration::new(ping_pong, MatchingRule::All, 0);
    output_pool.register(pong_reg).unwrap();
    
    // 3. Create package with "/ping" text
    let package = Package::new()
        .with_payload(TextPayload::new("/ping"))
        .with_target_site(TargetSite::tag("text"));
    
    println!("Input package:");
    println!("  Payload: {:?}", package.payload);
    println!("  Tags: {:?}", package.target_sites);
    println!("  Trace: {:?}", package.trace);
    
    // 4. Process through Input pool (CommandParser)
    let mut packages = input_pool.process_batch(vec![package]).await;
    
    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    
    println!("\nAfter Input pool:");
    println!("  Payload: {:?}", pkg.payload);
    println!("  Tags: {:?}", pkg.target_sites);
    println!("  Trace: {:?}", pkg.trace);
    
    // Verify CommandParser added tags
    assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
        loquat::events::SiteType::Tag(tag) if tag == "command")));
    assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
        loquat::events::SiteType::Tag(tag) if tag == "command:ping")));
    
    // 5. Process through Output pool (PingPongWorker)
    let packages = output_pool.process_batch(packages).await;
    
    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    
    println!("\nAfter Output pool:");
    println!("  Payload: {:?}", pkg.payload);
    println!("  Tags: {:?}", pkg.target_sites);
    println!("  Trace: {:?}", pkg.trace);
    
    // 6. Verify PingPongWorker changed payload to "pong"
    if let Some(payload) = pkg.get_payload::<TextPayload>() {
        assert_eq!(payload.content, "pong");
        println!("\n✅ SUCCESS: Ping-pong flow completed!");
        println!("   Input: /ping");
        println!("   Output: {}", payload.content);
    } else {
        panic!("Expected TextPayload");
    }
    
    // Verify response tag was added
    assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
        loquat::events::SiteType::Tag(tag) if tag == "response")));
    
    // Verify trace includes both workers
    assert!(pkg.trace.contains(&"command_parser".to_string()));
    assert!(pkg.trace.contains(&"ping_pong".to_string()));
}

#[tokio::test]
async fn test_ping_pong_with_custom_response() {
    let logger = create_logger();
    
    let mut input_pool = StandardPool::new(PoolType::Input, logger.clone());
    let command_parser = Box::new(CommandParser::new());
    let cmd_reg = WorkerRegistration::new(command_parser, MatchingRule::All, 0);
    input_pool.register(cmd_reg).unwrap();
    
    let mut output_pool = StandardPool::new(PoolType::Output, logger.clone());
    let ping_pong = Box::new(PingPongWorker::with_response("PONG!"));
    let pong_reg = WorkerRegistration::new(ping_pong, MatchingRule::All, 0);
    output_pool.register(pong_reg).unwrap();
    
    let package = Package::new()
        .with_payload(TextPayload::new("/ping"))
        .with_target_site(TargetSite::tag("text"));
    
    let packages = input_pool.process_batch(vec![package]).await;
    let packages = output_pool.process_batch(packages).await;
    
    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    
    if let Some(payload) = pkg.get_payload::<TextPayload>() {
        assert_eq!(payload.content, "PONG!");
    } else {
        panic!("Expected TextPayload");
    }
}

#[tokio::test]
async fn test_non_command_message() {
    let logger = create_logger();
    
    let mut input_pool = StandardPool::new(PoolType::Input, logger.clone());
    let command_parser = Box::new(CommandParser::new());
    let cmd_reg = WorkerRegistration::new(command_parser, MatchingRule::All, 0);
    input_pool.register(cmd_reg).unwrap();
    
    let mut output_pool = StandardPool::new(PoolType::Output, logger.clone());
    let ping_pong = Box::new(PingPongWorker::new());
    let pong_reg = WorkerRegistration::new(ping_pong, MatchingRule::All, 0);
    output_pool.register(pong_reg).unwrap();
    
    // Package with regular text (not a command)
    let package = Package::new()
        .with_payload(TextPayload::new("hello world"))
        .with_target_site(TargetSite::tag("text"));
    
    // Process through pools
    let packages = input_pool.process_batch(vec![package]).await;
    let packages = output_pool.process_batch(packages).await;
    
    // Should pass through unchanged
    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    
    // CommandParser should not add command tags
    assert!(!pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
        loquat::events::SiteType::Tag(tag) if tag == "command")));
    
    // PingPongWorker should not match
    if let Some(payload) = pkg.get_payload::<TextPayload>() {
        assert_eq!(payload.content, "hello world");
    } else {
        panic!("Expected TextPayload");
    }
}

#[tokio::test]
async fn test_ping_with_arguments() {
    let logger = create_logger();
    
    let mut input_pool = StandardPool::new(PoolType::Input, logger.clone());
    let command_parser = Box::new(CommandParser::new());
    let cmd_reg = WorkerRegistration::new(command_parser, MatchingRule::All, 0);
    input_pool.register(cmd_reg).unwrap();
    
    let mut output_pool = StandardPool::new(PoolType::Output, logger.clone());
    let ping_pong = Box::new(PingPongWorker::new());
    let pong_reg = WorkerRegistration::new(ping_pong, MatchingRule::All, 0);
    output_pool.register(pong_reg).unwrap();
    
    // Package with arguments: "/ping hello world"
    let package = Package::new()
        .with_payload(TextPayload::new("/ping hello world"))
        .with_target_site(TargetSite::tag("text"));
    
    let packages = input_pool.process_batch(vec![package]).await;
    let packages = output_pool.process_batch(packages).await;
    
    assert_eq!(packages.len(), 1);
    let pkg = &packages[0];
    
    // Should still respond "pong" (ignoring arguments)
    if let Some(payload) = pkg.get_payload::<TextPayload>() {
        assert_eq!(payload.content, "pong");
    } else {
        panic!("Expected TextPayload");
    }
}
