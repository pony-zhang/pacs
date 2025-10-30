//! DICOM服务实现

use async_trait::async_trait;
use pacs_core::{PacsError, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// DICOM服务特征
#[async_trait]
pub trait DicomService: Send + Sync {
    async fn handle_request(&self, request: DimseRequest) -> Result<DimseResponse>;
}

/// DICOM消息服务元素请求
#[derive(Debug, Clone)]
pub struct DimseRequest {
    pub command_field: CommandField,
    pub message_id: u16,
    pub affected_sop_class_uid: String,
    pub dataset: Option<Vec<u8>>,
}

/// DICOM消息服务元素响应
#[derive(Debug, Clone)]
pub struct DimseResponse {
    pub command_field: CommandField,
    pub message_id_being_responded_to: u16,
    pub status: DimseStatus,
    pub affected_sop_class_uid: String,
    pub dataset: Option<Vec<u8>>,
}

/// DICOM命令字段
#[derive(Debug, Clone, PartialEq)]
pub enum CommandField {
    CStore,
    CFind,
    CMove,
    CGet,
    CEcho,
    CCancel,
}

/// DIMSE状态码
#[derive(Debug, Clone)]
pub enum DimseStatus {
    Success,
    Warning,
    Failure(u16),
    Pending,
    Cancel,
}

/// C-ECHO服务
pub struct CEchoService;

#[async_trait]
impl DicomService for CEchoService {
    async fn handle_request(&self, request: DimseRequest) -> Result<DimseResponse> {
        debug!("处理C-ECHO请求");

        Ok(DimseResponse {
            command_field: CommandField::CEcho,
            message_id_being_responded_to: request.message_id,
            status: DimseStatus::Success,
            affected_sop_class_uid: request.affected_sop_class_uid,
            dataset: None,
        })
    }
}

/// C-STORE服务
pub struct CStoreService {
    storage_dir: String,
}

impl CStoreService {
    pub fn new(storage_dir: String) -> Self {
        Self { storage_dir }
    }
}

#[async_trait]
impl DicomService for CStoreService {
    async fn handle_request(&self, request: DimseRequest) -> Result<DimseResponse> {
        info!("处理C-STORE请求");

        match request.dataset {
            Some(dataset) => {
                // 这里应该解析DICOM数据集并存储
                debug!("接收到DICOM数据集，大小: {} bytes", dataset.len());

                // 简化实现：直接写入文件
                let filename = format!(
                    "{}/{}.dcm",
                    self.storage_dir,
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                );

                tokio::fs::write(&filename, dataset).await?;
                info!("DICOM文件已存储: {}", filename);

                Ok(DimseResponse {
                    command_field: CommandField::CStore,
                    message_id_being_responded_to: request.message_id,
                    status: DimseStatus::Success,
                    affected_sop_class_uid: request.affected_sop_class_uid,
                    dataset: None,
                })
            }
            None => {
                warn!("C-STORE请求缺少数据集");
                Ok(DimseResponse {
                    command_field: CommandField::CStore,
                    message_id_being_responded_to: request.message_id,
                    status: DimseStatus::Failure(0xC000), // 失败
                    affected_sop_class_uid: request.affected_sop_class_uid,
                    dataset: None,
                })
            }
        }
    }
}

/// C-FIND服务
pub struct CFindService {
    // 这里应该包含数据库连接
}

impl CFindService {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl DicomService for CFindService {
    async fn handle_request(&self, request: DimseRequest) -> Result<DimseResponse> {
        debug!("处理C-FIND请求");

        // 简化实现：返回空结果
        Ok(DimseResponse {
            command_field: CommandField::CFind,
            message_id_being_responded_to: request.message_id,
            status: DimseStatus::Success,
            affected_sop_class_uid: request.affected_sop_class_uid,
            dataset: None,
        })
    }
}

/// DICOM服务管理器
pub struct ServiceManager {
    services: HashMap<String, Box<dyn DicomService>>,
}

impl ServiceManager {
    pub fn new() -> Self {
        let mut services = HashMap::new();

        // 注册标准服务
        services.insert(
            "1.2.840.10008.1.1".to_string(), // Verification SOP Class
            Box::new(CEchoService) as Box<dyn DicomService>,
        );

        Self { services }
    }

    pub fn register_service(&mut self, sop_class_uid: String, service: Box<dyn DicomService>) {
        self.services.insert(sop_class_uid, service);
    }

    pub async fn handle_request(&self, request: DimseRequest) -> Result<DimseResponse> {
        match self.services.get(&request.affected_sop_class_uid) {
            Some(service) => service.handle_request(request).await,
            None => {
                warn!("不支持的SOP类: {}", request.affected_sop_class_uid);
                Ok(DimseResponse {
                    command_field: request.command_field,
                    message_id_being_responded_to: request.message_id,
                    status: DimseStatus::Failure(0x0122), // SOP类不支持
                    affected_sop_class_uid: request.affected_sop_class_uid,
                    dataset: None,
                })
            }
        }
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
