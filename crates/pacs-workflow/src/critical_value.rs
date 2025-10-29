//! 危急值处理流程
//!
//! 确保紧急情况能够及时通知相关人员

use pacs_core::{Result, PacsError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 危急值类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CriticalValueType {
    LifeThreatening,    // 威胁生命
    Emergency,          // 紧急
    Urgent,             // 急诊
    Critical,           // 危重
}

/// 危急值事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalValueEvent {
    pub id: Uuid,
    pub study_id: Uuid,
    pub patient_id: Uuid,
    pub value_type: CriticalValueType,
    pub description: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub detected_by: Uuid, // 发现危急值的用户ID
    pub severity: CriticalSeverity,
    pub clinical_context: Option<String>,
}

/// 危急值严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CriticalSeverity {
    Low,        // 低
    Medium,     // 中
    High,       // 高
    Critical,   // 危重
}

/// 通知方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationMethod {
    InApp,          // 应用内通知
    Email,          // 邮件
    SMS,            // 短信
    PhoneCall,      // 电话
    Pager,          // 寻呼机
}

/// 通知记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub event_id: Uuid,
    pub recipient_id: Uuid,
    pub method: NotificationMethod,
    pub sent_at: chrono::DateTime<chrono::Utc>,
    pub status: NotificationStatus,
    pub retry_count: i32,
    pub error_message: Option<String>,
}

/// 通知状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,     // 待发送
    Sent,        // 已发送
    Delivered,   // 已送达
    Read,        // 已读
    Acknowledged,// 已确认
    Failed,      // 发送失败
}

/// 危急值处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalValuePolicy {
    pub id: Uuid,
    pub name: String,
    pub value_types: Vec<CriticalValueType>,
    pub notification_rules: Vec<NotificationRule>,
    pub escalation_rules: Vec<EscalationRule>,
    pub is_active: bool,
}

/// 通知规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRule {
    pub recipient_type: RecipientType,
    pub recipient_id: Option<Uuid>, // 特定接收者ID
    pub methods: Vec<NotificationMethod>,
    pub delay_minutes: i32,
    pub require_acknowledgment: bool,
}

/// 升级规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    pub condition: EscalationCondition,
    pub action: EscalationAction,
    pub trigger_after_minutes: i32,
}

/// 升级条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationCondition {
    NotAcknowledged,         // 未确认
    NotDelivered,           // 未送达
    NoResponse,             // 无响应
    RecipientUnavailable,   // 接收者不可用
}

/// 升级动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationAction {
    NotifyBackupRecipient,  // 通知备用接收者
    IncreaseSeverity,       // 提高严重程度
    AddNotificationMethod,  // 添加通知方式
    NotifyAdmin,           // 通知管理员
}

/// 接收者类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecipientType {
    ReferringPhysician,     // 开单医生
    PrimaryRadiologist,     // 主要放射科医生
    DepartmentHead,         // 科室主任
    EmergencyRoom,          // 急诊科
    BackupRadiologist,      // 备用放射科医生
    SystemAdmin,           // 系统管理员
    SpecificUser(Uuid),     // 特定用户
}

/// 危急值处理器
#[derive(Debug)]
pub struct CriticalValueProcessor {
    events: HashMap<Uuid, CriticalValueEvent>,
    notifications: HashMap<Uuid, Vec<NotificationRecord>>,
    policies: Vec<CriticalValuePolicy>,
    notification_queue: Vec<NotificationRecord>,
}

impl CriticalValueProcessor {
    /// 创建新的危急值处理器
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            notifications: HashMap::new(),
            policies: Vec::new(),
            notification_queue: Vec::new(),
        }
    }

    /// 添加危急值策略
    pub fn add_policy(&mut self, policy: CriticalValuePolicy) {
        self.policies.push(policy);
    }

    /// 创建危急值事件
    pub fn create_critical_value_event(
        &mut self,
        study_id: Uuid,
        patient_id: Uuid,
        value_type: CriticalValueType,
        description: String,
        detected_by: Uuid,
        severity: CriticalSeverity,
        clinical_context: Option<String>,
    ) -> Result<CriticalValueEvent> {
        let event = CriticalValueEvent {
            id: Uuid::new_v4(),
            study_id,
            patient_id,
            value_type,
            description,
            detected_at: chrono::Utc::now(),
            detected_by,
            severity,
            clinical_context,
        };

        let event_id = event.id;
        self.events.insert(event_id, event.clone());

        tracing::warn!("Critical value event created: {} for study {}", event_id, study_id);

        // 立即开始处理通知
        self.process_critical_value_event(&event)?;

        Ok(event)
    }

    /// 处理危急值事件
    fn process_critical_value_event(&mut self, event: &CriticalValueEvent) -> Result<()> {
        // 找到匹配的策略
        let matching_policies: Vec<_> = self.policies
            .iter()
            .filter(|policy| policy.is_active && policy.value_types.contains(&event.value_type))
            .collect();

        if matching_policies.is_empty() {
            tracing::warn!("No matching policy found for critical value event {}", event.id);
            return Ok(());
        }

        // 应用所有匹配的策略
        for policy in matching_policies {
            for rule in &policy.notification_rules {
                self.create_notification(event, rule)?;
            }
        }

        Ok(())
    }

    /// 创建通知
    fn create_notification(&mut self, event: &CriticalValueEvent, rule: &NotificationRule) -> Result<()> {
        let recipient_id = match &rule.recipient_type {
            RecipientType::SpecificUser(id) => Some(*id),
            // TODO: 其他接收者类型需要查询相关数据库
            _ => {
                tracing::warn!("Recipient type {:?} not implemented yet", rule.recipient_type);
                return Ok(());
            }
        };

        if let Some(recipient_id) = recipient_id {
            for method in &rule.methods {
                let notification = NotificationRecord {
                    id: Uuid::new_v4(),
                    event_id: event.id,
                    recipient_id,
                    method: method.clone(),
                    sent_at: chrono::Utc::now(),
                    status: NotificationStatus::Pending,
                    retry_count: 0,
                    error_message: None,
                };

                self.notifications
                    .entry(event.id)
                    .or_insert_with(Vec::new)
                    .push(notification.clone());

                self.notification_queue.push(notification);
            }
        }

        Ok(())
    }

    /// 处理通知队列
    pub async fn process_notification_queue(&mut self) -> Result<()> {
        let mut notifications_to_process = Vec::new();
        std::mem::swap(&mut notifications_to_process, &mut self.notification_queue);

        for mut notification in notifications_to_process {
            match self.send_notification(&notification).await {
                Ok(_) => {
                    notification.status = NotificationStatus::Sent;
                    tracing::info!("Notification {} sent successfully", notification.id);
                }
                Err(e) => {
                    notification.status = NotificationStatus::Failed;
                    notification.error_message = Some(e.to_string());
                    notification.retry_count += 1;

                    tracing::error!("Failed to send notification {}: {}", notification.id, e);

                    // 如果重试次数少于3次，重新加入队列
                    if notification.retry_count < 3 {
                        self.notification_queue.push(notification.clone());
                    }
                }
            }

            // 更新通知记录
            if let Some(notifications) = self.notifications.get_mut(&notification.event_id) {
                if let Some(pos) = notifications.iter().position(|n| n.id == notification.id) {
                    notifications[pos] = notification;
                }
            }
        }

        Ok(())
    }

    /// 发送通知
    async fn send_notification(&self, notification: &NotificationRecord) -> Result<()> {
        // TODO: 实现实际的通知发送逻辑
        match notification.method {
            NotificationMethod::InApp => {
                // 应用内通知逻辑
                tracing::info!("Sending in-app notification to user {}", notification.recipient_id);
            }
            NotificationMethod::Email => {
                // 邮件通知逻辑
                tracing::info!("Sending email notification to user {}", notification.recipient_id);
            }
            NotificationMethod::SMS => {
                // 短信通知逻辑
                tracing::info!("Sending SMS notification to user {}", notification.recipient_id);
            }
            NotificationMethod::PhoneCall => {
                // 电话通知逻辑
                tracing::info!("Making phone call to user {}", notification.recipient_id);
            }
            NotificationMethod::Pager => {
                // 寻呼机通知逻辑
                tracing::info!("Sending pager notification to user {}", notification.recipient_id);
            }
        }

        // 模拟异步发送
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// 确认危急值事件
    pub fn acknowledge_critical_value(&mut self, event_id: Uuid, user_id: Uuid) -> Result<()> {
        if let Some(notifications) = self.notifications.get_mut(&event_id) {
            for notification in notifications {
                if notification.recipient_id == user_id {
                    notification.status = NotificationStatus::Acknowledged;
                    tracing::info!("Critical value {} acknowledged by user {}", event_id, user_id);
                    return Ok(());
                }
            }
        }

        Err(PacsError::NotFound(format!("No notification found for user {} in event {}", user_id, event_id)))
    }

    /// 获取危急值事件
    pub fn get_critical_value_event(&self, event_id: Uuid) -> Option<&CriticalValueEvent> {
        self.events.get(&event_id)
    }

    /// 获取事件的通知记录
    pub fn get_event_notifications(&self, event_id: Uuid) -> Option<&Vec<NotificationRecord>> {
        self.notifications.get(&event_id)
    }

    /// 获取未确认的危急值事件
    pub fn get_unacknowledged_events(&self) -> Vec<&CriticalValueEvent> {
        self.events
            .values()
            .filter(|event| {
                if let Some(notifications) = self.notifications.get(&event.id) {
                    !notifications.iter().any(|n| matches!(n.status, NotificationStatus::Acknowledged))
                } else {
                    true
                }
            })
            .collect()
    }

    /// 获取用户的危急值通知
    pub fn get_user_critical_notifications(&self, user_id: Uuid) -> Vec<&NotificationRecord> {
        self.notifications
            .values()
            .flatten()
            .filter(|notification| notification.recipient_id == user_id)
            .collect()
    }

    /// 检查是否需要升级
    pub fn check_escalations(&mut self) -> Result<Vec<EscalationAction>> {
        let mut escalations = Vec::new();
        let now = chrono::Utc::now();

        for (event_id, notifications) in &self.notifications {
            if let Some(event) = self.events.get(event_id) {
                for policy in &self.policies {
                    if !policy.is_active || !policy.value_types.contains(&event.value_type) {
                        continue;
                    }

                    for escalation_rule in &policy.escalation_rules {
                        let time_since_detection = now.signed_duration_since(event.detected_at);
                        let minutes_passed = time_since_detection.num_minutes();

                        if minutes_passed >= escalation_rule.trigger_after_minutes {
                            if self.should_escalate(notifications, &escalation_rule.condition) {
                                escalations.push(escalation_rule.action.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(escalations)
    }

    /// 判断是否需要升级
    fn should_escalate(&self, notifications: &[NotificationRecord], condition: &EscalationCondition) -> bool {
        match condition {
            EscalationCondition::NotAcknowledged => {
                !notifications.iter().any(|n| matches!(n.status, NotificationStatus::Acknowledged))
            }
            EscalationCondition::NotDelivered => {
                !notifications.iter().any(|n| matches!(n.status, NotificationStatus::Delivered | NotificationStatus::Read | NotificationStatus::Acknowledged))
            }
            EscalationCondition::NoResponse => {
                // TODO: 实现更复杂的响应检测逻辑
                notifications.iter().all(|n| matches!(n.status, NotificationStatus::Sent))
            }
            EscalationCondition::RecipientUnavailable => {
                // TODO: 实现接收者可用性检查
                false
            }
        }
    }
}

impl Default for CriticalValueProcessor {
    fn default() -> Self {
        Self::new()
    }
}