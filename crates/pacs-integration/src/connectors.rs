//! 外部系统连接器模块
//!
//! 提供与各种第三方系统的连接器，支持：
//! - EMR/EHR系统集成
//! - 第三方影像系统连接
//! - 云服务集成
//! - 标准化接口适配器

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// 连接器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub connector_type: ConnectorType,
    pub name: String,
    pub endpoint: String,
    pub authentication: AuthenticationConfig,
    pub settings: HashMap<String, serde_json::Value>,
    pub enabled: bool,
}

/// 连接器类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorType {
    EMR,
    EHR,
    PACS,
    RIS,
    CloudStorage,
    Notification,
    Custom(String),
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationConfig {
    None,
    BasicAuth { username: String, password: String },
    ApiKey { key: String, header: Option<String> },
    BearerToken { token: String },
    OAuth2 { client_id: String, client_secret: String, token_url: String },
    Certificate { cert_path: String, key_path: String },
}

/// 连接器状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// 连接器接口
#[async_trait]
pub trait Connector: Send + Sync {
    /// 获取连接器名称
    fn name(&self) -> &str;

    /// 获取连接器类型
    fn connector_type(&self) -> ConnectorType;

    /// 初始化连接器
    async fn initialize(&mut self, config: ConnectorConfig) -> Result<()>;

    /// 检查连接状态
    async fn check_connection(&self) -> Result<bool>;

    /// 获取连接状态
    fn status(&self) -> ConnectorStatus;

    /// 关闭连接器
    async fn shutdown(&mut self) -> Result<()>;
}

/// EMR连接器
pub struct EmrConnector {
    name: String,
    status: ConnectorStatus,
    config: Option<ConnectorConfig>,
    client: Option<reqwest::Client>,
}

impl EmrConnector {
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: ConnectorStatus::Disconnected,
            config: None,
            client: None,
        }
    }

    /// 查询患者信息
    pub async fn query_patient(&self, patient_id: &str) -> Result<serde_json::Value> {
        if let Some(client) = &self.client {
            if let Some(config) = &self.config {
                let url = format!("{}/patients/{}", config.endpoint, patient_id);
                let mut request = client.get(&url);

                // 添加认证头
                let request = Self::add_auth_headers(request, &config.authentication)?;

                let response = request.send().await?;
                if response.status().is_success() {
                    let patient_data = response.json().await?;
                    Ok(patient_data)
                } else {
                    Err(anyhow::anyhow!("Failed to query patient: {}", response.status()))
                }
            } else {
                Err(anyhow::anyhow!("Connector not configured"))
            }
        } else {
            Err(anyhow::anyhow!("Connector not initialized"))
        }
    }

    /// 提交检查申请
    pub async fn submit_order(&self, order_data: serde_json::Value) -> Result<String> {
        if let Some(client) = &self.client {
            if let Some(config) = &self.config {
                let url = format!("{}/orders", config.endpoint);
                let mut request = client.post(&url).json(&order_data);

                let request = Self::add_auth_headers(request, &config.authentication)?;

                let response = request.send().await?;
                if response.status().is_success() {
                    let result: serde_json::Value = response.json().await?;
                    let order_id = result["order_id"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("No order_id in response"))?;
                    Ok(order_id.to_string())
                } else {
                    Err(anyhow::anyhow!("Failed to submit order: {}", response.status()))
                }
            } else {
                Err(anyhow::anyhow!("Connector not configured"))
            }
        } else {
            Err(anyhow::anyhow!("Connector not initialized"))
        }
    }

    /// 添加认证头
    fn add_auth_headers(
        request: reqwest::RequestBuilder,
        auth: &AuthenticationConfig,
    ) -> Result<reqwest::RequestBuilder> {
        match auth {
            AuthenticationConfig::None => Ok(request),
            AuthenticationConfig::BasicAuth { username, password } => {
                Ok(request.basic_auth(username, Some(password)))
            },
            AuthenticationConfig::ApiKey { key, header } => {
                let header_name = header.as_deref().unwrap_or("X-API-Key");
                Ok(request.header(header_name, key))
            },
            AuthenticationConfig::BearerToken { token } => {
                Ok(request.bearer_auth(token))
            },
            AuthenticationConfig::OAuth2 { client_id, client_secret, token_url: _ } => {
                // TODO: 实现OAuth2流程
                warn!("OAuth2 authentication not fully implemented");
                Ok(request.header("X-Client-ID", client_id))
            },
            AuthenticationConfig::Certificate { cert_path: _, key_path: _ } => {
                // TODO: 实现证书认证
                warn!("Certificate authentication not implemented");
                Ok(request)
            },
        }
    }
}

#[async_trait]
impl Connector for EmrConnector {
    fn name(&self) -> &str {
        &self.name
    }

    fn connector_type(&self) -> ConnectorType {
        ConnectorType::EMR
    }

    async fn initialize(&mut self, config: ConnectorConfig) -> Result<()> {
        info!("Initializing EMR connector: {}", self.name);

        self.config = Some(config.clone());
        self.status = ConnectorStatus::Connecting;

        let client = reqwest::Client::new();
        self.client = Some(client);

        // 测试连接
        match self.check_connection().await {
            Ok(true) => {
                self.status = ConnectorStatus::Connected;
                info!("EMR connector {} connected successfully", self.name);
                Ok(())
            },
            Ok(false) => {
                self.status = ConnectorStatus::Error("Connection test failed".to_string());
                Err(anyhow::anyhow!("Connection test failed"))
            },
            Err(e) => {
                self.status = ConnectorStatus::Error(e.to_string());
                Err(e)
            }
        }
    }

    async fn check_connection(&self) -> Result<bool> {
        if let Some(client) = &self.client {
            if let Some(config) = &self.config {
                let health_url = format!("{}/health", config.endpoint);
                let mut request = client.get(&health_url);

                let request = Self::add_auth_headers(request, &config.authentication)?;

                match request.send().await {
                    Ok(response) => Ok(response.status().is_success()),
                    Err(e) => {
                        warn!("Health check failed for {}: {}", self.name, e);
                        Ok(false)
                    }
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    fn status(&self) -> ConnectorStatus {
        self.status.clone()
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down EMR connector: {}", self.name);
        self.client = None;
        self.status = ConnectorStatus::Disconnected;
        Ok(())
    }
}

/// 云存储连接器
pub struct CloudStorageConnector {
    name: String,
    status: ConnectorStatus,
    config: Option<ConnectorConfig>,
}

impl CloudStorageConnector {
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: ConnectorStatus::Disconnected,
            config: None,
        }
    }

    /// 上传文件
    pub async fn upload_file(&self, key: &str, data: Vec<u8>) -> Result<String> {
        if let Some(config) = &self.config {
            // TODO: 实现云存储上传逻辑
            // 这里应该使用object_store或其他云存储SDK
            info!("Uploading file {} to cloud storage", key);

            // 模拟上传
            let url = format!("{}/{}", config.endpoint, key);
            Ok(url)
        } else {
            Err(anyhow::anyhow!("Connector not configured"))
        }
    }

    /// 下载文件
    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>> {
        if let Some(config) = &self.config {
            // TODO: 实现云存储下载逻辑
            info!("Downloading file {} from cloud storage", key);

            // 模拟下载
            Ok(vec![])
        } else {
            Err(anyhow::anyhow!("Connector not configured"))
        }
    }
}

#[async_trait]
impl Connector for CloudStorageConnector {
    fn name(&self) -> &str {
        &self.name
    }

    fn connector_type(&self) -> ConnectorType {
        ConnectorType::CloudStorage
    }

    async fn initialize(&mut self, config: ConnectorConfig) -> Result<()> {
        info!("Initializing Cloud Storage connector: {}", self.name);

        self.config = Some(config);
        self.status = ConnectorStatus::Connecting;

        // TODO: 初始化云存储客户端
        self.status = ConnectorStatus::Connected;
        info!("Cloud Storage connector {} connected successfully", self.name);
        Ok(())
    }

    async fn check_connection(&self) -> Result<bool> {
        // TODO: 实现云存储连接检查
        Ok(self.config.is_some())
    }

    fn status(&self) -> ConnectorStatus {
        self.status.clone()
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down Cloud Storage connector: {}", self.name);
        self.status = ConnectorStatus::Disconnected;
        Ok(())
    }
}

/// 连接器管理器
pub struct ConnectorManager {
    connectors: HashMap<String, Box<dyn Connector>>,
}

impl ConnectorManager {
    /// 创建新的连接器管理器
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    /// 注册连接器
    pub fn register_connector(&mut self, connector: Box<dyn Connector>) {
        let name = connector.name().to_string();
        info!("Registering connector: {}", name);
        self.connectors.insert(name, connector);
    }

    /// 初始化连接器
    pub async fn initialize_connector(&mut self, name: &str, config: ConnectorConfig) -> Result<()> {
        if let Some(connector) = self.connectors.get_mut(name) {
            connector.initialize(config).await
        } else {
            Err(anyhow::anyhow!("Connector not found: {}", name))
        }
    }

    /// 获取连接器
    pub fn get_connector(&self, name: &str) -> Option<&dyn Connector> {
        self.connectors.get(name).map(|c| c.as_ref())
    }

    /// 获取EMR连接器
    pub fn get_emr_connector(&self, name: &str) -> Option<&EmrConnector> {
        self.connectors.get(name).and_then(|c| {
            c.as_ref().as_any().downcast_ref::<EmrConnector>()
        })
    }

    /// 获取云存储连接器
    pub fn get_cloud_storage_connector(&self, name: &str) -> Option<&CloudStorageConnector> {
        self.connectors.get(name).and_then(|c| {
            c.as_ref().as_any().downcast_ref::<CloudStorageConnector>()
        })
    }

    /// 列出所有连接器状态
    pub fn list_connector_status(&self) -> HashMap<String, ConnectorStatus> {
        self.connectors
            .iter()
            .map(|(name, connector)| (name.clone(), connector.status()))
            .collect()
    }

    /// 关闭所有连接器
    pub async fn shutdown_all(&mut self) -> Result<()> {
        info!("Shutting down all connectors");

        for (name, connector) in self.connectors.iter_mut() {
            if let Err(e) = connector.shutdown().await {
                error!("Failed to shutdown connector {}: {}", name, e);
            }
        }

        self.connectors.clear();
        Ok(())
    }
}

impl Default for ConnectorManager {
    fn default() -> Self {
        Self::new()
    }
}

// 为了支持downcast，需要为Connector trait添加as_any方法
pub trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Connector + 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}