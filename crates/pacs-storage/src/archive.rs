//! 归档管理

use crate::lifecycle::{LifecycleManager, LifecycleStage};
use crate::storage::{StorageConfig, StorageManager, StorageType};
use chrono::{DateTime, Utc};
use pacs_core::{PacsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// 归档策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePolicy {
    /// 策略名称
    pub name: String,
    /// 归档条件
    pub conditions: Vec<ArchiveCondition>,
    /// 目标存储配置
    pub target_storage: StorageConfig,
    /// 压缩设置
    pub compression_settings: Option<CompressionSettings>,
    /// 是否启用
    pub enabled: bool,
}

/// 归档条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveCondition {
    /// 基于时间（天数）
    TimeBasedDays(u32),
    /// 基于文件大小（字节）
    FileSizeGreaterThan(u64),
    /// 基于访问频率
    AccessFrequencyLessThan(u32), // 30天内访问次数少于N次
    /// 基于路径前缀
    PathPrefix(String),
}

/// 压缩设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionSettings {
    /// 压缩算法
    pub algorithm: CompressionAlgorithm,
    /// 压缩级别
    pub level: u8,
}

/// 压缩算法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// Gzip
    Gzip,
    /// Zstd
    Zstd,
    /// LZ4
    Lz4,
}

/// 归档任务状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArchiveTaskStatus {
    /// 等待中
    Pending,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 归档任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveTask {
    /// 任务ID
    pub id: String,
    /// 策略名称
    pub policy_name: String,
    /// 文件路径
    pub file_path: String,
    /// 原始存储路径
    pub original_path: String,
    /// 归档路径
    pub archive_path: String,
    /// 状态
    pub status: ArchiveTaskStatus,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 原始文件大小
    pub original_size: u64,
    /// 归档文件大小
    pub archive_size: Option<u64>,
    /// 压缩率
    pub compression_ratio: Option<f64>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 归档管理器
pub struct ArchiveManager {
    /// 存储管理器
    storage_managers: HashMap<String, StorageManager>,
    /// 归档策略
    policies: HashMap<String, ArchivePolicy>,
    /// 生命周期管理器
    lifecycle_manager: LifecycleManager,
    /// 归档任务历史
    task_history: Vec<ArchiveTask>,
    /// 活跃任务
    active_tasks: HashMap<String, ArchiveTask>,
}

impl ArchiveManager {
    /// 创建新的归档管理器
    pub fn new() -> Self {
        Self {
            storage_managers: HashMap::new(),
            policies: HashMap::new(),
            lifecycle_manager: LifecycleManager::new(),
            task_history: Vec::new(),
            active_tasks: HashMap::new(),
        }
    }

    /// 添加存储管理器
    pub fn add_storage_manager(&mut self, name: String, storage_manager: StorageManager) {
        self.storage_managers.insert(name, storage_manager);
    }

    /// 添加归档策略
    pub fn add_policy(&mut self, policy: ArchivePolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    /// 手动归档文件
    pub async fn archive_file(&mut self, file_path: &str, policy_name: &str) -> Result<String> {
        let policy = self
            .policies
            .get(policy_name)
            .ok_or_else(|| PacsError::configuration("Archive policy not found"))?;

        if !policy.enabled {
            return Err(PacsError::configuration("Archive policy is disabled"));
        }

        let task_id = format!("archive_{}_{}", policy_name, Utc::now().timestamp());

        let task = ArchiveTask {
            id: task_id.clone(),
            policy_name: policy_name.to_string(),
            file_path: file_path.to_string(),
            original_path: file_path.to_string(),
            archive_path: String::new(), // 将在执行时设置
            status: ArchiveTaskStatus::Pending,
            start_time: Utc::now(),
            end_time: None,
            original_size: 0,
            archive_size: None,
            compression_ratio: None,
            error_message: None,
        };

        self.active_tasks.insert(task_id.clone(), task);

        info!("Created archive task: {} for file: {}", task_id, file_path);

        // 执行归档
        self.execute_archive_task(&task_id).await?;

        Ok(task_id)
    }

    /// 执行归档任务
    async fn execute_archive_task(&mut self, task_id: &str) -> Result<()> {
        let task = self
            .active_tasks
            .get_mut(task_id)
            .ok_or_else(|| PacsError::configuration("Archive task not found"))?;

        let policy = self
            .policies
            .get(&task.policy_name)
            .ok_or_else(|| PacsError::configuration("Archive policy not found"))?;

        task.status = ArchiveTaskStatus::InProgress;

        info!("Executing archive task: {}", task_id);

        // 获取源存储管理器（默认使用第一个存储管理器）
        let source_storage = self
            .storage_managers
            .values()
            .next()
            .ok_or_else(|| PacsError::configuration("No storage manager available"))?;

        // 获取文件信息
        let file_data = source_storage.get_file(&task.file_path).await?;
        task.original_size = file_data.len() as u64;

        // 创建目标存储管理器
        let target_storage = StorageManager::new(policy.target_storage.clone()).await?;

        // 生成归档路径
        let archive_path = self.generate_archive_path(&task.file_path);
        task.archive_path = archive_path.clone();

        // 执行压缩（如果启用）
        let processed_data = if let Some(compression_settings) = &policy.compression_settings {
            self.compress_data(&file_data, compression_settings).await?
        } else {
            file_data
        };

        task.archive_size = Some(processed_data.len() as u64);
        task.compression_ratio = Some(1.0 - (processed_data.len() as f64 / file_data.len() as f64));

        // 存储到归档位置
        target_storage
            .store_file(&processed_data, &archive_path)
            .await?;

        // 从源存储删除原文件
        source_storage.delete_file(&task.file_path).await?;

        // 更新任务状态
        task.status = ArchiveTaskStatus::Completed;
        task.end_time = Some(Utc::now());

        info!(
            "Archive task completed: {} (compressed to {} bytes, ratio: {:.2}%)",
            task_id,
            processed_data.len(),
            task.compression_ratio.unwrap_or(0.0) * 100.0
        );

        // 移动到历史记录
        if let Some(completed_task) = self.active_tasks.remove(task_id) {
            self.task_history.push(completed_task);
        }

        // 更新生命周期管理
        if let Err(e) = self
            .lifecycle_manager
            .transition_file(&task.file_path, LifecycleStage::Archive)
            .await
        {
            warn!(
                "Failed to update lifecycle status for {}: {}",
                task.file_path, e
            );
        }

        Ok(())
    }

    /// 生成归档路径
    fn generate_archive_path(&self, original_path: &str) -> String {
        let date = Utc::now().format("%Y/%m/%d");
        let filename = std::path::Path::new(original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        format!("archive/{}/{}/{}", date, Utc::now().timestamp(), filename)
    }

    /// 压缩数据
    async fn compress_data(&self, data: &[u8], settings: &CompressionSettings) -> Result<Vec<u8>> {
        match settings.algorithm {
            CompressionAlgorithm::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;

                let mut encoder =
                    GzEncoder::new(Vec::new(), Compression::new(settings.level.into()));
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            CompressionAlgorithm::Zstd => {
                // TODO: 实现zstd压缩
                warn!("Zstd compression not yet implemented, using original data");
                Ok(data.to_vec())
            }
            CompressionAlgorithm::Lz4 => {
                // TODO: 实现lz4压缩
                warn!("LZ4 compression not yet implemented, using original data");
                Ok(data.to_vec())
            }
        }
    }

    /// 自动归档处理
    pub async fn process_auto_archive(&mut self) -> Result<Vec<String>> {
        let mut created_tasks = Vec::new();

        for (policy_name, policy) in &self.policies {
            if !policy.enabled {
                continue;
            }

            info!("Processing auto archive for policy: {}", policy_name);

            // 获取符合条件的文件
            let eligible_files = self.find_eligible_files(policy).await?;

            for file_path in eligible_files {
                // 检查是否已有归档任务
                let has_active_task = self
                    .active_tasks
                    .values()
                    .any(|t| t.file_path == file_path && t.policy_name == *policy_name);

                let has_completed_task = self
                    .task_history
                    .iter()
                    .any(|t| t.file_path == file_path && t.policy_name == *policy_name);

                if !has_active_task && !has_completed_task {
                    if let Ok(task_id) = self.archive_file(&file_path, policy_name).await {
                        created_tasks.push(task_id);
                    }
                }
            }
        }

        if !created_tasks.is_empty() {
            info!("Created {} archive tasks", created_tasks.len());
        }

        Ok(created_tasks)
    }

    /// 查找符合条件的文件
    async fn find_eligible_files(&self, policy: &ArchivePolicy) -> Result<Vec<String>> {
        let mut eligible_files = Vec::new();

        // 简化实现，实际应用中需要遍历存储并检查每个文件
        // 这里提供一个基本的框架

        // 获取存储管理器
        let storage_manager = self
            .storage_managers
            .values()
            .next()
            .ok_or_else(|| PacsError::configuration("No storage manager available"))?;

        // TODO: 实现文件遍历和条件检查逻辑
        // 这里需要根据具体的存储类型实现文件列表获取

        debug!("Searching for eligible files for policy: {}", policy.name);

        Ok(eligible_files)
    }

    /// 从归档恢复文件
    pub async fn restore_file(&mut self, task_id: &str, target_path: &str) -> Result<()> {
        // 查找归档任务
        let archive_task = self
            .task_history
            .iter()
            .find(|t| t.id == task_id && t.status == ArchiveTaskStatus::Completed)
            .ok_or_else(|| PacsError::configuration("Archive task not found or not completed"))?;

        info!(
            "Restoring file from archive: {} to {}",
            task_id, target_path
        );

        // 获取归档存储配置
        let policy = self
            .policies
            .get(&archive_task.policy_name)
            .ok_or_else(|| PacsError::configuration("Archive policy not found"))?;

        // 创建归档存储管理器
        let archive_storage = StorageManager::new(policy.target_storage.clone()).await?;

        // 读取归档文件
        let archived_data = archive_storage.get_file(&archive_task.archive_path).await?;

        // 解压缩（如果需要）
        let restored_data = if policy.compression_settings.is_some() {
            self.decompress_data(
                &archived_data,
                &policy.compression_settings.as_ref().unwrap(),
            )
            .await?
        } else {
            archived_data
        };

        // 存储到目标位置
        let target_storage = self
            .storage_managers
            .values()
            .next()
            .ok_or_else(|| PacsError::configuration("No storage manager available"))?;

        target_storage
            .store_file(&restored_data, target_path)
            .await?;

        info!("File restored successfully: {}", target_path);

        Ok(())
    }

    /// 解压缩数据
    async fn decompress_data(
        &self,
        data: &[u8],
        settings: &CompressionSettings,
    ) -> Result<Vec<u8>> {
        match settings.algorithm {
            CompressionAlgorithm::Gzip => {
                use flate2::read::GzDecoder;
                use std::io::Read;

                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            _ => {
                warn!("Decompression not implemented for algorithm, using original data");
                Ok(data.to_vec())
            }
        }
    }

    /// 获取归档任务列表
    pub fn get_task_history(&self) -> &[ArchiveTask] {
        &self.task_history
    }

    /// 获取活跃任务
    pub fn get_active_tasks(&self) -> &HashMap<String, ArchiveTask> {
        &self.active_tasks
    }

    /// 创建默认归档策略
    pub fn create_default_policy(target_storage: StorageConfig) -> ArchivePolicy {
        ArchivePolicy {
            name: "Default Archive Policy".to_string(),
            conditions: vec![
                ArchiveCondition::TimeBasedDays(365),          // 1年后归档
                ArchiveCondition::AccessFrequencyLessThan(10), // 30天内访问少于10次
            ],
            target_storage,
            compression_settings: Some(CompressionSettings {
                algorithm: CompressionAlgorithm::Gzip,
                level: 6,
            }),
            enabled: true,
        }
    }
}

impl Default for ArchiveManager {
    fn default() -> Self {
        Self::new()
    }
}
