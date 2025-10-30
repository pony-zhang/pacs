//! 数据库连接管理

use pacs_core::{PacsError, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// 数据库连接池
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// 创建新的数据库连接池
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// 获取连接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 运行数据库迁移
    pub async fn migrate(&self) -> Result<()> {
        // 这里可以集成sqlx migrate或者手动执行DDL
        tracing::info!("Running database migrations");
        // 实际迁移逻辑将在后续实现
        Ok(())
    }

    /// 检查数据库连接
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PacsError::Database(e.to_string()))?;
        Ok(())
    }

    /// 关闭连接池
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
