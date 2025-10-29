//! # PACS管理模块
//!
//! 提供系统监控、告警、日志聚合、性能分析和配置管理等运维功能

pub mod config;
pub mod monitoring;
pub mod alerting;
pub mod logging;
pub mod performance;
pub mod backup;

use std::sync::Arc;
use anyhow::Result;

/// 系统管理器
///
/// 集成所有管理功能的统一入口点
#[derive(Debug)]
pub struct SystemManager {
    /// 配置管理器
    config_manager: Arc<config::ConfigManager>,
    /// 系统监控器
    system_monitor: Arc<monitoring::SystemMonitor>,
    /// 告警管理器
    alert_manager: Arc<alerting::AlertManager>,
    /// 日志聚合器
    log_aggregator: Arc<logging::LogAggregator>,
    /// 性能监控器
    performance_monitor: Arc<performance::PerformanceMonitor>,
}

impl SystemManager {
    /// 创建新的系统管理器
    pub async fn new(config_path: &str) -> Result<Self> {
        // 初始化配置管理器
        let config_manager = Arc::new(config::ConfigManager::new(config_path, true)?);

        // 初始化系统监控器
        let system_monitor = Arc::new(monitoring::SystemMonitor::new()?);

        // 初始化告警管理器
        let notification_sender = Arc::new(alerting::DefaultNotificationSender);
        let metric_provider = system_monitor.clone() as Arc<dyn alerting::MetricProvider + Send + Sync>;
        let alert_manager = Arc::new(alerting::AlertManager::new(
            notification_sender,
            metric_provider,
        ));

        // 初始化日志聚合器
        let log_aggregator = Arc::new(logging::LogAggregator::default());

        // 初始化性能监控器
        let performance_monitor = Arc::new(performance::PerformanceMonitor::default());

        Ok(Self {
            config_manager,
            system_monitor,
            alert_manager,
            log_aggregator,
            performance_monitor,
        })
    }

    /// 启动系统管理器
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting system management services");

        // 启动配置热更新
        self.config_manager.start_hot_reload().await?;

        // 启动性能监控
        self.start_performance_monitoring().await?;

        // 启动告警评估
        self.start_alert_evaluation().await?;

        tracing::info!("System management services started successfully");
        Ok(())
    }

    /// 停止系统管理器
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping system management services");

        // 这里可以添加清理逻辑

        tracing::info!("System management services stopped");
        Ok(())
    }

    /// 获取配置管理器
    pub fn config_manager(&self) -> &Arc<config::ConfigManager> {
        &self.config_manager
    }

    /// 获取系统监控器
    pub fn system_monitor(&self) -> &Arc<monitoring::SystemMonitor> {
        &self.system_monitor
    }

    /// 获取告警管理器
    pub fn alert_manager(&self) -> &Arc<alerting::AlertManager> {
        &self.alert_manager
    }

    /// 获取日志聚合器
    pub fn log_aggregator(&self) -> &Arc<logging::LogAggregator> {
        &self.log_aggregator
    }

    /// 获取性能监控器
    pub fn performance_monitor(&self) -> &Arc<performance::PerformanceMonitor> {
        &self.performance_monitor
    }

    /// 启动性能监控
    async fn start_performance_monitoring(&self) -> Result<()> {
        let monitor = self.performance_monitor.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(30)
            );

            loop {
                interval.tick().await;

                if let Err(e) = monitor.collect_metrics().await {
                    tracing::error!("Failed to collect performance metrics: {}", e);
                }
            }
        });

        Ok(())
    }

    /// 启动告警评估
    async fn start_alert_evaluation(&self) -> Result<()> {
        let alert_manager = self.alert_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(60)
            );

            loop {
                interval.tick().await;

                if let Err(e) = alert_manager.evaluate_rules().await {
                    tracing::error!("Failed to evaluate alert rules: {}", e);
                }
            }
        });

        Ok(())
    }

    /// 生成系统状态报告
    pub async fn generate_status_report(&self) -> Result<SystemStatusReport> {
        let health_status = self.system_monitor.get_health_status().await;
        let performance_metrics = self.performance_monitor.get_current_metrics().await;
        let alert_stats = self.alert_manager.get_alert_stats().await;
        let log_stats = self.log_aggregator.get_log_stats(None).await?;

        Ok(SystemStatusReport {
            timestamp: chrono::Utc::now(),
            health_status,
            performance_metrics,
            alert_stats,
            log_stats,
        })
    }
}

/// 系统状态报告
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemStatusReport {
    /// 报告生成时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 健康状态
    pub health_status: monitoring::HealthStatus,
    /// 性能指标
    pub performance_metrics: performance::PerformanceMetrics,
    /// 告警统计
    pub alert_stats: alerting::AlertStats,
    /// 日志统计
    pub log_stats: logging::LogStats,
}

// 实现MetricProvider trait for SystemMonitor
#[async_trait::async_trait]
impl alerting::MetricProvider for monitoring::SystemMonitor {
    async fn get_metric_value(&self, metric_name: &str) -> Result<f64> {
        match metric_name {
            "cpu_usage" => Ok(45.0), // 模拟数据
            "memory_usage" => Ok(60.0), // 模拟数据
            "disk_usage" => Ok(70.0), // 模拟数据
            "active_connections" => Ok(25.0), // 模拟数据
            _ => Err(anyhow::anyhow!("Unknown metric: {}", metric_name)),
        }
    }

    async fn get_all_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("cpu_usage".to_string(), 45.0);
        metrics.insert("memory_usage".to_string(), 60.0);
        metrics.insert("disk_usage".to_string(), 70.0);
        metrics.insert("active_connections".to_string(), 25.0);
        Ok(metrics)
    }
}
