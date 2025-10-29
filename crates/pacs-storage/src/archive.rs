//! 归档管理

use pacs_core::Result;

/// 归档管理器
pub struct ArchiveManager;

impl ArchiveManager {
    pub fn new() -> Self {
        Self
    }

    /// 归档文件
    pub async fn archive_file(&self, path: &str) -> Result<()> {
        // TODO: 实现归档逻辑
        Ok(())
    }
}