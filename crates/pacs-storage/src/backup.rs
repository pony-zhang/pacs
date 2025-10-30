//! 备份和恢复机制

use crate::storage::{StorageConfig, StorageManager};
use chrono::{DateTime, Duration, Utc};
use pacs_core::{PacsError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// 备份类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackupType {
    /// 完整备份
    Full,
    /// 增量备份
    Incremental,
    /// 差异备份
    Differential,
}

/// 备份状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackupStatus {
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

/// 备份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// 备份名称
    pub name: String,
    /// 备份类型
    pub backup_type: BackupType,
    /// 源存储配置
    pub source_storage: StorageConfig,
    /// 目标存储配置
    pub target_storage: StorageConfig,
    /// 备份路径前缀
    pub backup_prefix: String,
    /// 备份计划（cron表达式，简化版）
    pub schedule: Option<String>,
    /// 保留备份数量
    pub retention_count: u32,
    /// 是否启用压缩
    pub compression_enabled: bool,
    /// 是否启用加密
    pub encryption_enabled: bool,
}

/// 备份信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// 备份ID
    pub id: String,
    /// 备份配置名称
    pub config_name: String,
    /// 备份类型
    pub backup_type: BackupType,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 备份状态
    pub status: BackupStatus,
    /// 备份文件数量
    pub file_count: u64,
    /// 备份数据大小
    pub total_size: u64,
    /// 错误信息
    pub error_message: Option<String>,
    /// 基础备份ID（用于增量/差异备份）
    pub base_backup_id: Option<String>,
    /// 文件清单
    pub file_manifest: Vec<BackupFileEntry>,
}

/// 备份文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFileEntry {
    /// 原始文件路径
    pub original_path: String,
    /// 备份文件路径
    pub backup_path: String,
    /// 文件大小
    pub size: u64,
    /// 文件哈希值
    pub hash: String,
    /// 修改时间
    pub modified_time: DateTime<Utc>,
    /// 备份状态
    pub backup_status: BackupStatus,
}

/// 恢复信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreInfo {
    /// 恢复ID
    pub id: String,
    /// 备份ID
    pub backup_id: String,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 恢复状态
    pub status: BackupStatus,
    /// 恢复文件数量
    pub file_count: u64,
    /// 恢复数据大小
    pub total_size: u64,
    /// 目标路径
    pub target_path: String,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 备份管理器
pub struct BackupManager {
    /// 备份配置
    configs: HashMap<String, BackupConfig>,
    /// 存储管理器
    storage_managers: HashMap<String, StorageManager>,
    /// 备份历史
    backup_history: Vec<BackupInfo>,
    /// 当前正在进行的备份
    active_backups: HashMap<String, BackupInfo>,
}

impl BackupManager {
    /// 创建新的备份管理器
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            storage_managers: HashMap::new(),
            backup_history: Vec::new(),
            active_backups: HashMap::new(),
        }
    }

    /// 添加备份配置
    pub fn add_config(&mut self, config: BackupConfig) -> Result<()> {
        self.configs.insert(config.name.clone(), config);
        Ok(())
    }

    /// 执行备份
    pub async fn execute_backup(
        &mut self,
        config_name: &str,
        backup_type: BackupType,
    ) -> Result<String> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| PacsError::configuration("Backup configuration not found"))?;

        let backup_id = format!("backup_{}_{}", config_name, Utc::now().timestamp());

        let backup_info = BackupInfo {
            id: backup_id.clone(),
            config_name: config_name.to_string(),
            backup_type: backup_type.clone(),
            start_time: Utc::now(),
            end_time: None,
            status: BackupStatus::InProgress,
            file_count: 0,
            total_size: 0,
            error_message: None,
            base_backup_id: None,
            file_manifest: Vec::new(),
        };

        self.active_backups
            .insert(backup_id.clone(), backup_info.clone());

        info!("Starting backup: {} ({})", backup_id, backup_type);

        let result = match backup_type {
            BackupType::Full => self.execute_full_backup(&backup_id, config).await,
            BackupType::Incremental => self.execute_incremental_backup(&backup_id, config).await,
            BackupType::Differential => self.execute_differential_backup(&backup_id, config).await,
        };

        let (file_count, total_size, file_manifest) = match result {
            Ok((count, size, manifest)) => {
                info!(
                    "Backup completed successfully: {} (files: {}, size: {} bytes)",
                    backup_id, count, size
                );
                (count, size, manifest)
            }
            Err(e) => {
                error!("Backup failed: {} - {}", backup_id, e);
                if let Some(active_backup) = self.active_backups.get_mut(&backup_id) {
                    active_backup.status = BackupStatus::Failed;
                    active_backup.error_message = Some(e.to_string());
                }
                return Err(e);
            }
        };

        // 更新备份信息
        if let Some(active_backup) = self.active_backups.remove(&backup_id) {
            let mut completed_backup = active_backup;
            completed_backup.end_time = Some(Utc::now());
            completed_backup.status = BackupStatus::Completed;
            completed_backup.file_count = file_count;
            completed_backup.total_size = total_size;
            completed_backup.file_manifest = file_manifest;

            self.backup_history.push(completed_backup);

            // 清理过期备份
            self.cleanup_expired_backups(config_name).await?;
        }

        Ok(backup_id)
    }

    /// 执行完整备份
    async fn execute_full_backup(
        &mut self,
        backup_id: &str,
        config: &BackupConfig,
    ) -> Result<(u64, u64, Vec<BackupFileEntry>)> {
        let source_storage = self.get_storage_manager(&config.source_storage).await?;
        let target_storage = self.get_storage_manager(&config.target_storage).await?;

        let mut file_count = 0u64;
        let mut total_size = 0u64;
        let mut file_manifest = Vec::new();

        // 这里简化处理，实际应用中需要遍历源存储的所有文件
        // 可以通过存储管理器的统计信息获取文件列表

        info!("Executing full backup: {}", backup_id);

        // 示例：备份所有DICOM文件
        // 实际实现需要根据具体存储类型进行文件遍历

        Ok((file_count, total_size, file_manifest))
    }

    /// 执行增量备份
    async fn execute_incremental_backup(
        &mut self,
        backup_id: &str,
        config: &BackupConfig,
    ) -> Result<(u64, u64, Vec<BackupFileEntry>)> {
        // 找到最近的基础备份
        let base_backup = self
            .find_latest_backup(&config.config_name, BackupType::Full)
            .or_else(|| self.find_latest_backup(&config.config_name, BackupType::Differential));

        if base_backup.is_none() {
            return Err(PacsError::configuration(
                "No base backup found for incremental backup",
            ));
        }

        info!(
            "Executing incremental backup: {} (base: {:?})",
            backup_id,
            base_backup.as_ref().map(|b| &b.id)
        );

        // TODO: 实现增量备份逻辑
        // 比较文件修改时间和哈希值，只备份变更的文件

        Ok((0, 0, Vec::new()))
    }

    /// 执行差异备份
    async fn execute_differential_backup(
        &mut self,
        backup_id: &str,
        config: &BackupConfig,
    ) -> Result<(u64, u64, Vec<BackupFileEntry>)> {
        // 找到最近的基础备份
        let base_backup = self.find_latest_backup(&config.config_name, BackupType::Full);

        if base_backup.is_none() {
            return Err(PacsError::configuration(
                "No full backup found for differential backup",
            ));
        }

        info!(
            "Executing differential backup: {} (base: {:?})",
            backup_id,
            base_backup.as_ref().map(|b| &b.id)
        );

        // TODO: 实现差异备份逻辑
        // 备份自上次完整备份以来的所有变更文件

        Ok((0, 0, Vec::new()))
    }

    /// 恢复备份
    pub async fn restore_backup(&mut self, backup_id: &str, target_path: &str) -> Result<String> {
        let backup_info = self
            .backup_history
            .iter()
            .find(|b| b.id == backup_id)
            .or_else(|| self.active_backups.get(backup_id))
            .ok_or_else(|| PacsError::configuration("Backup not found"))?;

        let restore_id = format!("restore_{}_{}", backup_id, Utc::now().timestamp());

        info!(
            "Starting restore: {} from backup {} to {}",
            restore_id, backup_id, target_path
        );

        let mut file_count = 0u64;
        let mut total_size = 0u64;

        // 恢复文件清单中的所有文件
        for file_entry in &backup_info.file_manifest {
            if file_entry.backup_status != BackupStatus::Completed {
                warn!(
                    "Skipping file with failed backup status: {}",
                    file_entry.original_path
                );
                continue;
            }

            // 从备份存储读取文件
            let backup_storage = self
                .get_storage_manager(&self.configs[&backup_info.config_name].target_storage)
                .await?;
            let file_data = backup_storage.get_file(&file_entry.backup_path).await?;

            // 计算文件哈希以验证完整性
            let hash = calculate_file_hash(&file_data);
            if hash != file_entry.hash {
                error!(
                    "File hash mismatch for {} (expected: {}, actual: {})",
                    file_entry.original_path, file_entry.hash, hash
                );
                continue;
            }

            // 写入到目标路径
            // 这里需要根据目标路径类型创建相应的存储管理器
            // 简化处理，假设是本地文件系统

            file_count += 1;
            total_size += file_entry.size;
        }

        info!(
            "Restore completed: {} (files: {}, size: {} bytes)",
            restore_id, file_count, total_size
        );

        Ok(restore_id)
    }

    /// 获取存储管理器
    async fn get_storage_manager(&mut self, config: &StorageConfig) -> Result<&StorageManager> {
        let config_key = format!("{:?}", config.storage_type);

        if !self.storage_managers.contains_key(&config_key) {
            let storage_manager = StorageManager::new(config.clone()).await?;
            self.storage_managers.insert(config_key, storage_manager);
        }

        Ok(self.storage_managers.get(&config_key).unwrap())
    }

    /// 查找最新的备份
    fn find_latest_backup(
        &self,
        config_name: &str,
        backup_type: BackupType,
    ) -> Option<&BackupInfo> {
        self.backup_history
            .iter()
            .filter(|b| {
                b.config_name == config_name
                    && b.backup_type == backup_type
                    && b.status == BackupStatus::Completed
            })
            .max_by(|a, b| a.start_time.cmp(&b.start_time))
    }

    /// 清理过期备份
    async fn cleanup_expired_backups(&mut self, config_name: &str) -> Result<()> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| PacsError::configuration("Backup configuration not found"))?;

        let mut backups_to_remove = Vec::new();
        let mut completed_backups: Vec<_> = self
            .backup_history
            .iter()
            .enumerate()
            .filter(|(_, b)| b.config_name == config_name && b.status == BackupStatus::Completed)
            .collect();

        // 按开始时间降序排序
        completed_backups.sort_by(|(_, a), (_, b)| b.start_time.cmp(&a.start_time));

        // 保留最近的N个备份
        if completed_backups.len() > config.retention_count as usize {
            for (index, backup) in completed_backups
                .iter()
                .skip(config.retention_count as usize)
            {
                backups_to_remove.push(*index);
            }
        }

        // 删除过期备份文件
        for &index in &backups_to_remove {
            let backup = &self.backup_history[index];
            info!("Removing expired backup: {}", backup.id);

            // 从目标存储删除备份文件
            if let Ok(target_storage) = self
                .get_storage_manager(&self.configs[config_name].target_storage)
                .await
            {
                for file_entry in &backup.file_manifest {
                    if let Err(e) = target_storage.delete_file(&file_entry.backup_path).await {
                        warn!(
                            "Failed to delete backup file {}: {}",
                            file_entry.backup_path, e
                        );
                    }
                }
            }
        }

        // 从历史记录中移除
        backups_to_remove.sort_by(|a, b| b.cmp(a)); // 降序删除，避免索引变化
        for index in backups_to_remove {
            self.backup_history.remove(index);
        }

        Ok(())
    }

    /// 获取备份列表
    pub fn get_backup_list(&self) -> &[BackupInfo] {
        &self.backup_history
    }

    /// 获取正在进行的备份
    pub fn get_active_backups(&self) -> &HashMap<String, BackupInfo> {
        &self.active_backups
    }

    /// 取消备份
    pub async fn cancel_backup(&mut self, backup_id: &str) -> Result<()> {
        if let Some(active_backup) = self.active_backups.get_mut(backup_id) {
            active_backup.status = BackupStatus::Cancelled;
            active_backup.end_time = Some(Utc::now());
            info!("Backup cancelled: {}", backup_id);
            Ok(())
        } else {
            Err(PacsError::configuration("Backup not found or not active"))
        }
    }

    /// 启动自动备份调度
    pub async fn start_auto_backup(&mut self) -> Result<()> {
        info!("Starting automatic backup scheduling");

        let mut interval = interval(tokio::time::Duration::from_secs(3600)); // 每小时检查一次

        loop {
            interval.tick().await;

            for (config_name, config) in &self.configs {
                // 检查是否有计划备份
                if let Some(_schedule) = &config.schedule {
                    // TODO: 解析cron表达式并检查是否到了备份时间
                    // 这里简化处理，实际应用中可以使用cron库

                    debug!("Checking backup schedule for: {}", config_name);

                    // 检查是否已有正在进行的备份
                    let has_active_backup = self
                        .active_backups
                        .values()
                        .any(|b| b.config_name == *config_name);

                    if !has_active_backup {
                        if let Err(e) = self.execute_backup(config_name, BackupType::Full).await {
                            error!("Auto backup failed for {}: {}", config_name, e);
                        }
                    }
                }
            }
        }
    }

    /// 创建默认备份配置
    pub fn create_default_config(
        source_storage: StorageConfig,
        target_storage: StorageConfig,
    ) -> BackupConfig {
        BackupConfig {
            name: "Default Backup".to_string(),
            backup_type: BackupType::Full,
            source_storage,
            target_storage,
            backup_prefix: "pacs_backup".to_string(),
            schedule: Some("0 2 * * *".to_string()), // 每天凌晨2点
            retention_count: 7,                      // 保留7个备份
            compression_enabled: true,
            encryption_enabled: true,
        }
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 计算文件哈希值
fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
