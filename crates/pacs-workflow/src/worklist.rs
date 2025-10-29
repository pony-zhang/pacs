//! 工作列表管理
//!
//! 为不同角色用户提供个性化的任务列表

use pacs_core::{Result, PacsError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 工作项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: Uuid,
    pub study_id: Uuid,
    pub radiologist_id: Option<Uuid>,
    pub status: WorkItemStatus,
    pub priority: WorkItemPriority,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub due_at: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_duration_minutes: i32,
    pub tags: Vec<String>,
}

/// 工作项状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkItemStatus {
    Pending,        // 待处理
    InProgress,     // 处理中
    Completed,      // 已完成
    Rejected,       // 已拒绝
    OnHold,         // 暂停
}

/// 工作项优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkItemPriority {
    Critical,       // 危急
    High,          // 高
    Normal,        // 正常
    Low,           // 低
}

/// 工作列表过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkListFilter {
    pub radiologist_id: Option<Uuid>,
    pub status: Option<Vec<WorkItemStatus>>,
    pub priority: Option<Vec<WorkItemPriority>>,
    pub modality: Option<Vec<String>>,
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl Default for WorkListFilter {
    fn default() -> Self {
        Self {
            radiologist_id: None,
            status: None,
            priority: None,
            modality: None,
            date_from: None,
            date_to: None,
            tags: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// 工作列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkList {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<WorkItem>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 工作列表统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkListStats {
    pub total_items: i32,
    pub pending_items: i32,
    pub in_progress_items: i32,
    pub completed_items: i32,
    pub overdue_items: i32,
    pub average_processing_time_minutes: f64,
    pub workload_by_priority: HashMap<WorkItemPriority, i32>,
}

/// 工作列表管理器
#[derive(Debug)]
pub struct WorkListManager {
    work_items: HashMap<Uuid, WorkItem>,
    radiologist_worklists: HashMap<Uuid, Vec<Uuid>>, // radiologist_id -> work_item_ids
    study_work_items: HashMap<Uuid, Vec<Uuid>>, // study_id -> work_item_ids
}

impl WorkListManager {
    /// 创建新的工作列表管理器
    pub fn new() -> Self {
        Self {
            work_items: HashMap::new(),
            radiologist_worklists: HashMap::new(),
            study_work_items: HashMap::new(),
        }
    }

    /// 创建工作项
    pub fn create_work_item(
        &mut self,
        study_id: Uuid,
        radiologist_id: Option<Uuid>,
        priority: WorkItemPriority,
        estimated_duration_minutes: i32,
        tags: Vec<String>,
        due_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<WorkItem> {
        let work_item = WorkItem {
            id: Uuid::new_v4(),
            study_id,
            radiologist_id,
            status: WorkItemStatus::Pending,
            priority,
            assigned_at: chrono::Utc::now(),
            due_at,
            estimated_duration_minutes,
            tags,
        };

        let work_item_id = work_item.id;

        // 存储工作项
        self.work_items.insert(work_item_id, work_item.clone());

        // 更新放射科医生工作列表
        if let Some(radiologist_id) = radiologist_id {
            self.radiologist_worklists
                .entry(radiologist_id)
                .or_insert_with(Vec::new)
                .push(work_item_id);
        }

        // 更新检查工作项映射
        self.study_work_items
            .entry(study_id)
            .or_insert_with(Vec::new)
            .push(work_item_id);

        tracing::info!("Created work item {} for study {}", work_item_id, study_id);
        Ok(work_item)
    }

    /// 获取工作项
    pub fn get_work_item(&self, work_item_id: Uuid) -> Option<&WorkItem> {
        self.work_items.get(&work_item_id)
    }

    /// 更新工作项状态
    pub fn update_work_item_status(&mut self, work_item_id: Uuid, status: WorkItemStatus) -> Result<()> {
        if let Some(work_item) = self.work_items.get_mut(&work_item_id) {
            let old_status = work_item.status.clone();
            work_item.status = status.clone();

            tracing::info!("Updated work item {} status from {:?} to {:?}",
                work_item_id, old_status, status);

            // 如果工作项被拒绝或完成，可以从放射科医生的活跃列表中移除
            if matches!(status, WorkItemStatus::Completed | WorkItemStatus::Rejected) {
                if let Some(radiologist_id) = work_item.radiologist_id {
                    if let Some(worklist) = self.radiologist_worklists.get_mut(&radiologist_id) {
                        worklist.retain(|&id| id != work_item_id);
                    }
                }
            }

            Ok(())
        } else {
            Err(PacsError::NotFound(format!("Work item {} not found", work_item_id)))
        }
    }

    /// 分配工作项给放射科医生
    pub fn assign_work_item(&mut self, work_item_id: Uuid, radiologist_id: Uuid) -> Result<()> {
        if let Some(work_item) = self.work_items.get_mut(&work_item_id) {
            let old_radiologist = work_item.radiologist_id;

            // 从旧放射科医生的列表中移除
            if let Some(old_id) = old_radiologist {
                if let Some(worklist) = self.radiologist_worklists.get_mut(&old_id) {
                    worklist.retain(|&id| id != work_item_id);
                }
            }

            // 更新工作项
            work_item.radiologist_id = Some(radiologist_id);
            work_item.assigned_at = chrono::Utc::now();

            // 添加到新放射科医生的列表
            self.radiologist_worklists
                .entry(radiologist_id)
                .or_insert_with(Vec::new)
                .push(work_item_id);

            tracing::info!("Assigned work item {} to radiologist {}", work_item_id, radiologist_id);
            Ok(())
        } else {
            Err(PacsError::NotFound(format!("Work item {} not found", work_item_id)))
        }
    }

    /// 查询工作列表
    pub fn query_worklist(&self, filter: &WorkListFilter) -> Result<Vec<WorkItem>> {
        let mut items: Vec<&WorkItem> = self.work_items.values().collect();

        // 应用过滤器
        if let Some(radiologist_id) = filter.radiologist_id {
            items.retain(|item| item.radiologist_id == Some(radiologist_id));
        }

        if let Some(statuses) = &filter.status {
            items.retain(|item| statuses.contains(&item.status));
        }

        if let Some(priorities) = &filter.priority {
            items.retain(|item| priorities.contains(&item.priority));
        }

        // 按优先级和创建时间排序
        items.sort_by(|a, b| {
            match b.priority.cmp(&a.priority) {
                std::cmp::Ordering::Equal => a.assigned_at.cmp(&b.assigned_at),
                other => other,
            }
        });

        // 应用分页
        let offset = filter.offset.unwrap_or(0) as usize;
        let limit = filter.limit.unwrap_or(50) as usize;

        let total_items = items.len();
        let start = offset.min(total_items);
        let end = (start + limit).min(total_items);

        Ok(items[start..end].iter().map(|item| (*item).clone()).collect())
    }

    /// 获取放射科医生的工作列表
    pub fn get_radiologist_worklist(&self, radiologist_id: Uuid) -> Result<Vec<WorkItem>> {
        let filter = WorkListFilter {
            radiologist_id: Some(radiologist_id),
            ..Default::default()
        };
        self.query_worklist(&filter)
    }

    /// 获取检查的工作项
    pub fn get_study_work_items(&self, study_id: Uuid) -> Vec<&WorkItem> {
        self.study_work_items
            .get(&study_id)
            .map(|ids| ids.iter().filter_map(|&id| self.work_items.get(&id)).collect())
            .unwrap_or_default()
    }

    /// 获取工作列表统计
    pub fn get_worklist_stats(&self, radiologist_id: Option<Uuid>) -> WorkListStats {
        let filter = WorkListFilter {
            radiologist_id,
            ..Default::default()
        };

        let items = match self.query_worklist(&filter) {
            Ok(items) => items,
            Err(_) => Vec::new(),
        };

        let mut stats = WorkListStats {
            total_items: items.len() as i32,
            pending_items: 0,
            in_progress_items: 0,
            completed_items: 0,
            overdue_items: 0,
            average_processing_time_minutes: 0.0,
            workload_by_priority: HashMap::new(),
        };

        let now = chrono::Utc::now();
        let mut completed_items = Vec::new();

        for item in &items {
            match item.status {
                WorkItemStatus::Pending => stats.pending_items += 1,
                WorkItemStatus::InProgress => stats.in_progress_items += 1,
                WorkItemStatus::Completed => {
                    stats.completed_items += 1;
                    completed_items.push(item);
                }
                _ => {}
            }

            // 检查是否过期
            if let Some(due_at) = item.due_at {
                if now > due_at && !matches!(item.status, WorkItemStatus::Completed) {
                    stats.overdue_items += 1;
                }
            }

            // 统计按优先级分布
            *stats.workload_by_priority.entry(item.priority.clone()).or_insert(0) += 1;
        }

        // 计算平均处理时间
        if !completed_items.is_empty() {
            let total_duration: i64 = completed_items
                .iter()
                .map(|item| {
                    // 这里简化处理，实际应该记录完成时间
                    item.estimated_duration_minutes as i64
                })
                .sum();
            stats.average_processing_time_minutes = total_duration as f64 / completed_items.len() as f64;
        }

        stats
    }

    /// 删除工作项
    pub fn remove_work_item(&mut self, work_item_id: Uuid) -> Result<()> {
        if let Some(work_item) = self.work_items.remove(&work_item_id) {
            // 从放射科医生工作列表中移除
            if let Some(radiologist_id) = work_item.radiologist_id {
                if let Some(worklist) = self.radiologist_worklists.get_mut(&radiologist_id) {
                    worklist.retain(|&id| id != work_item_id);
                }
            }

            // 从检查工作项映射中移除
            if let Some(work_items) = self.study_work_items.get_mut(&work_item.study_id) {
                work_items.retain(|&id| id != work_item_id);
            }

            tracing::info!("Removed work item {}", work_item_id);
            Ok(())
        } else {
            Err(PacsError::NotFound(format!("Work item {} not found", work_item_id)))
        }
    }

    /// 获取所有活跃工作项
    pub fn get_all_active_work_items(&self) -> Vec<&WorkItem> {
        self.work_items
            .values()
            .filter(|item| matches!(item.status, WorkItemStatus::Pending | WorkItemStatus::InProgress))
            .collect()
    }
}

impl Default for WorkListManager {
    fn default() -> Self {
        Self::new()
    }
}