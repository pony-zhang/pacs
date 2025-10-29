//! 自动路由规则引擎
//!
//! 根据检查类型和医生专长自动分配任务

use pacs_core::{Result, Study, PacsError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 医生专长
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RadiologistSpecialty {
    General,        // 通用
    Neuroradiology, // 神经放射学
    Musculoskeletal, // 骨骼肌肉
    Cardiac,        // 心脏
    Abdominal,      // 腹部
    Chest,          // 胸部
    Pediatric,      // 儿科
    Breast,         // 乳腺
}

/// 医生信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Radiologist {
    pub id: Uuid,
    pub name: String,
    pub specialties: Vec<RadiologistSpecialty>,
    pub max_workload: i32, // 最大工作负载
    pub is_available: bool,
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: Uuid,
    pub name: String,
    pub priority: i32,
    pub conditions: Vec<RuleCondition>,
    pub action: RuleAction,
    pub is_active: bool,
}

/// 规则条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    ModalityEquals(String),           // 检查类型等于
    ModalityIn(Vec<String>),          // 检查类型在列表中
    DescriptionContains(String),      // 描述包含
    TimeRange(String, String),        // 时间范围
    Emergency,                        // 紧急检查
    Routine,                          // 常规检查
}

/// 规则动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    AssignToRadiologist(Uuid),       // 分配给特定医生
    AssignToSpecialty(RadiologistSpecialty), // 分配给专长组
    QueueInPool(String),             // 加入特定队列
    NotifyAdmin,                     // 通知管理员
}

/// 路由请求
#[derive(Debug, Clone)]
pub struct RoutingRequest {
    pub study: Study,
    pub priority: RoutingPriority,
}

/// 路由优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingPriority {
    Emergency,  // 紧急
    Urgent,     // 急
    Routine,    // 常规
    Low,        // 低优先级
}

/// 路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    pub study_id: Uuid,
    pub assigned_to: Option<Uuid>,
    pub queue_name: Option<String>,
    pub priority: RoutingPriority,
    pub rule_applied: Option<Uuid>,
    pub reason: String,
}

/// 自动路由引擎
#[derive(Debug)]
pub struct RoutingEngine {
    rules: Vec<RoutingRule>,
    radiologists: HashMap<Uuid, Radiologist>,
    workload_map: HashMap<Uuid, i32>, // 当前工作负载
}

impl RoutingEngine {
    /// 创建新的路由引擎
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            radiologists: HashMap::new(),
            workload_map: HashMap::new(),
        }
    }

    /// 添加路由规则
    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.rules.push(rule);
        // 按优先级排序
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 添加医生信息
    pub fn add_radiologist(&mut self, radiologist: Radiologist) {
        self.workload_map.insert(radiologist.id, 0);
        self.radiologists.insert(radiologist.id, radiologist);
    }

    /// 更新医生工作负载
    pub fn update_workload(&mut self, radiologist_id: Uuid, delta: i32) {
        if let Some(workload) = self.workload_map.get_mut(&radiologist_id) {
            *workload += delta;
            if *workload < 0 {
                *workload = 0;
            }
        }
    }

    /// 获取医生当前工作负载
    pub fn get_workload(&self, radiologist_id: Uuid) -> i32 {
        self.workload_map.get(&radiologist_id).copied().unwrap_or(0)
    }

    /// 处理路由请求
    pub fn route_study(&mut self, request: RoutingRequest) -> Result<RoutingResult> {
        tracing::info!("Routing study {} with priority {:?}", request.study.id, request.priority);

        // 遍历规则，找到匹配的第一个规则
        for rule in &self.rules {
            if !rule.is_active {
                continue;
            }

            if self.evaluate_conditions(&rule.conditions, &request.study, &request.priority) {
                return self.apply_action(rule, &request);
            }
        }

        // 如果没有规则匹配，使用默认路由
        self.default_routing(&request)
    }

    /// 评估规则条件
    fn evaluate_conditions(&self, conditions: &[RuleCondition], study: &Study, priority: &RoutingPriority) -> bool {
        for condition in conditions {
            if !self.evaluate_condition(condition, study, priority) {
                return false;
            }
        }
        true
    }

    /// 评估单个条件
    fn evaluate_condition(&self, condition: &RuleCondition, study: &Study, priority: &RoutingPriority) -> bool {
        match condition {
            RuleCondition::ModalityEquals(modality) => study.modality == *modality,
            RuleCondition::ModalityIn(modalities) => modalities.contains(&study.modality),
            RuleCondition::DescriptionContains(keyword) => {
                study.description.as_ref()
                    .map(|desc| desc.to_lowercase().contains(&keyword.to_lowercase()))
                    .unwrap_or(false)
            }
            RuleCondition::Emergency => matches!(priority, RoutingPriority::Emergency),
            RuleCondition::Routine => matches!(priority, RoutingPriority::Routine),
            RuleCondition::TimeRange(_, _) => {
                // TODO: 实现时间范围判断
                true
            }
        }
    }

    /// 应用规则动作
    fn apply_action(&self, rule: &RoutingRule, request: &RoutingRequest) -> Result<RoutingResult> {
        match &rule.action {
            RuleAction::AssignToRadiologist(radiologist_id) => {
                if let Some(radiologist) = self.radiologists.get(radiologist_id) {
                    if radiologist.is_available {
                        Ok(RoutingResult {
                            study_id: request.study.id,
                            assigned_to: Some(*radiologist_id),
                            queue_name: None,
                            priority: request.priority.clone(),
                            rule_applied: Some(rule.id),
                            reason: format!("Assigned to radiologist {} by rule {}", radiologist.name, rule.name),
                        })
                    } else {
                        Err(PacsError::RoutingError("Radiologist is not available".to_string()))
                    }
                } else {
                    Err(PacsError::RoutingError("Radiologist not found".to_string()))
                }
            }
            RuleAction::AssignToSpecialty(specialty) => {
                let best_radiologist = self.find_best_radiologist_for_specialty(specialty);
                Ok(RoutingResult {
                    study_id: request.study.id,
                    assigned_to: best_radiologist,
                    queue_name: None,
                    priority: request.priority.clone(),
                    rule_applied: Some(rule.id),
                    reason: format!("Assigned to specialty {:?} by rule {}", specialty, rule.name),
                })
            }
            RuleAction::QueueInPool(queue_name) => {
                Ok(RoutingResult {
                    study_id: request.study.id,
                    assigned_to: None,
                    queue_name: Some(queue_name.clone()),
                    priority: request.priority.clone(),
                    rule_applied: Some(rule.id),
                    reason: format!("Queued in {} by rule {}", queue_name, rule.name),
                })
            }
            RuleAction::NotifyAdmin => {
                Ok(RoutingResult {
                    study_id: request.study.id,
                    assigned_to: None,
                    queue_name: Some("admin_review".to_string()),
                    priority: request.priority.clone(),
                    rule_applied: Some(rule.id),
                    reason: format!("Admin notification by rule {}", rule.name),
                })
            }
        }
    }

    /// 为特定专长找到最佳医生
    fn find_best_radiologist_for_specialty(&self, specialty: &RadiologistSpecialty) -> Option<Uuid> {
        self.radiologists
            .iter()
            .filter(|(_, radiologist)| {
                radiologist.is_available
                && radiologist.specialties.contains(specialty)
                && self.get_workload(radiologist.id) < radiologist.max_workload
            })
            .min_by_key(|(_, radiologist)| self.get_workload(radiologist.id))
            .map(|(id, _)| *id)
    }

    /// 默认路由逻辑
    fn default_routing(&self, request: &RoutingRequest) -> Result<RoutingResult> {
        // 找到工作量最小的通用放射科医生
        let best_general_radiologist = self.find_best_radiologist_for_specialty(&RadiologistSpecialty::General);

        Ok(RoutingResult {
            study_id: request.study.id,
            assigned_to: best_general_radiologist,
            queue_name: if best_general_radiologist.is_none() { Some("general_pool".to_string()) } else { None },
            priority: request.priority.clone(),
            rule_applied: None,
            reason: "Default routing applied".to_string(),
        })
    }

    /// 获取所有可用医生
    pub fn get_available_radiologists(&self) -> Vec<&Radiologist> {
        self.radiologists
            .values()
            .filter(|r| r.is_available)
            .collect()
    }

    /// 设置医生可用性
    pub fn set_radiologist_availability(&mut self, radiologist_id: Uuid, is_available: bool) {
        if let Some(radiologist) = self.radiologists.get_mut(&radiologist_id) {
            radiologist.is_available = is_available;
        }
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}