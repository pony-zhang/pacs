//! 影像存储管理

use chrono::{DateTime, Utc};
use object_store::{path::Path as ObjectPath, GetOptions, ObjectStore, PutOptions};
use pacs_core::{PacsError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// 存储类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageType {
    /// 本地文件系统
    Local,
    /// 对象存储 (S3, GCS, Azure等)
    ObjectStorage,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 存储类型
    pub storage_type: StorageType,
    /// 本地存储路径
    pub local_path: Option<String>,
    /// 对象存储配置
    pub object_store_config: Option<ObjectStoreConfig>,
}

/// 对象存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStoreConfig {
    /// AWS S3配置
    pub aws: Option<AwsS3Config>,
    /// Google Cloud Storage配置
    pub gcs: Option<GcsConfig>,
    /// Azure Blob Storage配置
    pub azure: Option<AzureConfig>,
}

/// AWS S3配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsS3Config {
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: Option<String>,
}

/// Google Cloud Storage配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsConfig {
    pub bucket: String,
    pub service_account_key: String,
}

/// Azure Blob Storage配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    pub container: String,
    pub account: String,
    pub access_key: String,
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// 总文件数
    pub total_files: u64,
    /// 总存储大小（字节）
    pub total_size: u64,
    /// 可用空间（字节）
    pub available_space: Option<u64>,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 存储管理器
pub struct StorageManager {
    config: StorageConfig,
    local_path: Option<String>,
    object_store: Option<Arc<dyn ObjectStore>>,
}

impl StorageManager {
    /// 创建新的存储管理器
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let local_path = config.local_path.clone();
        let object_store = match &config.storage_type {
            StorageType::ObjectStorage => {
                if let Some(os_config) = &config.object_store_config {
                    Some(Self::create_object_store(os_config).await?)
                } else {
                    return Err(PacsError::Config(
                        "Missing object store configuration".to_string(),
                    ));
                }
            }
            StorageType::Local => None,
        };

        Ok(Self {
            config,
            local_path,
            object_store,
        })
    }

    /// 创建对象存储客户端
    async fn create_object_store(config: &ObjectStoreConfig) -> Result<Arc<dyn ObjectStore>> {
        if let Some(aws_config) = &config.aws {
            #[cfg(feature = "aws")]
            use object_store::aws::AmazonS3Builder;

            let mut builder = AmazonS3Builder::new()
                .with_bucket_name(&aws_config.bucket)
                .with_region(&aws_config.region)
                .with_access_key_id(&aws_config.access_key_id)
                .with_secret_access_key(&aws_config.secret_access_key);

            if let Some(endpoint) = &aws_config.endpoint {
                builder = builder.with_endpoint(endpoint);
            }

            Ok(Arc::new(builder.build()?))
        } else if let Some(_gcs_config) = &config.gcs {
            return Err(PacsError::Config(
                "Google Cloud Storage not yet implemented".to_string(),
            ));
        } else if let Some(_azure_config) = &config.azure {
            return Err(PacsError::Config(
                "Azure Blob Storage not yet implemented".to_string(),
            ));
        } else {
            return Err(PacsError::Config(
                "No valid object store configuration found".to_string(),
            ));
        }
    }

    /// 存储DICOM文件
    pub async fn store_file(&self, data: &[u8], path: &str) -> Result<String> {
        match &self.config.storage_type {
            StorageType::Local => {
                let base_path = self.local_path.as_ref().ok_or_else(|| {
                    PacsError::Config("Local storage path not configured".to_string())
                })?;
                let full_path = Path::new(base_path).join(path);

                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                tokio::fs::write(&full_path, data).await?;
                Ok(full_path.to_string_lossy().to_string())
            }
            StorageType::ObjectStorage => {
                let store = self
                    .object_store
                    .as_ref()
                    .ok_or_else(|| PacsError::Config("Object store not initialized".to_string()))?;

                let object_path = ObjectPath::from(path);
                store
                    .put_opts(&object_path, data.into(), PutOptions::default())
                    .await?;
                Ok(path.to_string())
            }
        }
    }

    /// 获取文件
    pub async fn get_file(&self, path: &str) -> Result<Vec<u8>> {
        match &self.config.storage_type {
            StorageType::Local => {
                let base_path = self.local_path.as_ref().ok_or_else(|| {
                    PacsError::Config("Local storage path not configured".to_string())
                })?;
                let full_path = Path::new(base_path).join(path);
                let data = tokio::fs::read(full_path).await?;
                Ok(data)
            }
            StorageType::ObjectStorage => {
                let store = self
                    .object_store
                    .as_ref()
                    .ok_or_else(|| PacsError::Config("Object store not initialized".to_string()))?;

                let object_path = ObjectPath::from(path);
                let result = store.get_opts(&object_path, GetOptions::default()).await?;
                let data = result.bytes().await?;
                Ok(data.to_vec())
            }
        }
    }

    /// 检查文件是否存在
    pub async fn file_exists(&self, path: &str) -> Result<bool> {
        match &self.config.storage_type {
            StorageType::Local => {
                let base_path = self.local_path.as_ref().ok_or_else(|| {
                    PacsError::Config("Local storage path not configured".to_string())
                })?;
                let full_path = Path::new(base_path).join(path);
                Ok(tokio::fs::metadata(full_path).await.is_ok())
            }
            StorageType::ObjectStorage => {
                let store = self
                    .object_store
                    .as_ref()
                    .ok_or_else(|| PacsError::Config("Object store not initialized".to_string()))?;

                let object_path = ObjectPath::from(path);
                Ok(store.head(&object_path).await.is_ok())
            }
        }
    }

    /// 删除文件
    pub async fn delete_file(&self, path: &str) -> Result<()> {
        match &self.config.storage_type {
            StorageType::Local => {
                let base_path = self.local_path.as_ref().ok_or_else(|| {
                    PacsError::Config("Local storage path not configured".to_string())
                })?;
                let full_path = Path::new(base_path).join(path);
                tokio::fs::remove_file(full_path).await?;
                Ok(())
            }
            StorageType::ObjectStorage => {
                let store = self
                    .object_store
                    .as_ref()
                    .ok_or_else(|| PacsError::Config("Object store not initialized".to_string()))?;

                let object_path = ObjectPath::from(path);
                store.delete(&object_path).await?;
                Ok(())
            }
        }
    }

    /// 获取存储统计信息
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        match &self.config.storage_type {
            StorageType::Local => {
                let base_path = self.local_path.as_ref().ok_or_else(|| {
                    PacsError::Config("Local storage path not configured".to_string())
                })?;

                let (total_files, total_size) = self.scan_local_directory(base_path).await?;

                // 获取可用空间
                let available_space = match tokio::fs::metadata(base_path).await {
                    Ok(_) => {
                        // 在Unix系统上，我们需要获取文件系统的信息
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;
                            match tokio::fs::metadata(base_path).await {
                                Ok(meta) => Some(meta.available()),
                                Err(_) => None,
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            None
                        }
                    }
                    Err(_) => None,
                };

                Ok(StorageStats {
                    total_files,
                    total_size,
                    available_space,
                    last_updated: Utc::now(),
                })
            }
            StorageType::ObjectStorage => {
                // 对象存储的统计信息获取比较复杂，这里提供简化版本
                Ok(StorageStats {
                    total_files: 0,
                    total_size: 0,
                    available_space: None,
                    last_updated: Utc::now(),
                })
            }
        }
    }

    /// 扫描本地目录获取统计信息
    fn scan_local_directory(
        &self,
        dir_path: &str,
    ) -> impl std::future::Future<Output = Result<(u64, u64)>> + '_ {
        async move {
            let mut total_files = 0u64;
            let mut total_size = 0u64;

            let mut entries = tokio::fs::read_dir(dir_path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    let (files, size) = self.scan_local_directory(path.to_str().unwrap()).await?;
                    total_files += files;
                    total_size += size;
                } else {
                    total_files += 1;
                    if let Ok(metadata) = entry.metadata().await {
                        total_size += metadata.len();
                    }
                }
            }

            Ok((total_files, total_size))
        }
    }

    /// 获取存储类型
    pub fn storage_type(&self) -> &StorageType {
        &self.config.storage_type
    }
}
