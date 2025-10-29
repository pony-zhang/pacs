//! Web服务器

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put, delete},
    Router,
};
use pacs_core::Result;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use std::sync::Arc;

use crate::handlers::{health, api_root, get_patients, get_studies, get_series, get_instances};
use crate::wado::{qido_rs, wado_rs, stow_rs};
use crate::auth::{AuthService, auth_middleware, login_handler, get_current_user, get_all_users_handler};

pub struct WebServer {
    addr: SocketAddr,
    app: Router,
}

impl WebServer {
    pub fn new(addr: SocketAddr) -> Self {
        let auth_service = Arc::new(AuthService::new("your-secret-key-here".to_string()));
        let app = Self::create_app(auth_service);

        Self { addr, app }
    }

    fn create_app(auth_service: Arc<AuthService>) -> Router {
        Router::new()
            // 认证路由（无需token）
            .route("/auth/login", post(login_handler))
            .with_state(auth_service.clone())

            // 需要认证的路由
            .route("/auth/me", get(get_current_user))
            .with_state(auth_service.clone())
            .layer(axum::middleware::from_fn_with_state(
                auth_service.clone(),
                auth_middleware,
            ))

            // 根路径
            .route("/", get(api_root))

            // 健康检查
            .route("/health", get(health))

            // API路由
            .nest("/api/v1", api_routes())
            .with_state(auth_service.clone())

            // DICOMweb路由
            .nest("/dicom-web", dicom_web_routes())
            .with_state(auth_service.clone())

            // 静态文件服务
            .nest_service("/static", tower_http::services::ServeDir::new("static"))

            // 全局中间件
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(
                        CorsLayer::new()
                            .allow_origin(Any)
                            .allow_methods(Any)
                            .allow_headers(Any),
                    ),
            )
    }

    pub async fn run(self) -> Result<()> {
        info!("Starting web server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, self.app)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start web server: {}", e))?;

        Ok(())
    }
}

/// API v1 路由
fn api_routes() -> Router<Arc<AuthService>> {
    Router::new()
        .route("/", get(api_root))
        .route("/patients", get(get_patients))
        .route("/studies", get(get_studies))
        .route("/series", get(get_series))
        .route("/instances", get(get_instances))
}

/// DICOMweb 路由
fn dicom_web_routes() -> Router<Arc<AuthService>> {
    Router::new()
        .route("/search", get(qido_rs))        // QIDO-RS
        .route("/retrieve/:study_uid", get(wado_rs))  // WADO-RS
        .route("/retrieve/:study_uid/:series_uid", get(wado_rs))
        .route("/retrieve/:study_uid/:series_uid/:instance_uid", get(wado_rs))
        .route("/store", post(stow_rs))        // STOW-RS
        .route("/store/*path", post(stow_rs))
}

