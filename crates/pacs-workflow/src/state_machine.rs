//! 检查状态机
//!
//! 管理影像检查的完整生命周期状态转换

use pacs_core::{Result, PacsError, StudyStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 检查状态转换事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StudyEvent {
    Scheduled,
    Started,
    Completed,
    PreliminaryReport,
    FinalReport,
    Canceled,
}

/// 状态机转换规则
#[derive(Debug)]
pub struct StateTransition {
    from: StudyStatus,
    event: StudyEvent,
    to: StudyStatus,
}

/// 检查状态机
#[derive(Debug)]
pub struct StudyStateMachine {
    transitions: HashMap<(StudyStatus, StudyEvent), StudyStatus>,
}

impl StudyStateMachine {
    /// 创建新的状态机实例
    pub fn new() -> Self {
        let mut transitions = HashMap::new();

        // 定义状态转换规则
        transitions.insert((StudyStatus::Scheduled, StudyEvent::Started), StudyStatus::InProgress);
        transitions.insert((StudyStatus::InProgress, StudyEvent::Completed), StudyStatus::Completed);
        transitions.insert((StudyStatus::Completed, StudyEvent::PreliminaryReport), StudyStatus::Preliminary);
        transitions.insert((StudyStatus::Preliminary, StudyEvent::FinalReport), StudyStatus::Final);
        transitions.insert((StudyStatus::Scheduled, StudyEvent::Canceled), StudyStatus::Canceled);
        transitions.insert((StudyStatus::InProgress, StudyEvent::Canceled), StudyStatus::Canceled);

        Self { transitions }
    }

    /// 检查状态转换是否有效
    pub fn can_transition(&self, from: &StudyStatus, event: &StudyEvent) -> bool {
        self.transitions.contains_key(&(from.clone(), event.clone()))
    }

    /// 执行状态转换
    pub fn transition(&self, from: &StudyStatus, event: &StudyEvent) -> Result<StudyStatus> {
        match self.transitions.get(&(from.clone(), event.clone())) {
            Some(to) => Ok(to.clone()),
            None => Err(PacsError::InvalidStateTransition {
                from: format!("{:?}", from),
                event: format!("{:?}", event),
            }),
        }
    }

    /// 获取所有可能的状态
    pub fn get_all_states() -> Vec<StudyStatus> {
        vec![
            StudyStatus::Scheduled,
            StudyStatus::InProgress,
            StudyStatus::Completed,
            StudyStatus::Preliminary,
            StudyStatus::Final,
            StudyStatus::Canceled,
        ]
    }

    /// 获取状态的所有可能事件
    pub fn get_possible_events(&self, current_state: &StudyStatus) -> Vec<StudyEvent> {
        self.transitions
            .keys()
            .filter(|(state, _)| state == current_state)
            .map(|(_, event)| event.clone())
            .collect()
    }
}

impl Default for StudyStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let sm = StudyStateMachine::new();

        // 测试有效转换
        assert!(sm.can_transition(&StudyStatus::Scheduled, &StudyEvent::Started));
        assert!(sm.can_transition(&StudyStatus::InProgress, &StudyEvent::Completed));
        assert!(sm.can_transition(&StudyStatus::Completed, &StudyEvent::PreliminaryReport));
    }

    #[test]
    fn test_invalid_transitions() {
        let sm = StudyStateMachine::new();

        // 测试无效转换
        assert!(!sm.can_transition(&StudyStatus::Final, &StudyEvent::Started));
        assert!(!sm.can_transition(&StudyStatus::Canceled, &StudyEvent::Completed));
    }

    #[test]
    fn test_state_execution() {
        let sm = StudyStateMachine::new();

        let result = sm.transition(&StudyStatus::Scheduled, &StudyEvent::Started);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StudyStatus::InProgress);

        let result = sm.transition(&StudyStatus::Scheduled, &StudyEvent::Completed);
        assert!(result.is_err());
    }
}