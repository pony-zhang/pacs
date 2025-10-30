//! # PACS工作流模块
//!
//! 提供完整的医学影像工作流管理功能，包括：
//! - 检查状态机：管理影像检查的完整生命周期
//! - 自动路由引擎：根据检查类型和医生专长自动分配任务
//! - 工作列表管理：为不同角色用户提供个性化的任务列表
//! - 危急值处理：确保紧急情况能够及时通知相关人员

pub mod critical_value;
pub mod engine;
pub mod routing;
pub mod state_machine;
pub mod worklist;

// 重新导出主要类型
pub use critical_value::{
    CriticalSeverity, CriticalValueEvent, CriticalValueProcessor, CriticalValueType,
};
pub use engine::{WorkflowEngine, WorkflowSystemOverview};
pub use routing::{
    Radiologist, RadiologistSpecialty, RoutingEngine, RoutingPriority, RoutingRequest,
    RoutingResult,
};
pub use state_machine::{StudyEvent, StudyStateMachine};
pub use worklist::{
    WorkItem, WorkItemPriority, WorkItemStatus, WorkListFilter, WorkListManager, WorkListStats,
};
