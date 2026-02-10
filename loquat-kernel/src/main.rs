//! Loquat Kernel - 主程序入口
//!
//! 微内核负责管理Engine生命周期、进程监控、资源分配

use loquat_kernel::{Kernel, KernelConfig};
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber;
use anyhow::Result;

/// Loquat Kernel - 微内核进程管理器
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "config/kernel.toml")]
    config: String,
    
    /// gRPC 服务器地址
    #[arg(long, default_value = "0.0.0.0:50051")]
    grpc_addr: String,
    
    /// HTTP 服务器地址
    #[arg(long, default_value = "0.0.0.0:8080")]
    http_addr: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("Starting Loquat Kernel v{}", env!("CARGO_PKG_VERSION"));
    info!("Config file: {}", args.config);
    info!("gRPC address: {}", args.grpc_addr);
    info!("HTTP address: {}", args.http_addr);
    
    // 加载配置
    let config = KernelConfig::from_file(&args.config)?;
    
    // 创建Kernel实例
    let kernel = Arc::new(Kernel::new(config.clone()));
    
    // 启动Kernel
    kernel.start().await?;
    
    // 启动gRPC服务器
    let grpc_server = Arc::new(kernel.start_grpc());
    let grpc_server_clone = grpc_server.clone();
    let grpc_handle = tokio::spawn(async move {
        let _ = grpc_server_clone.run().await;
    });
    info!("gRPC server started on {}", args.grpc_addr);
    
    // 启动HTTP服务器
    let http_server = Arc::new(kernel.start_http());
    let http_server_clone = http_server.clone();
    let http_handle = tokio::spawn(async move {
        let _ = http_server_clone.run().await;
    });
    info!("HTTP server started on {}", args.http_addr);
    
    // 等待关闭信号
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = grpc_handle => {
            info!("gRPC server shutdown");
        }
        _ = http_handle => {
            info!("HTTP server shutdown");
        }
    }
    
    // 优雅关闭
    info!("Shutting down Loquat Kernel...");
    
    // 停止HTTP服务器
    http_server.stop().await;
    
    // 停止gRPC服务器
    grpc_server.stop().await;
    
    // 停止Kernel
    if let Err(e) = kernel.stop().await {
        error!("Failed to stop Kernel: {}", e);
    }
    
    info!("Loquat Kernel stopped");
    
    Ok(())
}
