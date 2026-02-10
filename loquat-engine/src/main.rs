use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::{info, error};

use loquat_engine::{Engine, EngineConfig};

/// Loquat Engine - 插件运行时
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Engine ID
    #[arg(long)]
    engine_id: Option<String>,

    /// Kernel gRPC address
    #[arg(long)]
    kernel_address: Option<String>,

    /// Port to listen on
    #[arg(long)]
    port: Option<u16>,

    /// Configuration file path
    #[arg(short, long, default_value = "config/engine.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("loquat_engine=info".parse().unwrap())
        )
        .init();

    // 解析命令行参数
    let args = Args::parse();

    // 加载配置
    let mut config = if args.config.exists() {
        EngineConfig::from_file(args.config.to_str().unwrap())?
    } else {
        info!("Config file not found, using defaults");
        EngineConfig::default()
    };

    // 应用命令行参数覆盖
    if let Some(engine_id) = args.engine_id {
        config.engine.engine_id = engine_id;
    }
    if let Some(kernel_address) = args.kernel_address {
        config.engine.kernel_address = kernel_address;
    }
    if let Some(port) = args.port {
        config.engine.port = port;
    }

    info!("Starting Loquat Engine {}", config.engine.engine_id);
    info!("Kernel address: {}", config.engine.kernel_address);
    info!("Port: {}", config.engine.port);

    // 创建Engine实例
    let config = Arc::new(config);
    let engine = Engine::new(config).await?;

    // 设置信号处理
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        let result = tokio::signal::ctrl_c().await;
        if result.is_ok() {
            info!("Received shutdown signal");
            if let Err(e) = engine_clone.stop().await {
                error!("Error during shutdown: {}", e);
            }
        }
    });

    // 启动Engine
    if let Err(e) = engine.start().await {
        error!("Failed to start engine: {}", e);
        return Err(e.into());
    }

    // 保持运行
    info!("Engine is running, press Ctrl+C to stop");
    
    // 等待运行状态变为false
    while engine.is_running().await {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    info!("Engine shutting down");
    
    Ok(())
}
