//! 告警系统
//!
//! 提供智能告警功能，支持多种告警规则、通知渠道和升级机制

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use super::monitoring::{AlertRule, AlertEvent, AlertSeverity, ComparisonOperator, NotificationConfig};

/// 告警管理器
pub struct AlertManager {
    /// 告警规则
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    /// 活跃告警
    active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    /// 告警历史
    alert_history: Arc<RwLock<Vec<AlertEvent>>>,
    /// 通知发送器
    notification_sender: Arc<dyn NotificationSender + Send + Sync>,
    /// 指标获取器
    metric_provider: Arc<dyn MetricProvider + Send + Sync>,
}

impl std::fmt::Debug for AlertManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlertManager")
            .field("rules_count", &self.rules.read().await.len())
            .field("active_alerts_count", &self.active_alerts.read().await.len())
            .field("alert_history_count", &self.alert_history.read().await.len())
            .finish()
    }
}

/// 活跃告警
#[derive(Debug)]
struct ActiveAlert {
    /// 告警事件
    event: AlertEvent,
    /// 首次触发时间
    first_triggered: Instant,
    /// 最后触发时间
    last_triggered: Instant,
    /// 连续触发次数
    trigger_count: u64,
}

/// 通知发送器特征
#[async_trait::async_trait]
pub trait NotificationSender {
    /// 发送告警通知
    async fn send_alert(&self, alert: &AlertEvent, config: &NotificationConfig) -> Result<()>;
}

/// 指标提供者特征
#[async_trait::async_trait]
pub trait MetricProvider {
    /// 获取指标值
    async fn get_metric_value(&self, metric_name: &str) -> Result<f64>;
    /// 获取所有指标
    async fn get_all_metrics(&self) -> Result<HashMap<String, f64>>;
}

/// 告警评估器
pub struct AlertEvaluator {
    rules: Vec<AlertRule>,
    metric_provider: Arc<dyn MetricProvider + Send + Sync>,
}

/// 告警统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    /// 总告警数
    pub total_alerts: u64,
    /// 当前活跃告警数
    pub active_alerts: u64,
    /// 今日告警数
    pub alerts_today: u64,
    /// 本周告警数
    pub alerts_this_week: u64,
    /// 按严重级别统计
    pub alerts_by_severity: HashMap<AlertSeverity, u64>,
    /// 按规则统计
    pub alerts_by_rule: HashMap<String, u64>,
}

/// 告警聚合信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAggregation {
    /// 时间窗口
    pub time_window: Duration,
    /// 聚合规则
    pub aggregation_rule: AggregationRule,
    /// 聚合结果
    pub aggregated_alerts: Vec<AggregatedAlert>,
}

/// 聚合规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationRule {
    /// 按严重级别聚合
    BySeverity,
    /// 按规则名称聚合
    ByRule,
    /// 按时间聚合
    ByTime,
    /// 按组件聚合
    ByComponent,
}

/// 聚合告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedAlert {
    /// 聚合键
    pub key: String,
    /// 告警数量
    pub count: u64,
    /// 首次告警时间
    pub first_occurrence: chrono::DateTime<chrono::Utc>,
    /// 最后告警时间
    pub last_occurrence: chrono::DateTime<chrono::Utc>,
    /// 告警严重级别（取最高）
    pub severity: AlertSeverity,
    /// 示例告警消息
    pub sample_message: String,
}

impl AlertManager {
    /// 创建新的告警管理器
    pub fn new(
        notification_sender: Arc<dyn NotificationSender + Send + Sync>,
        metric_provider: Arc<dyn MetricProvider + Send + Sync>,
    ) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notification_sender,
            metric_provider,
        }
    }

    /// 添加告警规则
    pub async fn add_rule(&self, rule: AlertRule) -> Result<()> {
        let mut rules = self.rules.write().await;
        rules.insert(rule.name.clone(), rule);
        info!("Added alert rule: {}", rule.name);
        Ok(())
    }

    /// 删除告警规则
    pub async fn remove_rule(&self, rule_name: &str) -> Result<bool> {
        let mut rules = self.rules.write().await;
        let removed = rules.remove(rule_name).is_some();
        if removed {
            info!("Removed alert rule: {}", rule_name);
        }
        Ok(removed)
    }

    /// 获取所有告警规则
    pub async fn get_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    /// 评估所有告警规则
    pub async fn evaluate_rules(&self) -> Result<Vec<AlertEvent>> {
        let rules = self.rules.read().await;
        let mut triggered_alerts = Vec::new();

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let Ok(alert) = self.evaluate_rule(rule).await {
                triggered_alerts.push(alert);
            }
        }

        Ok(triggered_alerts)
    }

    /// 评估单个告警规则
    async fn evaluate_rule(&self, rule: &AlertRule) -> Result<AlertEvent> {
        let current_value = self.metric_provider.get_metric_value(&rule.metric).await
            .with_context(|| format!("Failed to get metric value for: {}", rule.metric))?;

        let triggered = self.check_threshold(current_value, rule.threshold, &rule.operator);

        if triggered {
            let message = self.format_alert_message(rule, current_value);
            let alert = AlertEvent {
                id: Uuid::new_v4().to_string(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                current_value,
                threshold: rule.threshold,
                message,
                timestamp: chrono::Utc::now(),
                resolved: false,
            };

            self.handle_triggered_alert(&alert, rule).await?;
            Ok(alert)
        } else {
            // 检查是否需要解决现有的告警
            self.resolve_alert_if_exists(&rule.name).await?;

            // 返回一个已解决的告警事件
            Ok(AlertEvent {
                id: Uuid::new_v4().to_string(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                current_value,
                threshold: rule.threshold,
                message: format!("Alert condition resolved for {}", rule.name),
                timestamp: chrono::Utc::now(),
                resolved: true,
            })
        }
    }

    /// 检查阈值条件
    fn check_threshold(&self, current: f64, threshold: f64, operator: &ComparisonOperator) -> bool {
        match operator {
            ComparisonOperator::GreaterThan => current > threshold,
            ComparisonOperator::LessThan => current < threshold,
            ComparisonOperator::Equals => (current - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEquals => (current - threshold).abs() >= f64::EPSILON,
            ComparisonOperator::GreaterThanOrEqual => current >= threshold,
            ComparisonOperator::LessThanOrEqual => current <= threshold,
        }
    }

    /// 格式化告警消息
    fn format_alert_message(&self, rule: &AlertRule, current_value: f64) -> String {
        rule.message_template
            .replace("{metric}", &rule.metric)
            .replace("{threshold}", &rule.threshold.to_string())
            .replace("{current}", &current_value.to_string())
            .replace("{severity}", &format!("{:?}", rule.severity))
    }

    /// 处理触发的告警
    async fn handle_triggered_alert(&self, alert: &AlertEvent, rule: &AlertRule) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        let alert_key = &alert.rule_name;

        match active_alerts.get_mut(alert_key) {
            Some(active_alert) => {
                // 更新现有告警
                active_alert.last_triggered = Instant::now();
                active_alert.trigger_count += 1;

                // 检查是否需要再次发送通知（重试逻辑）
                if self.should_resend_notification(active_alert, rule).await {
                    self.send_alert_notification(alert, &rule.notifications).await?;
                }
            }
            None => {
                // 新告警
                let active_alert = ActiveAlert {
                    event: alert.clone(),
                    first_triggered: Instant::now(),
                    last_triggered: Instant::now(),
                    trigger_count: 1,
                };
                active_alerts.insert(alert_key.to_string(), active_alert);

                // 发送新告警通知
                self.send_alert_notification(alert, &rule.notifications).await?;
            }
        }

        // 记录到历史
        self.record_alert_event(alert).await;

        Ok(())
    }

    /// 判断是否需要重新发送通知
    async fn should_resend_notification(&self, active_alert: &ActiveAlert, rule: &AlertRule) -> bool {
        // 简单的重试策略：每10分钟重试一次，最多重试3次
        let retry_interval = Duration::from_secs(600); // 10分钟
        let max_retries = 3;

        active_alert.trigger_count <= max_retries &&
        active_alert.last_triggered.elapsed() >= retry_interval
    }

    /// 发送告警通知
    async fn send_alert_notification(&self, alert: &AlertEvent, notification_config: &NotificationConfig) -> Result<()> {
        match self.notification_sender.send_alert(alert, notification_config).await {
            Ok(()) => {
                info!("Alert notification sent successfully: {}", alert.id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to send alert notification {}: {}", alert.id, e);
                Err(e)
            }
        }
    }

    /// 解决告警（如果存在）
    async fn resolve_alert_if_exists(&self, rule_name: &str) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;

        if let Some(mut active_alert) = active_alerts.remove(rule_name) {
            active_alert.event.resolved = true;
            active_alert.event.timestamp = chrono::Utc::now();

            info!("Alert resolved: {}", rule_name);
            self.record_alert_event(&active_alert.event).await;
        }

        Ok(())
    }

    /// 记录告警事件
    async fn record_alert_event(&self, alert: &AlertEvent) {
        let mut history = self.alert_history.write().await;
        history.push(alert.clone());

        // 限制历史记录数量，保留最近10000条
        if history.len() > 10000 {
            history.drain(0..1000);
        }
    }

    /// 获取活跃告警
    pub async fn get_active_alerts(&self) -> Vec<AlertEvent> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values()
            .map(|active| active.event.clone())
            .collect()
    }

    /// 获取告警历史
    pub async fn get_alert_history(&self, limit: Option<usize>) -> Vec<AlertEvent> {
        let history = self.alert_history.read().await;
        match limit {
            Some(limit) => history.iter().rev().take(limit).cloned().collect(),
            None => history.iter().rev().cloned().collect(),
        }
    }

    /// 获取告警统计信息
    pub async fn get_alert_stats(&self) -> AlertStats {
        let active_alerts = self.active_alerts.read().await;
        let history = self.alert_history.read().await;

        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let week_start = today_start - Duration::from_secs(7 * 24 * 60 * 60);

        let mut alerts_today = 0;
        let mut alerts_this_week = 0;
        let mut alerts_by_severity = HashMap::new();
        let mut alerts_by_rule = HashMap::new();

        for alert in history.iter() {
            if alert.timestamp >= today_start {
                alerts_today += 1;
            }
            if alert.timestamp >= week_start {
                alerts_this_week += 1;
            }

            *alerts_by_severity.entry(alert.severity.clone()).or_insert(0) += 1;
            *alerts_by_rule.entry(alert.rule_name.clone()).or_insert(0) += 1;
        }

        AlertStats {
            total_alerts: history.len() as u64,
            active_alerts: active_alerts.len() as u64,
            alerts_today,
            alerts_this_week,
            alerts_by_severity,
            alerts_by_rule,
        }
    }

    /// 清理过期的活跃告警
    pub async fn cleanup_expired_alerts(&self, max_age: Duration) -> Result<usize> {
        let mut active_alerts = self.active_alerts.write().await;
        let initial_count = active_alerts.len();

        active_alerts.retain(|_, active| {
            active.first_triggered.elapsed() < max_age
        });

        let removed_count = initial_count - active_alerts.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired alerts", removed_count);
        }

        Ok(removed_count)
    }

    /// 手动解决告警
    pub async fn manually_resolve_alert(&self, alert_id: &str) -> Result<bool> {
        let mut active_alerts = self.active_alerts.write().await;

        for (rule_name, active_alert) in active_alerts.iter_mut() {
            if active_alert.event.id == alert_id {
                active_alert.event.resolved = true;
                active_alert.event.timestamp = chrono::Utc::now();

                info!("Manually resolved alert: {}", alert_id);
                self.record_alert_event(&active_alert.event).await;

                active_alerts.remove(rule_name);
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl AlertEvaluator {
    /// 创建新的告警评估器
    pub fn new(
        rules: Vec<AlertRule>,
        metric_provider: Arc<dyn MetricProvider + Send + Sync>,
    ) -> Self {
        Self {
            rules,
            metric_provider,
        }
    }

    /// 评估所有规则
    pub async fn evaluate(&self) -> Result<Vec<AlertEvent>> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // 这里实现具体的评估逻辑
            // 可以根据需要添加复杂的规则评估
        }

        Ok(alerts)
    }
}

/// 默认通知发送器实现
pub struct DefaultNotificationSender;

#[async_trait::async_trait]
impl NotificationSender for DefaultNotificationSender {
    async fn send_alert(&self, alert: &AlertEvent, config: &NotificationConfig) -> Result<()> {
        // 实现默认的通知逻辑
        info!("Alert notification: {}", alert.message);

        // 这里可以添加实际的邮件、Webhook、短信发送逻辑
        // 暂时只记录日志

        Ok(())
    }
}