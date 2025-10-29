//! # PACS数据库模块
//!
//! 负责医学影像元数据的存储和管理，提供PostgreSQL数据库连接池和完整的CRUD操作。

pub mod connection;
pub mod models;
pub mod queries;

// 重新导出主要类型
pub use connection::DatabasePool;
pub use models::*;
pub use queries::DatabaseQueries;