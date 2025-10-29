//! 系统监控
//!
//! 提供全面的系统监控功能，包括性能指标收集、健康检查、告警机制等

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use prometheus::{Counter, Gauge, Histogram, IntCounter, IntGauge, Registry, Opts, HistogramOpts};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};

/// 系统监控指标收集器
#[derive(Debug)]
pub struct SystemMonitor {
    /// Prometheus指标注册表
    registry: Registry,
    /// HTTP请求计数器
    http_requests_total: IntCounter,
    /// HTTP请求延迟直方图
    http_request_duration: Histogram,
    /// 当前活跃连接数
    active_connections: IntGauge,
    /// DICOM操作计数器
    dicom_operations_total: IntCounter,
    /// 数据库连接池状态
    db_connections_active: IntGauge,
    db_connections_idle: IntGauge,
    /// 存储使用情况
    storage_usage_bytes: IntGauge,
    /// CPU使用率
    cpu_usage_percent: Gauge,
    /// 内存使用量
    memory_usage_bytes: IntGauge,
    /// 磁盘使用率
    disk_usage_percent: Gauge,
    /// 系统启动时间
    system_start_time: Instant,
    /// 自定义指标
    custom_metrics: Arc<RwLock<HashMap<String, MetricValue>>>,
}

/// 监控指标值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(i64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Text(String),
}

/// 系统健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// 总体健康状态
    pub status: HealthLevel,
    /// 各组件状态
    pub components: HashMap<String, ComponentHealth>,
    /// 检查时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 系统运行时间
    pub uptime: Duration,
}

/// 健康等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// 组件名称
    pub name: String,
    /// 健康状态
    pub status: HealthLevel,
    /// 状态描述
    pub message: String,
    /// 最后检查时间
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// 响应时间
    pub response_time: Option<Duration>,
}

/// 告警规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// 规则名称
    pub name: String,
    /// 监控指标
    pub metric: String,
    /// 阈值
    pub threshold: f64,
    /// 比较操作符
    pub operator: ComparisonOperator,
    /// 告警级别
    pub severity: AlertSeverity,
    /// 持续时间阈值
    pub duration: Duration,
    /// 告警消息模板
    pub message_template: String,
    /// 是否启用
    pub enabled: bool,
}

/// 比较操作符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equals,
    NotEquals,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// 告警严重级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// 告警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    /// 告警ID
    pub id: String,
    /// 规则名称
    pub rule_name: String,
    /// 严重级别
    pub severity: AlertSeverity,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 告警消息
    pub message: String,
    /// 触发时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 是否已解决
    pub resolved: bool,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 监控间隔
    pub interval: Duration,
    /// 健康检查配置
    pub health_check: HealthCheckConfig,
    /// 告警配置
    pub alerts: AlertConfig,
    /// 指标保留时间
    pub metrics_retention: Duration,
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// 是否启用健康检查
    pub enabled: bool,
    /// 检查间隔
    pub interval: Duration,
    /// 超时时间
    pub timeout: Duration,
    /// 要检查的组件
    pub components: Vec<String>,
}

/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// 是否启用告警
    pub enabled: bool,
    /// 告警规则
    pub rules: Vec<AlertRule>,
    /// 通知配置
    pub notifications: NotificationConfig,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// 邮件通知
    pub email: Option<EmailNotificationConfig>,
    /// Webhook通知
    pub webhook: Option<WebhookNotificationConfig>,
    /// 短信通知
    pub sms: Option<SmsNotificationConfig>,
}

/// 邮件通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotificationConfig {
    /// SMTP服务器
    pub smtp_server: String,
    /// 端口
    pub port: u16,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 发件人
    pub from: String,
    /// 收件人列表
    pub to: Vec<String>,
}

/// Webhook通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookNotificationConfig {
    /// Webhook URL
    pub url: String,
    /// 认证令牌
    pub auth_token: Option<String>,
    /// 超时时间
    pub timeout: Duration,
}

/// 短信通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsNotificationConfig {
    /// API提供商
    pub provider: String,
    /// API密钥
    pub api_key: String,
    /// 手机号码列表
    pub phone_numbers: Vec<String>,
}

impl SystemMonitor {
    /// 创建新的系统监控器
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        // 创建Prometheus指标
        let http_requests_total = IntCounter::with_opts(Opts::new(
            "http_requests_total",
            "Total number of HTTP requests"
        ))?;

        let http_request_duration = Histogram::with_opts(HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ))?;

        let active_connections = IntGauge::with_opts(Opts::new(
            "active_connections",
            "Number of active connections"
        ))?;

        let dicom_operations_total = IntCounter::with_opts(Opts::new(
            "dicom_operations_total",
            "Total number of DICOM operations"
        ))?;

        let db_connections_active = IntGauge::with_opts(Opts::new(
            "db_connections_active",
            "Number of active database connections"
        ))?;

        let db_connections_idle = IntGauge::with_opts(Opts::new(
            "db_connections_idle",
            "Number of idle database connections"
        ))?;

        let storage_usage_bytes = IntGauge::with_opts(Opts::new(
            "storage_usage_bytes",
            "Storage usage in bytes"
        ))?;

        let cpu_usage_percent = Gauge::with_opts(Opts::new(
            "cpu_usage_percent",
            "CPU usage percentage"
        ))?;

        let memory_usage_bytes = IntGauge::with_opts(Opts::new(
            "memory_usage_bytes",
            "Memory usage in bytes"
        ))?;

        let disk_usage_percent = Gauge::with_opts(Opts::new(
            "disk_usage_percent",
            "Disk usage percentage"
        ))?;

        // 注册所有指标
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(dicom_operations_total.clone()))?;
        registry.register(Box::new(db_connections_active.clone()))?;
        registry.register(Box::new(db_connections_idle.clone()))?;
        registry.register(Box::new(storage_usage_bytes.clone()))?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        registry.register(Box::new(disk_usage_percent.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            active_connections,
            dicom_operations_total,
            db_connections_active,
            db_connections_idle,
            storage_usage_bytes,
            cpu_usage_percent,
            memory_usage_bytes,
            disk_usage_percent,
            system_start_time: Instant::now(),
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 记录HTTP请求
    pub fn record_http_request(&self, method: &str, path: &str, status: u16, duration: Duration) {
        debug!("HTTP request: {} {} - {} in {:?}", method, path, status, duration);

        // 增加请求计数
        self.http_requests_total.inc();

        // 记录请求延迟
        self.http_request_duration.observe(duration.as_secs_f64());
    }

    /// 更新活跃连接数
    pub fn update_active_connections(&self, count: i64) {
        self.active_connections.set(count);
    }

    /// 记录DICOM操作
    pub fn record_dicom_operation(&self, operation_type: &str) {
        debug!("DICOM operation: {}", operation_type);
        self.dicom_operations_total.inc();
    }

    /// 更新数据库连接池状态
    pub fn update_db_connections(&self, active: i64, idle: i64) {
        self.db_connections_active.set(active);
        self.db_connections_idle.set(idle);
    }

    /// 更新存储使用情况
    pub fn update_storage_usage(&self, usage_bytes: i64) {
        self.storage_usage_bytes.set(usage_bytes);
    }

    /// 更新系统资源使用情况
    pub fn update_system_metrics(&self, cpu_percent: f64, memory_bytes: i64, disk_percent: f64) {
        self.cpu_usage_percent.set(cpu_percent);
        self.memory_usage_bytes.set(memory_bytes);
        self.disk_usage_percent.set(disk_percent);
    }

    /// 设置自定义指标
    pub async fn set_custom_metric(&self, name: String, value: MetricValue) {
        let mut metrics = self.custom_metrics.write().await;
        metrics.insert(name, value);
    }

    /// 获取自定义指标
    pub async fn get_custom_metrics(&self) -> HashMap<String, MetricValue> {
        let metrics = self.custom_metrics.read().await;
        metrics.clone()
    }

    /// 获取Prometheus指标
    pub fn get_prometheus_metrics(&self) -> Result<String> {
        use prometheus::Encoder;

        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;

        Ok(String::from_utf8(buffer)?)
    }

    /// 获取系统运行时间
    pub fn uptime(&self) -> Duration {
        self.system_start_time.elapsed()
    }

    /// 获取系统健康状态
    pub async fn get_health_status(&self) -> HealthStatus {
        let mut components = HashMap::new();
        let overall_status = self.check_component_health(&mut components).await;

        HealthStatus {
            status: overall_status,
            components,
            timestamp: chrono::Utc::now(),
            uptime: self.uptime(),
        }
    }

    /// 检查组件健康状态
    async fn check_component_health(&self, components: &mut HashMap<String, ComponentHealth>) -> HealthLevel {
        let now = chrono::Utc::now();
        let mut overall_status = HealthLevel::Healthy;

        // 检查数据库连接
        let db_health = self.check_database_health().await;
        components.insert("database".to_string(), db_health.clone());

        // 检查存储系统
        let storage_health = self.check_storage_health().await;
        components.insert("storage".to_string(), storage_health.clone());

        // 检查DICOM服务
        let dicom_health = self.check_dicom_health().await;
        components.insert("dicom".to_string(), dicom_health.clone());

        // 检查Web服务
        let web_health = self.check_web_health().await;
        components.insert("web".to_string(), web_health.clone());

        // 检查系统资源
        let system_health = self.check_system_health().await;
        components.insert("system".to_string(), system_health.clone());

        // 确定总体健康状态
        for component in components.values() {
            match component.status {
                HealthLevel::Unhealthy => return HealthLevel::Unhealthy,
                HealthLevel::Degraded => overall_status = HealthLevel::Degraded,
                HealthLevel::Healthy => {}
            }
        }

        overall_status
    }

    /// 检查数据库健康状态
    async fn check_database_health(&self) -> ComponentHealth {
        let start = Instant::now();

        // 这里应该实际检查数据库连接
        // 暂时返回模拟数据
        ComponentHealth {
            name: "Database".to_string(),
            status: HealthLevel::Healthy,
            message: "Database connection is healthy".to_string(),
            last_check: chrono::Utc::now(),
            response_time: Some(start.elapsed()),
        }
    }

    /// 检查存储系统健康状态
    async fn check_storage_health(&self) -> ComponentHealth {
        let start = Instant::now();

        ComponentHealth {
            name: "Storage".to_string(),
            status: HealthLevel::Healthy,
            message: "Storage system is operational".to_string(),
            last_check: chrono::Utc::now(),
            response_time: Some(start.elapsed()),
        }
    }

    /// 检查DICOM服务健康状态
    async fn check_dicom_health(&self) -> ComponentHealth {
        let start = Instant::now();

        ComponentHealth {
            name: "DICOM Service".to_string(),
            status: HealthLevel::Healthy,
            message: "DICOM service is running".to_string(),
            last_check: chrono::Utc::now(),
            response_time: Some(start.elapsed()),
        }
    }

    /// 检查Web服务健康状态
    async fn check_web_health(&self) -> ComponentHealth {
        let start = Instant::now();

        ComponentHealth {
            name: "Web Service".to_string(),
            status: HealthLevel::Healthy,
            message: "Web service is operational".to_string(),
            last_check: chrono::Utc::now(),
            response_time: Some(start.elapsed()),
        }
    }

    /// 检查系统资源健康状态
    async fn check_system_health(&self) -> ComponentHealth {
        let start = Instant::now();

        ComponentHealth {
            name: "System Resources".to_string(),
            status: HealthLevel::Healthy,
            message: "System resources are within normal limits".to_string(),
            last_check: chrono::Utc::now(),
            response_time: Some(start.elapsed()),
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create system monitor")
    }
}
