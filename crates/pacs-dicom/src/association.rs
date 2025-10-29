//! DICOM关联管理

use pacs_core::{PacsError, Result};
use tracing::{debug, info};
use std::net::SocketAddr;

/// DICOM关联信息
#[derive(Debug, Clone)]
pub struct AssociationInfo {
    pub id: String,
    pub remote_addr: SocketAddr,
    pub calling_ae_title: String,
    pub called_ae_title: String,
    pub max_pdu_length: u32,
    pub presentation_contexts: Vec<PresentationContext>,
    pub established_at: chrono::DateTime<chrono::Utc>,
}

/// 表示上下文
#[derive(Debug, Clone)]
pub struct PresentationContext {
    pub id: u8,
    pub abstract_syntax: String,
    pub transfer_syntaxes: Vec<String>,
    pub result: PresentationContextResult,
}

/// 表示上下文结果
#[derive(Debug, Clone)]
pub enum PresentationContextResult {
    Acceptance,
    Rejection,
    AbstractSyntaxNotSupported,
    TransferSyntaxNotSupported,
}

/// DICOM关联管理器
pub struct AssociationManager {
    associations: std::collections::HashMap<String, AssociationInfo>,
}

impl AssociationManager {
    pub fn new() -> Self {
        Self {
            associations: std::collections::HashMap::new(),
        }
    }

    /// 建立新的DICOM关联
    pub async fn establish_association(
        &mut self,
        remote_addr: SocketAddr,
        calling_ae_title: String,
        called_ae_title: String,
        presentation_contexts: Vec<PresentationContext>,
    ) -> Result<String> {
        let association_id = uuid::Uuid::new_v4().to_string();

        let association_info = AssociationInfo {
            id: association_id.clone(),
            remote_addr,
            calling_ae_title,
            called_ae_title,
            max_pdu_length: 16384, // 默认值
            presentation_contexts,
            established_at: chrono::Utc::now(),
        };

        info!("建立DICOM关联: {:?}", association_info);
        self.associations.insert(association_id.clone(), association_info);

        Ok(association_id)
    }

    /// 关闭DICOM关联
    pub async fn close_association(&mut self, association_id: &str) -> Result<()> {
        if let Some(association) = self.associations.remove(association_id) {
            info!("关闭DICOM关联: {} from {}",
                  association.id, association.remote_addr);
        }
        Ok(())
    }

    /// 获取关联信息
    pub fn get_association(&self, association_id: &str) -> Option<&AssociationInfo> {
        self.associations.get(association_id)
    }

    /// 列出所有活跃的关联
    pub fn list_associations(&self) -> Vec<&AssociationInfo> {
        self.associations.values().collect()
    }
}

impl Default for AssociationManager {
    fn default() -> Self {
        Self::new()
    }
}