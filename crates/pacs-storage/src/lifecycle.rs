//! 数据生命周期管理

use crate::storage::{StorageConfig, StorageManager, StorageType};
use chrono::{DateTime, Duration, Utc};
use pacs_core::{PacsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// 生命周期阶段
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleStage {
    /// 在线存储 - 高速访问
    Online,
    /// 近线存储 - 中等访问速度
    Nearline,
    /// 离线归档 - 低成本长期存储
    Archive,
    /// 待删除 - 已过期，等待删除
    PendingDeletion,
}

/// 生命周期策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecyclePolicy {
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 规则列表
    pub rules: Vec<LifecycleRule>,
    /// 是否启用
    pub enabled: bool,
}

/// 生命周期规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 条件过滤器
    pub filter: LifecycleFilter,
    /// 转换操作
    pub transitions: Vec<LifecycleTransition>,
    /// 是否启用
    pub enabled: bool,
}

/// 生命周期过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleFilter {
    /// 文件路径前缀
    pub prefix: Option<String>,
    /// 文件路径后缀
    pub suffix: Option<String>,
    /// 标签过滤
    pub tags: Option<HashMap<String, String>>,
    /// 最小文件大小（字节）
    pub min_size_bytes: Option<u64>,
    /// 最大文件大小（字节）
    pub max_size_bytes: Option<u64>,
}

/// 生命周期转换
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    /// 目标阶段
    pub stage: LifecycleStage,
    /// 转换条件（天数）
    pub days_after_creation: u32,
    /// 目标存储配置
    pub target_storage: Option<StorageConfig>,
}

/// 生命周期状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleStatus {
    /// 文件路径
    pub file_path: String,
    /// 当前阶段
    pub current_stage: LifecycleStage,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed_at: Option<DateTime<Utc>>,
    /// 下次转换时间
    pub next_transition_at: Option<DateTime<Utc>>,
    /// 访问次数
    pub access_count: u64,
}

/// 生命周期管理器
pub struct LifecycleManager {
    /// 存储管理器映射
    storage_managers: HashMap<LifecycleStage, StorageManager>,
    /// 生命周期策略
    policies: Vec<LifecyclePolicy>,
    /// 文件状态缓存
    file_status_cache: HashMap<String, LifecycleStatus>,
    /// 是否启用自动管理
    auto_management_enabled: bool,
}

impl LifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new() -> Self {
        Self {
            storage_managers: HashMap::new(),
            policies: Vec::new(),
            file_status_cache: HashMap::new(),
            auto_management_enabled: true,
        }
    }

    /// 添加存储管理器
    pub fn add_storage_manager(&mut self, stage: LifecycleStage, storage_manager: StorageManager) {
        self.storage_managers.insert(stage, storage_manager);
    }

    /// 添加生命周期策略
    pub fn add_policy(&mut self, policy: LifecyclePolicy) {
        self.policies.push(policy);
    }

    /// 设置自动管理状态
    pub fn set_auto_management(&mut self, enabled: bool) {
        self.auto_management_enabled = enabled;
    }

    /// 注册新文件到生命周期管理
    pub async fn register_file(
        &mut self,
        file_path: &str,
        tags: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let status = LifecycleStatus {
            file_path: file_path.to_string(),
            current_stage: LifecycleStage::Online,
            created_at: Utc::now(),
            last_accessed_at: None,
            next_transition_at: None,
            access_count: 0,
        };

        self.file_status_cache.insert(file_path.to_string(), status);

        info!("Registered file in lifecycle management: {}", file_path);
        Ok(())
    }

    /// 更新文件访问记录
    pub async fn record_access(&mut self, file_path: &str) -> Result<()> {
        if let Some(status) = self.file_status_cache.get_mut(file_path) {
            status.last_accessed_at = Some(Utc::now());
            status.access_count += 1;

            debug!(
                "Recorded access for file: {} (count: {})",
                file_path, status.access_count
            );
        }
        Ok(())
    }

    /// 执行生命周期转换
    pub async fn execute_transitions(&mut self) -> Result<Vec<String>> {
        let mut transitions_executed = Vec::new();
        let now = Utc::now();

        for (file_path, status) in &mut self.file_status_cache {
            // 检查是否需要转换
            if let Some(next_transition) = status.next_transition_at {
                if next_transition <= now {
                    // 执行转换
                    if let Ok(transitioned) = self.execute_file_transition(file_path, status).await
                    {
                        if transitioned {
                            transitions_executed.push(file_path.clone());
                        }
                    }
                }
            } else {
                // 计算下次转换时间
                self.update_next_transition_time(file_path, status).await?;
            }
        }

        if !transitions_executed.is_empty() {
            info!(
                "Executed {} lifecycle transitions",
                transitions_executed.len()
            );
        }

        Ok(transitions_executed)
    }

    /// 执行单个文件的生命周期转换
    async fn execute_file_transition(
        &mut self,
        file_path: &str,
        status: &mut LifecycleStatus,
    ) -> Result<bool> {
        // 获取适用的策略
        let applicable_rules = self.get_applicable_rules(file_path, status)?;

        for rule in applicable_rules {
            for transition in &rule.transitions {
                if self.should_transition(status, transition) {
                    // 执行转换
                    if let Err(e) = self.transition_file(file_path, status, transition).await {
                        error!("Failed to transition file {}: {}", file_path, e);
                        continue;
                    }

                    info!("Transitioned file {} to {:?}", file_path, transition.stage);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 获取适用的生命周期规则
    fn get_applicable_rules(
        &self,
        file_path: &str,
        status: &LifecycleStatus,
    ) -> Result<Vec<&LifecycleRule>> {
        let mut applicable_rules = Vec::new();

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                if !rule.enabled {
                    continue;
                }

                // 检查过滤器条件
                if self.matches_filter(file_path, &rule.filter) {
                    applicable_rules.push(rule);
                }
            }
        }

        // 按优先级排序（这里简单按规则ID排序，实际应用中可以添加优先级字段）
        applicable_rules.sort_by_key(|r| &r.id);

        Ok(applicable_rules)
    }

    /// 检查文件是否匹配过滤器
    fn matches_filter(&self, file_path: &str, filter: &LifecycleFilter) -> bool {
        // 检查前缀
        if let Some(prefix) = &filter.prefix {
            if !file_path.starts_with(prefix) {
                return false;
            }
        }

        // 检查后缀
        if let Some(suffix) = &filter.suffix {
            if !file_path.ends_with(suffix) {
                return false;
            }
        }

        // TODO: 实现其他过滤条件（标签、文件大小等）
        true
    }

    /// 检查是否应该进行转换
    fn should_transition(
        &self,
        status: &LifecycleStatus,
        transition: &LifecycleTransition,
    ) -> bool {
        let days_since_creation = (Utc::now() - status.created_at).num_days() as u32;
        days_since_creation >= transition.days_after_creation
    }

    /// 转换文件到新的存储阶段
    async fn transition_file(
        &mut self,
        file_path: &str,
        status: &mut LifecycleStatus,
        transition: &LifecycleTransition,
    ) -> Result<()> {
        let current_storage = self
            .storage_managers
            .get(&status.current_stage)
            .ok_or_else(|| PacsError::configuration("Current storage stage not configured"))?;

        let target_storage = if let Some(target_config) = &transition.target_storage {
            // 创建新的存储管理器
            StorageManager::new(target_config.clone()).await?
        } else {
            self.storage_managers
                .get(&transition.stage)
                .ok_or_else(|| PacsError::configuration("Target storage stage not configured"))?
                .clone()
        };

        // 读取文件数据
        let file_data = current_storage.get_file(file_path).await?;

        // 存储到目标位置
        let new_path = if transition.target_storage.is_some() {
            // 如果是新的存储配置，可能需要调整路径
            file_path.to_string()
        } else {
            file_path.to_string()
        };

        target_storage.store_file(&file_data, &new_path).await?;

        // 删除原文件（可选，根据策略决定）
        if transition.stage != LifecycleStage::Online {
            // 在实际应用中，这里可能需要更复杂的逻辑来确保数据完整性
            current_storage.delete_file(file_path).await?;
        }

        // 更新状态
        status.current_stage = transition.stage.clone();
        status.next_transition_at = None;

        Ok(())
    }

    /// 更新下次转换时间
    async fn update_next_transition_time(
        &mut self,
        file_path: &str,
        status: &mut LifecycleStatus,
    ) -> Result<()> {
        let applicable_rules = self.get_applicable_rules(file_path, status)?;

        let mut next_time: Option<DateTime<Utc>> = None;

        for rule in applicable_rules {
            for transition in &rule.transitions {
                if transition.stage != status.current_stage {
                    let transition_time =
                        status.created_at + Duration::days(transition.days_after_creation as i64);

                    if next_time.is_none() || transition_time < next_time.unwrap() {
                        next_time = Some(transition_time);
                    }
                }
            }
        }

        status.next_transition_at = next_time;
        Ok(())
    }

    /// 启动自动生命周期管理
    pub async fn start_auto_management(&mut self) -> Result<()> {
        if !self.auto_management_enabled {
            info!("Auto lifecycle management is disabled");
            return Ok(());
        }

        info!("Starting auto lifecycle management");

        let mut interval = interval(tokio::time::Duration::from_secs(3600)); // 每小时检查一次

        loop {
            interval.tick().await;

            if let Err(e) = self.execute_transitions().await {
                error!("Error executing lifecycle transitions: {}", e);
            }

            // 清理过期文件
            if let Err(e) = self.cleanup_expired_files().await {
                error!("Error cleaning up expired files: {}", e);
            }
        }
    }

    /// 清理过期文件
    async fn cleanup_expired_files(&mut self) -> Result<()> {
        let now = Utc::now();
        let mut files_to_remove = Vec::new();

        for (file_path, status) in &self.file_status_cache {
            if status.current_stage == LifecycleStage::PendingDeletion {
                // 检查是否已经过了保留期
                if let Some(transition_time) = status.next_transition_at {
                    if transition_time <= now {
                        files_to_remove.push(file_path.clone());
                    }
                }
            }
        }

        for file_path in files_to_remove {
            if let Err(e) = self.remove_file(&file_path).await {
                error!("Failed to remove expired file {}: {}", file_path, e);
            } else {
                info!("Removed expired file: {}", file_path);
                self.file_status_cache.remove(&file_path);
            }
        }

        Ok(())
    }

    /// 删除文件
    async fn remove_file(&self, file_path: &str) -> Result<()> {
        // 从所有存储阶段删除文件
        for (stage, storage_manager) in &self.storage_managers {
            if storage_manager
                .file_exists(file_path)
                .await
                .unwrap_or(false)
            {
                storage_manager.delete_file(file_path).await?;
                debug!("Removed file from {:?} storage: {}", stage, file_path);
            }
        }
        Ok(())
    }

    /// 获取文件状态
    pub fn get_file_status(&self, file_path: &str) -> Option<&LifecycleStatus> {
        self.file_status_cache.get(file_path)
    }

    /// 获取所有文件状态
    pub fn get_all_file_status(&self) -> &HashMap<String, LifecycleStatus> {
        &self.file_status_cache
    }

    /// 创建默认生命周期策略
    pub fn create_default_policy() -> LifecyclePolicy {
        LifecyclePolicy {
            name: "Default Medical Imaging Policy".to_string(),
            description: "Default lifecycle policy for medical images".to_string(),
            rules: vec![
                LifecycleRule {
                    id: "rule_nearline".to_string(),
                    name: "Move to Nearline after 90 days".to_string(),
                    filter: LifecycleFilter {
                        prefix: None,
                        suffix: Some(".dcm".to_string()),
                        tags: None,
                        min_size_bytes: None,
                        max_size_bytes: None,
                    },
                    transitions: vec![LifecycleTransition {
                        stage: LifecycleStage::Nearline,
                        days_after_creation: 90,
                        target_storage: None,
                    }],
                    enabled: true,
                },
                LifecycleRule {
                    id: "rule_archive".to_string(),
                    name: "Archive after 1 year".to_string(),
                    filter: LifecycleFilter {
                        prefix: None,
                        suffix: Some(".dcm".to_string()),
                        tags: None,
                        min_size_bytes: None,
                        max_size_bytes: None,
                    },
                    transitions: vec![LifecycleTransition {
                        stage: LifecycleStage::Archive,
                        days_after_creation: 365,
                        target_storage: None,
                    }],
                    enabled: true,
                },
                LifecycleRule {
                    id: "rule_delete".to_string(),
                    name: "Delete after 7 years".to_string(),
                    filter: LifecycleFilter {
                        prefix: None,
                        suffix: Some(".dcm".to_string()),
                        tags: None,
                        min_size_bytes: None,
                        max_size_bytes: None,
                    },
                    transitions: vec![LifecycleTransition {
                        stage: LifecycleStage::PendingDeletion,
                        days_after_creation: 2555, // 7 years
                        target_storage: None,
                    }],
                    enabled: true,
                },
            ],
            enabled: true,
        }
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
