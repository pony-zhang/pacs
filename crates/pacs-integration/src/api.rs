//! RESTful API接口模块
//!
//! 为外部系统提供标准化的REST API接口

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::webhook::{WebhookManager, WebhookSubscriptionRequest};

/// API状态管理器
#[derive(Clone)]
pub struct ApiState {
    pub webhook_manager: Arc<RwLock<WebhookManager>>,
}

impl ApiState {
    pub fn new() -> Self {
        Self {
            webhook_manager: Arc::new(RwLock::new(WebhookManager::new())),
        }
    }
}

/// 系统统计响应
#[derive(Debug, Serialize)]
pub struct SystemStatsResponse {
    pub total_patients: u64,
    pub total_studies: u64,
    pub total_series: u64,
    pub total_instances: u64,
    pub storage_used_bytes: u64,
    pub daily_studies: u64,
    pub active_worklists: u64,
}


/// API处理器
pub struct ApiHandler;

impl ApiHandler {
    /// 获取系统统计信息
    pub async fn get_system_stats(
        State(_state): State<ApiState>,
    ) -> Result<Json<SystemStatsResponse>, StatusCode> {
        debug!("Getting system statistics");

        // TODO: 实现系统统计逻辑
        let stats = SystemStatsResponse {
            total_patients: 0,
            total_studies: 0,
            total_series: 0,
            total_instances: 0,
            storage_used_bytes: 0,
            daily_studies: 0,
            active_worklists: 0,
        };

        Ok(Json(stats))
    }

    /// 健康检查
    pub async fn health_check() -> Json<HashMap<String, String>> {
        let mut status = HashMap::new();
        status.insert("status".to_string(), "healthy".to_string());
        status.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        status.insert("version".to_string(), "0.1.0".to_string());
        Json(status)
    }

    /// 创建Webhook订阅
    pub async fn create_webhook(
        State(state): State<ApiState>,
        Json(request): Json<WebhookSubscriptionRequest>,
    ) -> Result<(StatusCode, Json<HashMap<String, String>>), StatusCode> {
        info!("Creating webhook subscription for URL: {}", request.url);

        let mut webhook_manager = state.webhook_manager.write().await;

        match webhook_manager.subscribe(request).await {
            Ok(subscription_id) => {
                let mut response = HashMap::new();
                response.insert("subscription_id".to_string(), subscription_id);
                Ok((StatusCode::CREATED, Json(response)))
            },
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

/// 创建API路由
pub fn create_api_routes() -> Router<ApiState> {
    let api_state = ApiState::new();

    Router::new()
        .route("/system/stats", get(ApiHandler::get_system_stats))
        .route("/health", get(ApiHandler::health_check))
        .route("/webhooks", post(ApiHandler::create_webhook))
        .with_state(api_state)
        .layer(axum::middleware::from_fn(
            |req, next| async move {
                info!("API request: {} {}", req.method(), req.uri());
                let response = next.run(req).await;
                info!("API response: {}", response.status());
                response
            },
        ))
}

/// API服务器
pub struct ApiServer {
    app: Router,
}

impl ApiServer {
    pub fn new() -> Self {
        let app = create_api_routes();
        let app = app.layer(tower_http::cors::CorsLayer::permissive());
        Self { app }
    }

    pub async fn run(self, addr: &str) -> anyhow::Result<()> {
        info!("Starting API server on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.app).await?;
        Ok(())
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new()
    }
}
