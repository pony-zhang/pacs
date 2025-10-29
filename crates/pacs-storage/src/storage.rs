//! 影像存储管理

use pacs_core::{PacsError, Result};
use std::path::Path;

/// 存储管理器
pub struct StorageManager {
    base_path: String,
}

impl StorageManager {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
        }
    }

    /// 存储DICOM文件
    pub async fn store_file(&self, data: &[u8], path: &str) -> Result<String> {
        let full_path = Path::new(&self.base_path).join(path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, data).await?;
        Ok(full_path.to_string_lossy().to_string())
    }

    /// 获取文件
    pub async fn get_file(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = Path::new(&self.base_path).join(path);
        let data = tokio::fs::read(full_path).await?;
        Ok(data)
    }
}