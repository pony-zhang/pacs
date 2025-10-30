//! 用户认证和授权系统

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use pacs_core::{error::PacsError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 用户角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    /// 管理员 - 完全访问权限
    Admin,
    /// 放射科医生 - 诊断和查看权限
    Radiologist,
    /// 技师 - 上传和基础查看权限
    Technician,
    /// 只读用户 - 仅查看权限
    Viewer,
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::Admin => "admin".to_string(),
            UserRole::Radiologist => "radiologist".to_string(),
            UserRole::Technician => "technician".to_string(),
            UserRole::Viewer => "viewer".to_string(),
        }
    }
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// 用户信息（不包含敏感数据）
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub is_active: bool,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,      // 用户ID
    username: String, // 用户名
    role: String,     // 角色
    exp: usize,       // 过期时间
    iat: usize,       // 签发时间
    jti: String,      // JWT ID
}

/// 认证服务
#[derive(Clone)]
pub struct AuthService {
    users: Arc<RwLock<HashMap<String, User>>>,
    jwt_secret: String,
    token_expiry_hours: i64,
}

impl AuthService {
    pub fn new(jwt_secret: String) -> Self {
        let service = Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret,
            token_expiry_hours: 24,
        };

        // 初始化默认用户
        tokio::spawn({
            let service = service.clone();
            async move {
                service.init_default_users().await;
            }
        });

        service
    }

    /// 初始化默认用户
    async fn init_default_users(&self) {
        let default_users = vec![
            User {
                id: Uuid::new_v4(),
                username: "admin".to_string(),
                email: "admin@pacs.local".to_string(),
                name: "System Administrator".to_string(),
                role: UserRole::Admin,
                is_active: true,
                created_at: chrono::Utc::now(),
                last_login: None,
            },
            User {
                id: Uuid::new_v4(),
                username: "radiologist".to_string(),
                email: "radio@pacs.local".to_string(),
                name: "Dr. Smith".to_string(),
                role: UserRole::Radiologist,
                is_active: true,
                created_at: chrono::Utc::now(),
                last_login: None,
            },
            User {
                id: Uuid::new_v4(),
                username: "tech".to_string(),
                email: "tech@pacs.local".to_string(),
                name: "John Technician".to_string(),
                role: UserRole::Technician,
                is_active: true,
                created_at: chrono::Utc::now(),
                last_login: None,
            },
        ];

        let mut users = self.users.write().await;
        for user in default_users {
            // 注意：实际应用中应该使用安全的密码哈希
            // 这里为了演示使用明文密码
            users.insert(user.username.clone(), user);
        }

        info!("Initialized default users for PACS system");
    }

    /// 用户登录
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        let users = self.users.read().await;

        let user = users
            .get(&request.username)
            .ok_or_else(|| PacsError::Validation("Invalid username or password".to_string()))?;

        if !user.is_active {
            return Err(PacsError::Validation("Account is disabled".to_string()));
        }

        // TODO: 实际应用中应该使用安全的密码验证
        // 这里为了演示，简单验证密码为用户名
        if request.password != user.username {
            return Err(PacsError::Validation(
                "Invalid username or password".to_string(),
            ));
        }

        // 生成JWT token
        let token = self.generate_token(user).await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(self.token_expiry_hours);

        // 更新最后登录时间
        drop(users);
        let mut users = self.users.write().await;
        if let Some(user_mut) = users.get_mut(&user.username) {
            user_mut.last_login = Some(chrono::Utc::now());
        }

        Ok(LoginResponse {
            token,
            user: UserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
                name: user.name.clone(),
                role: user.role.clone(),
                is_active: user.is_active,
            },
            expires_at,
        })
    }

    /// 生成JWT token
    async fn generate_token(&self, user: &User) -> Result<String> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(self.token_expiry_hours);

        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti: Uuid::new_v4().to_string(),
        };

        // TODO: 实际使用真实的JWT库
        // 这里为了演示，简单编码claims
        let token = format!(
            "{}.{}.{}",
            base64::encode(serde_json::to_string(&claims)?),
            "signature", // 模拟签名
            "header"     // 模拟头部
        );

        Ok(token)
    }

    /// 验证JWT token
    pub async fn verify_token(&self, token: &str) -> Result<User> {
        // TODO: 实际使用真实的JWT验证
        // 这里为了演示，简单解析token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(PacsError::Validation("Invalid token format".to_string()));
        }

        let claims_data = base64::decode(parts[0])
            .map_err(|_| PacsError::Validation("Invalid token encoding".to_string()))?;

        let claims: Claims = serde_json::from_slice(&claims_data)
            .map_err(|_| PacsError::Validation("Invalid token claims".to_string()))?;

        // 检查过期时间
        let now = chrono::Utc::now().timestamp() as usize;
        if claims.exp < now {
            return Err(PacsError::Validation("Token has expired".to_string()));
        }

        // 获取用户信息
        let users = self.users.read().await;
        let user = users
            .get(&claims.username)
            .ok_or_else(|| PacsError::Validation("User not found".to_string()))?;

        if !user.is_active {
            return Err(PacsError::Validation("Account is disabled".to_string()));
        }

        Ok(user.clone())
    }

    /// 获取所有用户（管理员功能）
    pub async fn get_all_users(&self) -> Vec<User> {
        self.users.read().await.values().cloned().collect()
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    request: Request,
    next: Next,
) -> Result<Response, pacs_core::error::PacsError> {
    // 从请求头获取token
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            &header[7..] // 移除 "Bearer " 前缀
        }
        _ => {
            return Err(PacsError::Validation("Missing token".to_string()));
        }
    };

    // 验证token
    match auth_service.verify_token(token).await {
        Ok(user) => {
            // 将用户信息添加到请求扩展中
            let mut request = request;
            request.extensions_mut().insert(user);
            Ok(next.run(request).await)
        }
        Err(_) => Err(PacsError::Validation("Invalid token".to_string())),
    }
}

/// 登录处理器
pub async fn login_handler(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    info!("Login attempt for user: {}", request.username);

    match auth_service.login(request).await {
        Ok(response) => {
            info!("User logged in successfully: {}", response.user.username);
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Login failed: {}", e);
            Err(e)
        }
    }
}

/// 获取当前用户信息
pub async fn get_current_user(request: Request) -> Result<impl IntoResponse> {
    let user = request
        .extensions()
        .get::<User>()
        .ok_or_else(|| PacsError::Validation("User not authenticated".to_string()))?;

    let user_info = UserInfo {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        role: user.role.clone(),
        is_active: user.is_active,
    };

    Ok(Json(user_info))
}

/// 获取所有用户（仅管理员）
pub async fn get_all_users_handler(
    State(auth_service): State<Arc<AuthService>>,
    request: Request,
) -> Result<impl IntoResponse> {
    let current_user = request
        .extensions()
        .get::<User>()
        .ok_or_else(|| PacsError::Validation("User not authenticated".to_string()))?;

    if current_user.role != UserRole::Admin {
        return Err(PacsError::Validation("Admin access required".to_string()));
    }

    let users = auth_service.get_all_users().await;
    Ok(Json(users))
}

// 简单的base64编码解码（用于演示）
mod base64 {
    use std::collections::HashMap;

    pub fn encode(input: String) -> String {
        // 简化的base64编码（仅用于演示）
        // 实际应用中应该使用标准的base64库
        format!("BASE64({})", input.len())
    }

    pub fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
        // 简化的base64解码（仅用于演示）
        if input.starts_with("BASE64(") && input.ends_with(")") {
            Ok(vec![0u8; 100]) // 模拟解码结果
        } else {
            Err("Invalid base64 format")
        }
    }
}
