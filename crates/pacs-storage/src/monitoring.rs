//! 存储空间监控

use pacs_core::{PacsError, Result};
use crate::storage::{StorageManager, StorageConfig, StorageType, StorageStats};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 监控指标类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    /// 存储使用率
    StorageUsage,
    /// 文件数量
    FileCount,
    /// 可用空间
    AvailableSpace,
    /// 读写速度
    Throughput,
    /// 错误率
    ErrorRate,
    /// 响应时间
    ResponseTime,
}

/// 告警级别
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertLevel {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 严重
    Critical,
}

/// 监控指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标值
    pub value: f64,
    /// 单位
    pub unit: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 标签
    pub labels: HashMap<String, String>,
}

/// 告警规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// 规则名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 告警级别
    pub level: AlertLevel,
    /// 阈值
    pub threshold: f64,
    /// 比较操作符
    pub operator: ComparisonOperator,
    /// 持续时间（秒）
    pub duration: u64,
    /// 是否启用
    pub enabled: bool,
}

/// 比较操作符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    /// 大于
    GreaterThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessThanOrEqual,
    /// 等于
    Equal,
    /// 不等于
    NotEqual,
}

/// 告警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// 告警ID
    pub id: String,
    /// 规则名称
    pub rule_name: String,
    /// 告警级别
    pub level: AlertLevel,
    /// 消息
    pub message: String,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 是否活跃
    pub active: bool,
}

/// 存储性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 读操作计数
    pub read_operations: u64,
    /// 写操作计数
    pub write_operations: u64,
    /// 读操作总耗时（毫秒）
    pub total_read_time_ms: u64,
    /// 写操作总耗时（毫秒）
    pub total_write_time_ms: u64,
    /// 错误计数
    pub error_count: u64,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 监控间隔（秒）
    pub interval_seconds: u64,
    /// 指标保留时间（小时）
    pub retention_hours: u64,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
    /// 告警规则
    pub alert_rules: Vec<AlertRule>,
}

/// 存储监控器
pub struct StorageMonitor {
    /// 存储管理器
    storage_managers: HashMap<String, StorageManager>,
    /// 监控配置
    config: MonitoringConfig,
    /// 指标历史
    metrics_history: Arc<RwLock<Vec<Metric>>>,
    /// 性能指标
    performance_metrics: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
    /// 活跃告警
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// 告警历史
    alert_history: Vec<Alert>,
}

impl StorageMonitor {
    /// 创建新的存储监控器
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            storage_managers: HashMap::new(),
            config,
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Vec::new(),
        }
    }

    /// 添加存储管理器
    pub fn add_storage_manager(&mut self, name: String, storage_manager: StorageManager) {
        self.storage_managers.insert(name.clone(), storage_manager);

        // 初始化性能指标
        let perf_metrics = PerformanceMetrics {
            read_operations: 0,
            write_operations: 0,
            total_read_time_ms: 0,
            total_write_time_ms: 0,
            error_count: 0,
            last_updated: Utc::now(),
        };

        tokio::spawn({
            let metrics = self.performance_metrics.clone();
            let name_clone = name.clone();
            async move {
                let mut metrics_guard = metrics.write().await;
                metrics_guard.insert(name_clone, perf_metrics);
            }
        });
    }

    /// 启动监控
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting storage monitoring with interval: {} seconds", self.config.interval_seconds);

        let mut interval = interval(tokio::time::Duration::from_secs(self.config.interval_seconds));

        loop {
            interval.tick().await;

            // 收集存储指标
            if let Err(e) = self.collect_storage_metrics().await {
                error!("Error collecting storage metrics: {}", e);
            }

            // 检查告警规则
            if let Err(e) = self.check_alert_rules().await {
                error!("Error checking alert rules: {}", e);
            }

            // 清理过期指标
            if let Err(e) = self.cleanup_expired_metrics().await {
                error!("Error cleaning up expired metrics: {}", e);
            }
        }
    }

    /// 收集存储指标
    async fn collect_storage_metrics(&self) -> Result<()> {
        let timestamp = Utc::now();

        for (name, storage_manager) in &self.storage_managers {
            // 获取存储统计信息
            match storage_manager.get_storage_stats().await {
                Ok(stats) => {
                    // 存储使用率指标
                    let usage_metric = Metric {
                        name: "storage_usage".to_string(),
                        metric_type: MetricType::StorageUsage,
                        value: if stats.total_size > 0 {
                            (stats.total_size as f64 - stats.available_space.unwrap_or(0) as f64) / stats.total_size as f64 * 100.0
                        } else {
                            0.0
                        },
                        unit: "percent".to_string(),
                        timestamp,
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("storage_name".to_string(), name.clone());
                            labels.insert("storage_type".to_string(), format!("{:?}", storage_manager.storage_type()));
                            labels
                        },
                    };

                    // 文件数量指标
                    let file_count_metric = Metric {
                        name: "file_count".to_string(),
                        metric_type: MetricType::FileCount,
                        value: stats.total_files as f64,
                        unit: "count".to_string(),
                        timestamp,
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("storage_name".to_string(), name.clone());
                            labels
                        },
                    };

                    // 可用空间指标
                    let available_space_metric = Metric {
                        name: "available_space".to_string(),
                        metric_type: MetricType::AvailableSpace,
                        value: stats.available_space.unwrap_or(0) as f64,
                        unit: "bytes".to_string(),
                        timestamp,
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("storage_name".to_string(), name.clone());
                            labels
                        },
                    };

                    // 添加到指标历史
                    let mut metrics_guard = self.metrics_history.write().await;
                    metrics_guard.push(usage_metric);
                    metrics_guard.push(file_count_metric);
                    metrics_guard.push(available_space_metric);

                    debug!("Collected metrics for storage: {}", name);
                }
                Err(e) => {
                    error!("Failed to collect metrics for storage {}: {}", name, e);
                    self.record_error(name).await;
                }
            }

            // 收集性能指标
            if self.config.enable_performance_monitoring {
                self.collect_performance_metrics(name).await;
            }
        }

        Ok(())
    }

    /// 收集性能指标
    async fn collect_performance_metrics(&self, storage_name: &str) {
        let metrics_guard = self.performance_metrics.read().await;
        if let Some(perf_metrics) = metrics_guard.get(storage_name) {
            // 计算平均响应时间
            let avg_read_time = if perf_metrics.read_operations > 0 {
                perf_metrics.total_read_time_ms as f64 / perf_metrics.read_operations as f64
            } else {
                0.0
            };

            let avg_write_time = if perf_metrics.write_operations > 0 {
                perf_metrics.total_write_time_ms as f64 / perf_metrics.write_operations as f64
            } else {
                0.0
            };

            // 创建响应时间指标
            let read_time_metric = Metric {
                name: "read_response_time".to_string(),
                metric_type: MetricType::ResponseTime,
                value: avg_read_time,
                unit: "milliseconds".to_string(),
                timestamp: Utc::now(),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("storage_name".to_string(), storage_name.to_string());
                    labels.insert("operation".to_string(), "read".to_string());
                    labels
                },
            };

            let write_time_metric = Metric {
                name: "write_response_time".to_string(),
                metric_type: MetricType::ResponseTime,
                value: avg_write_time,
                unit: "milliseconds".to_string(),
                timestamp: Utc::now(),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("storage_name".to_string(), storage_name.to_string());
                    labels.insert("operation".to_string(), "write".to_string());
                    labels
                },
            };

            // 添加到指标历史
            let mut metrics_guard = self.metrics_history.write().await;
            metrics_guard.push(read_time_metric);
            metrics_guard.push(write_time_metric);
        }
    }

    /// 检查告警规则
    async fn check_alert_rules(&self) -> Result<()> {
        let metrics_guard = self.metrics_history.read().await;
        let mut active_alerts_guard = self.active_alerts.write().await;

        for rule in &self.config.alert_rules {
            if !rule.enabled {
                continue;
            }

            // 获取最近的指标
            let recent_metrics: Vec<&Metric> = metrics_guard
                .iter()
                .filter(|m| m.metric_type == rule.metric_type)
                .filter(|m| Utc::now() - m.timestamp <= Duration::seconds(rule.duration as i64))
                .collect();

            if recent_metrics.is_empty() {
                continue;
            }

            // 检查是否触发告警
            let latest_metric = recent_metrics[recent_metrics.len() - 1];
            let triggered = self.evaluate_condition(latest_metric.value, rule.threshold, &rule.operator);

            let alert_id = format!("{}_{}", rule.name, latest_metric.timestamp.timestamp());

            if triggered {
                if !active_alerts_guard.contains_key(&alert_id) {
                    // 创建新告警
                    let alert = Alert {
                        id: alert_id.clone(),
                        rule_name: rule.name.clone(),
                        level: rule.level.clone(),
                        message: format!("{} threshold breached: {} {} {}",
                                       rule.name,
                                       latest_metric.value,
                                       match rule.operator {
                                           ComparisonOperator::GreaterThan => ">",
                                           ComparisonOperator::GreaterThanOrEqual => ">=",
                                           ComparisonOperator::LessThan => "<",
                                           ComparisonOperator::LessThanOrEqual => "<=",
                                           ComparisonOperator::Equal => "=",
                                           ComparisonOperator::NotEqual => "!=",
                                       },
                                       rule.threshold),
                        current_value: latest_metric.value,
                        threshold: rule.threshold,
                        start_time: latest_metric.timestamp,
                        end_time: None,
                        active: true,
                    };

                    active_alerts_guard.insert(alert_id.clone(), alert);
                    warn!("Alert triggered: {}", latest_metric.value);
                }
            } else {
                // 检查是否需要关闭告警
                if let Some(alert) = active_alerts_guard.get_mut(&alert_id) {
                    alert.active = false;
                    alert.end_time = Some(Utc::now());
                    info!("Alert resolved: {}", alert_id);
                }
            }
        }

        Ok(())
    }

    /// 评估条件
    fn evaluate_condition(&self, current_value: f64, threshold: f64, operator: &ComparisonOperator) -> bool {
        match operator {
            ComparisonOperator::GreaterThan => current_value > threshold,
            ComparisonOperator::GreaterThanOrEqual => current_value >= threshold,
            ComparisonOperator::LessThan => current_value < threshold,
            ComparisonOperator::LessThanOrEqual => current_value <= threshold,
            ComparisonOperator::Equal => (current_value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (current_value - threshold).abs() >= f64::EPSILON,
        }
    }

    /// 记录错误
    async fn record_error(&self, storage_name: &str) {
        let mut metrics_guard = self.performance_metrics.write().await;
        if let Some(perf_metrics) = metrics_guard.get_mut(storage_name) {
            perf_metrics.error_count += 1;
            perf_metrics.last_updated = Utc::now();
        }
    }

    /// 清理过期指标
    async fn cleanup_expired_metrics(&mut self) -> Result<()> {
        let cutoff_time = Utc::now() - Duration::hours(self.config.retention_hours as i64);

        let mut metrics_guard = self.metrics_history.write().await;
        metrics_guard.retain(|m| m.timestamp > cutoff_time);

        // 清理非活跃告警
        let mut active_alerts_guard = self.active_alerts.write().await;
        let mut alerts_to_remove = Vec::new();

        for (alert_id, alert) in active_alerts_guard.iter() {
            if !alert.active {
                if let Some(end_time) = alert.end_time {
                    if Utc::now() - end_time > Duration::hours(24) { // 保留已解决告警24小时
                        alerts_to_remove.push(alert_id.clone());
                    }
                }
            }
        }

        for alert_id in alerts_to_remove {
            if let Some(alert) = active_alerts_guard.remove(&alert_id) {
                self.alert_history.push(alert);
            }
        }

        Ok(())
    }

    /// 记录读操作
    pub async fn record_read_operation(&self, storage_name: &str, duration_ms: u64) {
        let mut metrics_guard = self.performance_metrics.write().await;
        if let Some(perf_metrics) = metrics_guard.get_mut(storage_name) {
            perf_metrics.read_operations += 1;
            perf_metrics.total_read_time_ms += duration_ms;
            perf_metrics.last_updated = Utc::now();
        }
    }

    /// 记录写操作
    pub async fn record_write_operation(&self, storage_name: &str, duration_ms: u64) {
        let mut metrics_guard = self.performance_metrics.write().await;
        if let Some(perf_metrics) = metrics_guard.get_mut(storage_name) {
            perf_metrics.write_operations += 1;
            perf_metrics.total_write_time_ms += duration_ms;
            perf_metrics.last_updated = Utc::now();
        }
    }

    /// 获取最近的指标
    pub async fn get_recent_metrics(&self, metric_type: &MetricType, duration_hours: u64) -> Result<Vec<Metric>> {
        let cutoff_time = Utc::now() - Duration::hours(duration_hours as i64);
        let metrics_guard = self.metrics_history.read().await;

        let recent_metrics: Vec<Metric> = metrics_guard
            .iter()
            .filter(|m| &m.metric_type == metric_type && m.timestamp > cutoff_time)
            .cloned()
            .collect();

        Ok(recent_metrics)
    }

    /// 获取活跃告警
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        let active_alerts_guard = self.active_alerts.read().await;
        Ok(active_alerts_guard.values().cloned().collect())
    }

    /// 获取性能指标
    pub async fn get_performance_metrics(&self, storage_name: &str) -> Option<PerformanceMetrics> {
        let metrics_guard = self.performance_metrics.read().await;
        metrics_guard.get(storage_name).cloned()
    }

    /// 创建默认监控配置
    pub fn create_default_config() -> MonitoringConfig {
        MonitoringConfig {
            interval_seconds: 300, // 5分钟
            retention_hours: 24 * 7, // 7天
            enable_performance_monitoring: true,
            alert_rules: vec![
                AlertRule {
                    name: "high_storage_usage".to_string(),
                    metric_type: MetricType::StorageUsage,
                    level: AlertLevel::Warning,
                    threshold: 80.0,
                    operator: ComparisonOperator::GreaterThanOrEqual,
                    duration: 300, // 5分钟
                    enabled: true,
                },
                AlertRule {
                    name: "critical_storage_usage".to_string(),
                    metric_type: MetricType::StorageUsage,
                    level: AlertLevel::Critical,
                    threshold: 90.0,
                    operator: ComparisonOperator::GreaterThanOrEqual,
                    duration: 60, // 1分钟
                    enabled: true,
                },
                AlertRule {
                    name: "low_available_space".to_string(),
                    metric_type: MetricType::AvailableSpace,
                    level: AlertLevel::Warning,
                    threshold: 10.0 * 1024.0 * 1024.0 * 1024.0, // 10GB
                    operator: ComparisonOperator::LessThan,
                    duration: 300,
                    enabled: true,
                },
            ],
        }
    }
}