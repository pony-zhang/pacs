//! # PACS集成模块
//!
//! 提供与外部系统的集成功能，包括：
//! - HL7 v2.x标准接口，用于与HIS/RIS系统集成
//! - RESTful API接口，支持标准HTTP操作
//! - Webhook事件通知系统，实现实时事件推送
//! - 外部系统连接器，支持多种第三方系统集成
//! - 消息队列集成，提供可靠的消息传递机制

pub mod api;
pub mod connectors;
pub mod hl7;
pub mod message_queue;
pub mod webhook;

pub use api::{ApiServer, ApiState, SystemStatsResponse};
pub use hl7::{Hl7Interface, Hl7Message, Hl7Parser, OrderInfo, PatientInfo};
pub use webhook::{WebhookEvent, WebhookEventType, WebhookManager, WebhookSubscription};
