//! PACS服务器主程序

use clap::Parser;
use pacs_core::Result;
use pacs_dicom::{DicomServer, DicomServerConfig};
use tracing::{error, info};
use tracing_subscriber;

/// PACS服务器命令行参数
#[derive(Parser, Debug)]
#[command(name = "pacs-server")]
#[command(about = "PACS (Picture Archiving and Communication System) 服务器")]
struct Args {
    /// 服务器端口
    #[arg(short, long, default_value = "11112")]
    port: u16,

    /// AE标题 (Application Entity Title)
    #[arg(short, long, default_value = "PACS_SERVER")]
    ae_title: String,

    /// DICOM文件存储目录
    #[arg(short, long, default_value = "./data/dicom")]
    storage_dir: String,

    /// 配置文件路径
    #[arg(short, long)]
    config: Option<String>,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();

    info!("启动PACS服务器...");

    // 创建服务器配置
    let server_config = DicomServerConfig {
        ae_title: args.ae_title.clone(),
        port: args.port,
        max_associations: 100,
        storage_dir: args.storage_dir.clone(),
    };

    info!("PACS服务器配置:");
    info!("  AE标题: {}", server_config.ae_title);
    info!("  监听端口: {}", server_config.port);
    info!("  存储目录: {}", server_config.storage_dir);

    // 创建并启动DICOM服务器
    let server = DicomServer::new(server_config).await?;

    // 启动服务器
    if let Err(e) = server.start().await {
        error!("服务器启动失败: {}", e);
        return Err(e);
    }

    Ok(())
}
