//! 配置管理
//!
//! 提供统一的配置管理功能，支持动态配置、验证和热更新

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};
use config::{Config, ConfigError, Environment, File};

/// 配置管理器
#[derive(Debug)]
pub struct ConfigManager {
    /// 配置数据
    config: Arc<RwLock<PacsConfig>>,
    /// 配置文件路径
    config_path: String,
    /// 是否启用热更新
    hot_reload: bool,
    /// 配置验证器
    validator: ConfigValidator,
}

/// PACS系统完整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacsConfig {
    /// 服务器配置
    pub server: ServerConfig,
    /// 数据库配置
    pub database: DatabaseConfig,
    /// 存储配置
    pub storage: StorageConfig,
    /// DICOM服务配置
    pub dicom: DicomConfig,
    /// Web服务配置
    pub web: WebConfig,
    /// 监控配置
    pub monitoring: MonitoringConfig,
    /// 工作流配置
    pub workflow: WorkflowConfig,
    /// 系统集成配置
    pub integration: IntegrationConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 服务器名称
    pub name: String,
    /// 监听主机
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 工作线程数
    pub worker_threads: Option<usize>,
    /// 请求超时时间
    pub request_timeout: Duration,
    /// 启用TLS
    pub tls_enabled: bool,
    /// TLS证书路径
    pub tls_cert_path: Option<String>,
    /// TLS私钥路径
    pub tls_key_path: Option<String>,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库类型
    pub database_type: String,
    /// 连接字符串
    pub connection_string: String,
    /// 最大连接数
    pub max_connections: u32,
    /// 最小连接数
    pub min_connections: u32,
    /// 连接超时时间
    pub connect_timeout: Duration,
    /// 空闲超时时间
    pub idle_timeout: Duration,
    /// 查询超时时间
    pub query_timeout: Duration,
    /// 是否启用连接池
    pub enable_pooling: bool,
    /// 健康检查间隔
    pub health_check_interval: Duration,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 默认存储类型
    pub default_storage_type: StorageType,
    /// 本地存储配置
    pub local: Option<LocalStorageConfig>,
    /// 对象存储配置
    pub object_storage: Option<ObjectStorageConfig>,
    /// 生命周期管理配置
    pub lifecycle: LifecycleConfig,
    /// 备份配置
    pub backup: BackupConfig,
}

/// 存储类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    Local,
    S3,
    Gcs,
    Azure,
}

/// 本地存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    /// 根目录
    pub root_path: String,
    /// 目录权限
    pub dir_permissions: Option<u32>,
    /// 文件权限
    pub file_permissions: Option<u32>,
}

/// 对象存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStorageConfig {
    /// 存储提供商
    pub provider: String,
    /// 访问密钥
    pub access_key: String,
    /// 密钥
    pub secret_key: String,
    /// 区域
    pub region: String,
    /// 桶名
    pub bucket: String,
    /// 端点URL
    pub endpoint: Option<String>,
}

/// 生命周期配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// 是否启用生命周期管理
    pub enabled: bool,
    /// 在线存储时间
    pub online_duration: Duration,
    /// 归档存储时间
    pub archive_duration: Duration,
    /// 冷存储时间
    pub cold_duration: Duration,
}

/// 备份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// 是否启用自动备份
    pub auto_backup: bool,
    /// 备份间隔
    pub backup_interval: Duration,
    /// 备份保留数量
    pub backup_retention: u32,
    /// 备份存储位置
    pub backup_location: String,
    /// 备份压缩
    pub compress_backups: bool,
}

/// DICOM服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DicomConfig {
    /// AETitle
    pub ae_title: String,
    /// 监听端口
    pub port: u16,
    /// 最大PDU大小
    pub max_pdu_size: u32,
    /// 支持的传输语法
    pub supported_transfer_syntaxes: Vec<String>,
    /// 关联超时时间
    pub association_timeout: Duration,
    /// 是否启用C-ECHO
    pub enable_c_echo: bool,
    /// 是否启用C-STORE
    pub enable_c_store: bool,
    /// 是否启用C-FIND
    pub enable_c_find: bool,
    /// 是否启用C-MOVE
    pub enable_c_move: bool,
}

/// Web服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// 启用HTTP
    pub http_enabled: bool,
    /// HTTP端口
    pub http_port: u16,
    /// 启用HTTPS
    pub https_enabled: bool,
    /// HTTPS端口
    pub https_port: u16,
    /// 静态文件目录
    pub static_files_dir: Option<String>,
    /// 启用CORS
    pub enable_cors: bool,
    /// CORS允许的源
    pub cors_allowed_origins: Vec<String>,
    /// 会话超时时间
    pub session_timeout: Duration,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 启用监控
    pub enabled: bool,
    /// 监控间隔
    pub interval: Duration,
    /// 指标端口
    pub metrics_port: u16,
    /// 健康检查配置
    pub health_check: HealthCheckConfig,
    /// 告警配置
    pub alerts: AlertsConfig,
    /// 性能分析配置
    pub performance: PerformanceAnalysisConfig,
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// 启用健康检查
    pub enabled: bool,
    /// 检查间隔
    pub interval: Duration,
    /// 超时时间
    pub timeout: Duration,
    /// 检查的组件
    pub components: Vec<String>,
}

/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertsConfig {
    /// 启用告警
    pub enabled: bool,
    /// 告警规则文件
    pub rules_file: Option<String>,
    /// 通知配置
    pub notifications: NotificationConfig,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// 邮件通知
    pub email: Option<EmailConfig>,
    /// Webhook通知
    pub webhook: Option<WebhookConfig>,
    /// 短信通知
    pub sms: Option<SmsConfig>,
}

/// 邮件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP服务器
    pub smtp_server: String,
    /// SMTP端口
    pub smtp_port: u16,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 发件人
    pub from: String,
    /// 收件人
    pub to: Vec<String>,
}

/// Webhook配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// 认证令牌
    pub auth_token: Option<String>,
    /// 超时时间
    pub timeout: Duration,
}

/// 短信配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    /// 提供商
    pub provider: String,
    /// API密钥
    pub api_key: String,
    /// 手机号列表
    pub phone_numbers: Vec<String>,
}

/// 性能分析配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysisConfig {
    /// 启用性能分析
    pub enabled: bool,
    /// 采样间隔
    pub sampling_interval: Duration,
    /// 历史数据保留时间
    pub retention_period: Duration,
    /// 报告生成间隔
    pub report_interval: Duration,
}

/// 工作流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// 启用工作流引擎
    pub enabled: bool,
    /// 自动路由配置
    pub auto_routing: AutoRoutingConfig,
    /// 危急值处理配置
    pub critical_values: CriticalValuesConfig,
    /// 工作列表配置
    pub worklist: WorklistConfig,
}

/// 自动路由配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRoutingConfig {
    /// 启用自动路由
    pub enabled: bool,
    /// 路由规则文件
    pub rules_file: Option<String>,
    /// 负载均衡算法
    pub load_balancing_algorithm: String,
}

/// 危急值处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalValuesConfig {
    /// 启用危急值处理
    pub enabled: bool,
    /// 危急值规则文件
    pub rules_file: Option<String>,
    /// 自动通知
    pub auto_notification: bool,
}

/// 工作列表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorklistConfig {
    /// 启用工作列表
    pub enabled: bool,
    /// 默认页面大小
    pub default_page_size: usize,
    /// 最大页面大小
    pub max_page_size: usize,
}

/// 系统集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// HL7配置
    pub hl7: Hl7Config,
    /// REST API配置
    pub rest_api: RestApiConfig,
    /// 消息队列配置
    pub message_queue: MessageQueueConfig,
    /// 外部连接器配置
    pub connectors: HashMap<String, ConnectorConfig>,
}

/// HL7配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hl7Config {
    /// 启用HL7接口
    pub enabled: bool,
    /// 监听端口
    pub port: u16,
    /// 支持的消息类型
    pub supported_message_types: Vec<String>,
}

/// REST API配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    /// 启用REST API
    pub enabled: bool,
    /// API版本
    pub version: String,
    /// 启用认证
    pub authentication: bool,
    /// API密钥
    pub api_keys: Vec<String>,
}

/// 消息队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueConfig {
    /// 队列类型
    pub queue_type: String,
    /// 连接字符串
    pub connection_string: String,
    /// 队列名称
    pub queue_name: String,
}

/// 连接器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// 连接器类型
    pub connector_type: String,
    /// 连接字符串
    pub connection_string: String,
    /// 认证配置
    pub auth: Option<HashMap<String, String>>,
    /// 额外配置
    pub settings: HashMap<String, String>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: String,
    /// 日志格式
    pub format: String,
    /// 日志输出
    pub outputs: Vec<String>,
    /// 日志文件路径
    pub file_path: Option<String>,
    /// 日志轮转配置
    pub rotation: LogRotationConfig,
}

/// 日志轮转配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// 启用轮转
    pub enabled: bool,
    /// 最大文件大小
    pub max_file_size: u64,
    /// 最大文件数量
    pub max_files: u32,
}

/// 配置验证器
#[derive(Debug)]
pub struct ConfigValidator {
    /// 验证规则
    validation_rules: Vec<ValidationRule>,
}

/// 验证规则
#[derive(Debug)]
struct ValidationRule {
    /// 字段路径
    field_path: String,
    /// 验证函数
    validator: fn(&PacsConfig) -> Result<()>,
    /// 错误消息
    error_message: String,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: &str, hot_reload: bool) -> Result<Self> {
        let config = Self::load_config(config_path)?;
        let validator = ConfigValidator::new();

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.to_string(),
            hot_reload,
            validator,
        })
    }

    /// 从文件加载配置
    fn load_config(config_path: &str) -> Result<PacsConfig> {
        let settings = Config::builder()
            .add_source(File::with_name(config_path))
            .add_source(Environment::with_prefix("PACS").separator("_"))
            .build()?;

        let config: PacsConfig = settings.try_deserialize()
            .context("Failed to deserialize configuration")?;

        info!("Configuration loaded successfully from: {}", config_path);
        Ok(config)
    }

    /// 获取配置
    pub async fn get_config(&self) -> PacsConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, new_config: PacsConfig) -> Result<()> {
        // 验证新配置
        self.validator.validate(&new_config)?;

        // 更新配置
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // 保存配置到文件
        self.save_config().await?;

        info!("Configuration updated successfully");
        Ok(())
    }

    /// 保存配置到文件
    async fn save_config(&self) -> Result<()> {
        let config = self.config.read().await;
        let config_str = toml::to_string_pretty(&*config)
            .context("Failed to serialize configuration")?;

        tokio::fs::write(&self.config_path, config_str).await
            .context("Failed to write configuration file")?;

        info!("Configuration saved to: {}", self.config_path);
        Ok(())
    }

    /// 重新加载配置
    pub async fn reload_config(&self) -> Result<()> {
        let new_config = Self::load_config(&self.config_path)?;
        self.update_config(new_config).await
    }

    /// 获取配置值
    pub async fn get_value<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let config = self.config.read().await;
        let value = self.extract_nested_value(&config, path)
            .context(format!("Configuration path not found: {}", path))?;

        serde_json::from_value(value)
            .context("Failed to deserialize configuration value")
    }

    /// 设置配置值
    pub async fn set_value<T>(&self, path: &str, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let value_json = serde_json::to_value(value)
            .context("Failed to serialize value")?;

        let mut config = self.config.write().await;
        self.set_nested_value(&mut config, path, value_json)?;

        info!("Configuration value updated: {}", path);
        Ok(())
    }

    /// 提取嵌套值
    fn extract_nested_value(&self, config: &PacsConfig, path: &str) -> Result<serde_json::Value> {
        let config_json = serde_json::to_value(config)
            .context("Failed to serialize config to JSON")?;

        let mut current = &config_json;
        for part in path.split('.') {
            match current {
                serde_json::Value::Object(map) => {
                    current = map.get(part)
                        .ok_or_else(|| anyhow::anyhow!("Path segment not found: {}", part))?;
                }
                _ => return Err(anyhow::anyhow!("Invalid path at segment: {}", part)),
            }
        }

        Ok(current.clone())
    }

    /// 设置嵌套值
    fn set_nested_value(&self, config: &mut PacsConfig, path: &str, value: serde_json::Value) -> Result<()> {
        // 简化实现，实际应该支持深度嵌套路径
        match path {
            "server.name" => {
                if let Some(name) = value.as_str() {
                    config.server.name = name.to_string();
                }
            }
            "server.port" => {
                if let Some(port) = value.as_u64() {
                    config.server.port = port as u16;
                }
            }
            "database.max_connections" => {
                if let Some(max_connections) = value.as_u64() {
                    config.database.max_connections = max_connections as u32;
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported configuration path: {}", path)),
        }

        Ok(())
    }

    /// 验证配置
    pub async fn validate_config(&self) -> Result<()> {
        let config = self.config.read().await;
        self.validator.validate(&*config)
    }

    /// 启动热更新监控
    pub async fn start_hot_reload(&self) -> Result<()> {
        if !self.hot_reload {
            return Ok(());
        }

        info!("Starting configuration hot reload monitoring");

        // 这里应该实现文件监控逻辑
        // 暂时只是一个占位符

        Ok(())
    }
}

impl ConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        let validation_rules = vec![
            ValidationRule {
                field_path: "server.port".to_string(),
                validator: |config| {
                    if config.server.port == 0 {
                        Err(anyhow::anyhow!("Server port cannot be 0"))
                    } else {
                        Ok(())
                    }
                },
                error_message: "Invalid server port".to_string(),
            },
            ValidationRule {
                field_path: "database.max_connections".to_string(),
                validator: |config| {
                    if config.database.max_connections == 0 {
                        Err(anyhow::anyhow!("Database max connections cannot be 0"))
                    } else {
                        Ok(())
                    }
                },
                error_message: "Invalid database max connections".to_string(),
            },
        ];

        Self {
            validation_rules,
        }
    }

    /// 验证配置
    pub fn validate(&self, config: &PacsConfig) -> Result<()> {
        for rule in &self.validation_rules {
            if let Err(e) = (rule.validator)(config) {
                error!("Configuration validation failed for {}: {}", rule.field_path, e);
                return Err(anyhow::anyhow!("{}: {}", rule.error_message, e));
            }
        }

        info!("Configuration validation passed");
        Ok(())
    }
}

impl Default for PacsConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            storage: StorageConfig::default(),
            dicom: DicomConfig::default(),
            web: WebConfig::default(),
            monitoring: MonitoringConfig::default(),
            workflow: WorkflowConfig::default(),
            integration: IntegrationConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "PACS-Server".to_string(),
            host: "0.0.0.0".to_string(),
            port: 11112,
            worker_threads: None,
            request_timeout: Duration::from_secs(30),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_type: "postgresql".to_string(),
            connection_string: "postgresql://pacs:password@localhost/pacs".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            query_timeout: Duration::from_secs(30),
            enable_pooling: true,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            default_storage_type: StorageType::Local,
            local: Some(LocalStorageConfig {
                root_path: "./data".to_string(),
                dir_permissions: Some(0o755),
                file_permissions: Some(0o644),
            }),
            object_storage: None,
            lifecycle: LifecycleConfig {
                enabled: false,
                online_duration: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                archive_duration: Duration::from_secs(365 * 24 * 60 * 60), // 1 year
                cold_duration: Duration::from_secs(7 * 365 * 24 * 60 * 60), // 7 years
            },
            backup: BackupConfig {
                auto_backup: false,
                backup_interval: Duration::from_secs(24 * 60 * 60), // 1 day
                backup_retention: 30,
                backup_location: "./backups".to_string(),
                compress_backups: true,
            },
        }
    }
}

impl Default for DicomConfig {
    fn default() -> Self {
        Self {
            ae_title: "PACS-SERVER".to_string(),
            port: 11112,
            max_pdu_size: 16384,
            supported_transfer_syntaxes: vec![
                "1.2.840.10008.1.2".to_string(), // Implicit VR Little Endian
                "1.2.840.10008.1.2.1".to_string(), // Explicit VR Little Endian
            ],
            association_timeout: Duration::from_secs(30),
            enable_c_echo: true,
            enable_c_store: true,
            enable_c_find: true,
            enable_c_move: false,
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            http_enabled: true,
            http_port: 8080,
            https_enabled: false,
            https_port: 8443,
            static_files_dir: Some("./static".to_string()),
            enable_cors: true,
            cors_allowed_origins: vec!["*".to_string()],
            session_timeout: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            metrics_port: 9090,
            health_check: HealthCheckConfig {
                enabled: true,
                interval: Duration::from_secs(60),
                timeout: Duration::from_secs(5),
                components: vec!["database".to_string(), "storage".to_string()],
            },
            alerts: AlertsConfig {
                enabled: false,
                rules_file: None,
                notifications: NotificationConfig {
                    email: None,
                    webhook: None,
                    sms: None,
                },
            },
            performance: PerformanceAnalysisConfig {
                enabled: true,
                sampling_interval: Duration::from_secs(30),
                retention_period: Duration::from_secs(24 * 60 * 60), // 24 hours
                report_interval: Duration::from_secs(60 * 60), // 1 hour
            },
        }
    }
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_routing: AutoRoutingConfig {
                enabled: true,
                rules_file: None,
                load_balancing_algorithm: "round_robin".to_string(),
            },
            critical_values: CriticalValuesConfig {
                enabled: true,
                rules_file: None,
                auto_notification: true,
            },
            worklist: WorklistConfig {
                enabled: true,
                default_page_size: 20,
                max_page_size: 100,
            },
        }
    }
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            hl7: Hl7Config {
                enabled: false,
                port: 2575,
                supported_message_types: vec!["ADT".to_string(), "ORM".to_string()],
            },
            rest_api: RestApiConfig {
                enabled: true,
                version: "v1".to_string(),
                authentication: true,
                api_keys: vec![],
            },
            message_queue: MessageQueueConfig {
                queue_type: "rabbitmq".to_string(),
                connection_string: "amqp://localhost:5672".to_string(),
                queue_name: "pacs".to_string(),
            },
            connectors: HashMap::new(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            outputs: vec!["console".to_string()],
            file_path: Some("./logs/pacs.log".to_string()),
            rotation: LogRotationConfig {
                enabled: true,
                max_file_size: 100 * 1024 * 1024, // 100MB
                max_files: 10,
            },
        }
    }
}
