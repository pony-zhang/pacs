//! 错误定义模块

use thiserror::Error;

/// PACS系统统一错误类型
#[derive(Error, Debug)]
pub enum PacsError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("DICOM处理错误: {0}")]
    Dicom(String),

    #[error("DICOM解析错误: {0}")]
    DicomParseError(String),

    #[error("存储错误: {0}")]
    Storage(String),

    #[error("网络错误: {0}")]
    Network(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("权限错误: {0}")]
    Permission(String),

    #[error("系统内部错误: {0}")]
    Internal(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("IO错误: {0}")]
    Io(String),

    #[error("工作流错误: {0}")]
    Workflow(String),

    #[error("路由错误: {0}")]
    RoutingError(String),

    #[error("无效状态转换: 从 {from} 到 {event}")]
    InvalidStateTransition { from: String, event: String },
}

/// PACS系统统一结果类型
pub type Result<T> = std::result::Result<T, PacsError>;