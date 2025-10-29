//! DICOM服务器实现

use pacs_core::{PacsError, Result};
use crate::{
    association::{AssociationManager, PresentationContext, PresentationContextResult},
    services::{ServiceManager, DicomService},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Decoder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug};
use std::net::SocketAddr;

/// DICOM服务器配置
#[derive(Debug, Clone)]
pub struct DicomServerConfig {
    pub ae_title: String,           // 应用实体标题
    pub port: u16,                  // 监听端口
    pub max_associations: u32,      // 最大关联数
    pub storage_dir: String,        // 存储目录
}

impl Default for DicomServerConfig {
    fn default() -> Self {
        Self {
            ae_title: "PACS_SERVER".to_string(),
            port: 11112,
            max_associations: 100,
            storage_dir: "./data/dicom".to_string(),
        }
    }
}

/// DICOM服务器
pub struct DicomServer {
    config: DicomServerConfig,
    association_manager: AssociationManager,
    service_manager: ServiceManager,
}

impl DicomServer {
    /// 创建新的DICOM服务器
    pub async fn new(config: DicomServerConfig) -> Result<Self> {
        // 确保存储目录存在
        tokio::fs::create_dir_all(&config.storage_dir).await?;

        Ok(Self {
            config,
            association_manager: AssociationManager::new(),
            service_manager: ServiceManager::new(),
        })
    }

    /// 启动DICOM服务器
    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        let listener = TcpListener::bind(addr).await?;

        info!("DICOM服务器启动: AE={}, 地址={}", self.config.ae_title, addr);

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    info!("接受连接: {}", remote_addr);
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, remote_addr).await {
                            error!("处理连接失败: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("接受连接失败: {}", e);
                }
            }
        }
    }

    /// 处理客户端连接
    async fn handle_connection(&self, mut stream: TcpStream, remote_addr: SocketAddr) -> Result<()> {
        debug!("处理DICOM连接: {}", remote_addr);

        // 简化实现：直接处理数据
        let mut buffer = vec![0; 4096];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    debug!("连接关闭: {}", remote_addr);
                    break;
                }
                Ok(n) => {
                    debug!("接收到数据: {} bytes", n);
                    // 这里应该解析DICOM PDU并处理
                    // 简化实现：发送响应
                    let response = b"DICOM_RESPONSE";
                    stream.write_all(response).await?;
                }
                Err(e) => {
                    error!("读取数据失败: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 注册自定义DICOM服务
    pub fn register_service(&mut self, sop_class_uid: String, service: Box<dyn DicomService>) {
        self.service_manager.register_service(sop_class_uid, service);
    }
}

impl Clone for DicomServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            association_manager: AssociationManager::new(),
            service_manager: ServiceManager::new(),
        }
    }
}

/// DICOM网络编解码器
pub struct DicomCodec;

impl Decoder for DicomCodec {
    type Item = Vec<u8>;
    type Error = PacsError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>> {
        if src.len() < 6 {
            return Ok(None);
        }

        // 简化的PDU解析
        let pdu_length = u32::from_be_bytes([src[2], src[3], src[4], src[5]]) as usize;
        let total_length = 6 + pdu_length;

        if src.len() < total_length {
            return Ok(None);
        }

        Ok(Some(src.split_to(total_length).to_vec()))
    }
}