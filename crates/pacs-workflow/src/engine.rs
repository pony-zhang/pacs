//! 工作流引擎
//!
//! 协调状态机、路由、工作列表和危急值处理的核心引擎

use crate::{
    critical_value::{CriticalValueProcessor, CriticalValueType, CriticalSeverity},
    routing::{RoutingEngine, RoutingRequest, RoutingPriority},
    state_machine::{StudyStateMachine, StudyEvent},
    worklist::{WorkListManager, WorkItemPriority, WorkItemStatus},
};
use pacs_core::{Result, Study, StudyStatus};
use std::collections::HashMap;
use uuid::Uuid;

/// 工作流引擎
///
/// 协调所有工作流组件，提供统一的工作流管理接口
#[derive(Debug)]
pub struct WorkflowEngine {
    state_machine: StudyStateMachine,
    routing_engine: RoutingEngine,
    worklist_manager: WorkListManager,
    critical_processor: CriticalValueProcessor,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new() -> Self {
        Self {
            state_machine: StudyStateMachine::new(),
            routing_engine: RoutingEngine::new(),
            worklist_manager: WorkListManager::new(),
            critical_processor: CriticalValueProcessor::new(),
        }
    }

    /// 处理新的检查
    pub async fn process_new_study(&mut self, study: Study, priority: RoutingPriority) -> Result<()> {
        tracing::info!("Processing new study {} with priority {:?}", study.id, priority);

        // 1. 状态机处理 - 确保状态正确
        let current_status = &study.status;
        if current_status != &StudyStatus::Scheduled {
            // 如果不是预定状态，尝试转换
            if self.state_machine.can_transition(current_status, &StudyEvent::Scheduled) {
                // 这里应该更新数据库中的状态
                tracing::debug!("Study {} status transitioned to Scheduled", study.id);
            }
        }

        // 2. 自动路由
        let routing_request = RoutingRequest {
            study: study.clone(),
            priority: priority.clone(),
        };

        let routing_result = self.routing_engine.route_study(routing_request)?;
        tracing::info!("Study {} routed: {:?}", study.id, routing_result);

        // 3. 创建工作项
        if let Some(radiologist_id) = routing_result.assigned_to {
            let work_item = self.worklist_manager.create_work_item(
                study.id,
                Some(radiologist_id),
                self.map_priority_to_worklist(priority),
                30, // 预估30分钟
                vec![study.modality.clone()],
                None, // 无截止时间
            )?;

            tracing::info!("Created work item {} for study {} assigned to {}",
                work_item.id, study.id, radiologist_id);

            // 更新医生工作负载
            self.routing_engine.update_workload(radiologist_id, 1);
        } else if let Some(queue_name) = routing_result.queue_name {
            let work_item = self.worklist_manager.create_work_item(
                study.id,
                None,
                self.map_priority_to_worklist(priority),
                30,
                vec![study.modality.clone(), queue_name.clone()],
                None,
            )?;

            tracing::info!("Created work item {} for study {} in queue {}",
                work_item.id, study.id, queue_name);
        }

        Ok(())
    }

    /// 更新检查状态
    pub async fn update_study_status(
        &mut self,
        study_id: Uuid,
        current_status: StudyStatus,
        event: StudyEvent,
    ) -> Result<StudyStatus> {
        tracing::info!("Updating study {} status with event {:?}", study_id, event);

        // 1. 验证状态转换
        let new_status = self.state_machine.transition(&current_status, &event)?;

        // 2. 更新相关工作项
        let work_item_ids: Vec<Uuid> = self.worklist_manager.get_study_work_items(study_id)
            .iter()
            .map(|item| item.id)
            .collect();

        for work_item_id in work_item_ids {
            // 获取工作项的详细信息
            if let Some(work_item) = self.worklist_manager.get_work_item(work_item_id) {
                let radiologist_id = work_item.radiologist_id;

                match event {
                    StudyEvent::Started => {
                        self.worklist_manager.update_work_item_status(work_item_id, WorkItemStatus::InProgress)?;
                    }
                    StudyEvent::Completed => {
                        self.worklist_manager.update_work_item_status(work_item_id, WorkItemStatus::Completed)?;

                        // 减少医生工作负载
                        if let Some(radiologist_id) = radiologist_id {
                            self.routing_engine.update_workload(radiologist_id, -1);
                        }
                    }
                    StudyEvent::Canceled => {
                        self.worklist_manager.update_work_item_status(work_item_id, WorkItemStatus::Rejected)?;

                        // 减少医生工作负载
                        if let Some(radiologist_id) = radiologist_id {
                            self.routing_engine.update_workload(radiologist_id, -1);
                        }
                    }
                    _ => {}
                }
            }
        }

        tracing::info!("Study {} status updated from {:?} to {:?}", study_id, current_status, new_status);
        Ok(new_status)
    }

    /// 创建危急值事件
    pub async fn create_critical_value(
        &mut self,
        study_id: Uuid,
        patient_id: Uuid,
        value_type: CriticalValueType,
        description: String,
        detected_by: Uuid,
        severity: CriticalSeverity,
        clinical_context: Option<String>,
    ) -> Result<()> {
        tracing::warn!("Creating critical value for study {} with severity {:?}", study_id, severity);

        let _event = self.critical_processor.create_critical_value_event(
            study_id,
            patient_id,
            value_type,
            description,
            detected_by,
            severity.clone(),
            clinical_context,
        )?;

        // 立即处理通知队列
        self.critical_processor.process_notification_queue().await?;

        // 如果是高危紧急情况，可能需要自动提高路由优先级
        if matches!(severity, CriticalSeverity::Critical | CriticalSeverity::High) {
            // TODO: 实现紧急路由逻辑
            tracing::warn!("High severity critical value detected - urgent routing required");
        }

        Ok(())
    }

    /// 获取放射科医生的工作列表
    pub fn get_radiologist_worklist(&self, radiologist_id: Uuid) -> Result<Vec<crate::worklist::WorkItem>> {
        self.worklist_manager.get_radiologist_worklist(radiologist_id)
    }

    /// 获取工作列表统计
    pub fn get_worklist_stats(&self, radiologist_id: Option<Uuid>) -> crate::worklist::WorkListStats {
        self.worklist_manager.get_worklist_stats(radiologist_id)
    }

    /// 获取未确认的危急值事件
    pub fn get_unacknowledged_critical_values(&self) -> Vec<&crate::critical_value::CriticalValueEvent> {
        self.critical_processor.get_unacknowledged_events()
    }

    /// 确认危急值
    pub fn acknowledge_critical_value(&mut self, event_id: Uuid, user_id: Uuid) -> Result<()> {
        self.critical_processor.acknowledge_critical_value(event_id, user_id)
    }

    /// 处理通知队列
    pub async fn process_notifications(&mut self) -> Result<()> {
        self.critical_processor.process_notification_queue().await
    }

    /// 检查升级条件
    pub fn check_escalations(&mut self) -> Result<Vec<crate::critical_value::EscalationAction>> {
        self.critical_processor.check_escalations()
    }

    /// 获取状态机实例
    pub fn state_machine(&self) -> &StudyStateMachine {
        &self.state_machine
    }

    /// 获取路由引擎实例
    pub fn routing_engine(&self) -> &RoutingEngine {
        &self.routing_engine
    }

    /// 获取工作列表管理器实例
    pub fn worklist_manager(&self) -> &WorkListManager {
        &self.worklist_manager
    }

    /// 获取危急值处理器实例
    pub fn critical_processor(&self) -> &CriticalValueProcessor {
        &self.critical_processor
    }

    /// 获取可变路由引擎实例
    pub fn routing_engine_mut(&mut self) -> &mut RoutingEngine {
        &mut self.routing_engine
    }

    /// 获取可变工作列表管理器实例
    pub fn worklist_manager_mut(&mut self) -> &mut WorkListManager {
        &mut self.worklist_manager
    }

    /// 获取可变危急值处理器实例
    pub fn critical_processor_mut(&mut self) -> &mut CriticalValueProcessor {
        &mut self.critical_processor
    }

    /// 映射路由优先级到工作列表优先级
    fn map_priority_to_worklist(&self, routing_priority: RoutingPriority) -> WorkItemPriority {
        match routing_priority {
            RoutingPriority::Emergency => WorkItemPriority::Critical,
            RoutingPriority::Urgent => WorkItemPriority::High,
            RoutingPriority::Routine => WorkItemPriority::Normal,
            RoutingPriority::Low => WorkItemPriority::Low,
        }
    }

    /// 手动分配工作项
    pub fn assign_work_item(&mut self, work_item_id: Uuid, radiologist_id: Uuid) -> Result<()> {
        self.worklist_manager.assign_work_item(work_item_id, radiologist_id)?;
        self.routing_engine.update_workload(radiologist_id, 1);
        Ok(())
    }

    /// 更新工作项状态
    pub fn update_work_item_status(&mut self, work_item_id: Uuid, status: WorkItemStatus) -> Result<()> {
        // 获取工作项信息
        let radiologist_id = if let Some(work_item) = self.worklist_manager.get_work_item(work_item_id) {
            let old_status = work_item.status.clone();
            let radiologist_id = work_item.radiologist_id;

            self.worklist_manager.update_work_item_status(work_item_id, status.clone())?;

            // 更新医生工作负载
            if let Some(radiologist_id) = radiologist_id {
                let workload_delta = match (&old_status, &status) {
                    (WorkItemStatus::Pending, WorkItemStatus::InProgress) => 0,
                    (WorkItemStatus::InProgress, WorkItemStatus::Completed) => -1,
                    (WorkItemStatus::InProgress, WorkItemStatus::Rejected) => -1,
                    (WorkItemStatus::Pending, WorkItemStatus::Rejected) => -1,
                    _ => 0,
                };

                if workload_delta != 0 {
                    self.routing_engine.update_workload(radiologist_id, workload_delta);
                }
            }
        } else {
            return Err(pacs_core::PacsError::NotFound(format!("Work item {} not found", work_item_id)));
        }

        Ok(())
    }

    /// 获取系统概览
    pub fn get_system_overview(&self) -> WorkflowSystemOverview {
        let active_work_items = self.worklist_manager.get_all_active_work_items();
        let unacknowledged_critical = self.critical_processor.get_unacknowledged_events();
        let available_radiologists = self.routing_engine.get_available_radiologists();

        WorkflowSystemOverview {
            total_active_work_items: active_work_items.len(),
            total_unacknowledged_critical_values: unacknowledged_critical.len(),
            available_radiologists_count: available_radiologists.len(),
            system_load: self.calculate_system_load(&active_work_items, &available_radiologists),
        }
    }

    /// 计算系统负载
    fn calculate_system_load(&self, work_items: &[&crate::worklist::WorkItem], radiologists: &[&crate::routing::Radiologist]) -> f64 {
        if radiologists.is_empty() {
            return 1.0; // 无可用医生时负载为100%
        }

        let total_capacity: i32 = radiologists.iter().map(|r| r.max_workload).sum();
        let current_workload: i32 = work_items.iter().map(|item| {
            if let Some(radiologist_id) = item.radiologist_id {
                self.routing_engine.get_workload(radiologist_id)
            } else {
                0
            }
        }).sum();

        if total_capacity == 0 {
            1.0
        } else {
            (current_workload as f64) / (total_capacity as f64)
        }
    }
}

/// 系统概览
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSystemOverview {
    pub total_active_work_items: usize,
    pub total_unacknowledged_critical_values: usize,
    pub available_radiologists_count: usize,
    pub system_load: f64,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
