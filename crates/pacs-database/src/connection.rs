//! 数据库连接管理

use pacs_core::Result;

/// 数据库连接池
pub struct DatabasePool;

impl DatabasePool {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}