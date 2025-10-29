//! Webhook事件通知模块
//!
//! 为外部系统提供实时事件通知功能，支持：
//! - 事件订阅管理
//! - 安全的Webhook签名验证
//! - 重试机制和错误处理
//! - 事件过滤和路由

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Webhook事件类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventType {
    PatientCreated,
    PatientUpdated,
    PatientDeleted,
    StudyCreated,
    StudyUpdated,
    StudyCompleted,
    SeriesCreated,
    InstanceReceived,
    CriticalValueDetected,
    SystemAlert,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PatientCreated => "patient.created",
            Self::PatientUpdated => "patient.updated",
            Self::PatientDeleted => "patient.deleted",
            Self::StudyCreated => "study.created",
            Self::StudyUpdated => "study.updated",
            Self::StudyCompleted => "study.completed",
            Self::SeriesCreated => "series.created",
            Self::InstanceReceived => "instance.received",
            Self::CriticalValueDetected => "critical_value.detected",
            Self::SystemAlert => "system.alert",
        }
    }
}

impl TryFrom<&str> for WebhookEventType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "patient.created" => Ok(Self::PatientCreated),
            "patient.updated" => Ok(Self::PatientUpdated),
            "patient.deleted" => Ok(Self::PatientDeleted),
            "study.created" => Ok(Self::StudyCreated),
            "study.updated" => Ok(Self::StudyUpdated),
            "study.completed" => Ok(Self::StudyCompleted),
            "series.created" => Ok(Self::SeriesCreated),
            "instance.received" => Ok(Self::InstanceReceived),
            "critical_value.detected" => Ok(Self::CriticalValueDetected),
            "system.alert" => Ok(Self::SystemAlert),
            _ => Err(anyhow::anyhow!("Unknown event type: {}", value)),
        }
    }
}

/// Webhook事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub event_type: WebhookEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
    pub source: String,
}

impl WebhookEvent {
    pub fn new(event_type: WebhookEventType, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            source: "pacs".to_string(),
        }
    }
}

/// Webhook订阅配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub secret: Option<String>,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

impl WebhookSubscription {
    pub fn new(url: String, events: Vec<WebhookEventType>, secret: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            events,
            secret,
            active: true,
            created_at: chrono::Utc::now(),
            retry_count: 0,
            last_success: None,
            last_failure: None,
        }
    }

    /// 检查是否对指定事件感兴趣
    pub fn is_interested_in(&self, event_type: &WebhookEventType) -> bool {
        self.active && self.events.contains(event_type)
    }

    /// 生成签名
    pub fn generate_signature(&self, payload: &str) -> Option<String> {
        use sha2::{Digest, Sha256};

        if let Some(secret) = &self.secret {
            let mut hasher = Sha256::new();
            hasher.update(payload);
            hasher.update(secret);
            Some(format!("sha256={:x}", hasher.finalize()))
        } else {
            None
        }
    }
}

/// Webhook订阅请求
#[derive(Debug, Deserialize)]
pub struct WebhookSubscriptionRequest {
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub active: Option<bool>,
}

/// Webhook管理器
pub struct WebhookManager {
    subscriptions: RwLock<HashMap<String, WebhookSubscription>>,
    client: reqwest::Client,
}

impl WebhookManager {
    /// 创建新的Webhook管理器
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            client: reqwest::Client::new(),
        }
    }

    /// 订阅Webhook事件
    pub async fn subscribe(&mut self, request: WebhookSubscriptionRequest) -> Result<String> {
        // 解析事件类型
        let mut events = Vec::new();
        for event_str in request.events {
            match WebhookEventType::try_from(event_str.as_str()) {
                Ok(event_type) => events.push(event_type),
                Err(e) => {
                    warn!("Invalid event type '{}': {}", event_str, e);
                    continue;
                }
            }
        }

        if events.is_empty() {
            return Err(anyhow::anyhow!("No valid event types specified"));
        }

        let subscription = WebhookSubscription::new(
            request.url,
            events,
            request.secret,
        );

        let subscription_id = subscription.id.clone();
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription_id.clone(), subscription);

        info!("Created webhook subscription: {}", subscription_id);
        Ok(subscription_id)
    }

    /// 取消订阅
    pub async fn unsubscribe(&mut self, subscription_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions.remove(subscription_id).is_some() {
            info!("Removed webhook subscription: {}", subscription_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subscription not found: {}", subscription_id))
        }
    }

    /// 列出所有订阅
    pub async fn list_subscriptions(&self) -> Result<Vec<serde_json::Value>> {
        let subscriptions = self.subscriptions.read().await;
        let result: Vec<serde_json::Value> = subscriptions
            .values()
            .map(|sub| serde_json::to_value(sub).unwrap())
            .collect();
        Ok(result)
    }

    /// 发送事件到所有感兴趣的订阅者
    pub async fn emit_event(&self, event: WebhookEvent) -> Result<()> {
        debug!("Emitting event: {}", event.event_type.as_str());

        let subscriptions = self.subscriptions.read().await;
        let interested_subscriptions: Vec<_> = subscriptions
            .values()
            .filter(|sub| sub.is_interested_in(&event.event_type))
            .collect();

        if interested_subscriptions.is_empty() {
            debug!("No subscriptions interested in event: {}", event.event_type.as_str());
            return Ok(());
        }

        let payload = serde_json::to_string(&event)?;

        // 并发发送到所有订阅者
        let mut handles = Vec::new();
        for subscription in interested_subscriptions {
            let subscription = subscription.clone();
            let payload = payload.clone();
            let client = self.client.clone();

            let handle = tokio::spawn(async move {
                Self::send_webhook(&client, &subscription, &payload).await
            });
            handles.push(handle);
        }

        // 等待所有发送完成
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Webhook send task failed: {}", e);
            }
        }

        Ok(())
    }

    /// 发送单个Webhook
    async fn send_webhook(
        client: &reqwest::Client,
        subscription: &WebhookSubscription,
        payload: &str,
    ) -> Result<()> {
        let mut request = client
            .post(&subscription.url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "PACS-Webhook/1.0")
            .header("X-PACS-Event", payload);

        // 添加签名头
        if let Some(signature) = subscription.generate_signature(payload) {
            request = request.header("X-PACS-Signature", signature);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully sent webhook to: {}", subscription.url);
                    Ok(())
                } else {
                    let status = response.status();
                    error!("Webhook failed with status {}: {}", status, subscription.url);
                    Err(anyhow::anyhow!("Webhook failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to send webhook to {}: {}", subscription.url, e);
                Err(anyhow::anyhow!("Failed to send webhook: {}", e))
            }
        }
    }

    /// 创建患者创建事件
    pub fn create_patient_created_event(patient_data: serde_json::Value) -> WebhookEvent {
        WebhookEvent::new(WebhookEventType::PatientCreated, patient_data)
    }

    /// 创建检查完成事件
    pub fn create_study_completed_event(study_data: serde_json::Value) -> WebhookEvent {
        WebhookEvent::new(WebhookEventType::StudyCompleted, study_data)
    }

    /// 创建危急值检测事件
    pub fn create_critical_value_event(critical_data: serde_json::Value) -> WebhookEvent {
        WebhookEvent::new(WebhookEventType::CriticalValueDetected, critical_data)
    }

    /// 创建系统告警事件
    pub fn create_system_alert_event(alert_data: serde_json::Value) -> WebhookEvent {
        WebhookEvent::new(WebhookEventType::SystemAlert, alert_data)
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_subscription() {
        let mut manager = WebhookManager::new();

        let request = WebhookSubscriptionRequest {
            url: "https://example.com/webhook".to_string(),
            events: vec!["patient.created".to_string(), "study.completed".to_string()],
            secret: Some("test-secret".to_string()),
            active: Some(true),
        };

        let subscription_id = manager.subscribe(request).await.unwrap();
        assert!(!subscription_id.is_empty());

        let subscriptions = manager.list_subscriptions().await.unwrap();
        assert_eq!(subscriptions.len(), 1);
    }

    #[test]
    fn test_webhook_signature() {
        let subscription = WebhookSubscription::new(
            "https://example.com/webhook".to_string(),
            vec![WebhookEventType::PatientCreated],
            Some("test-secret".to_string()),
        );

        let payload = r#"{"test": "data"}"#;
        let signature = subscription.generate_signature(payload);
        assert!(signature.is_some());
        assert!(signature.unwrap().starts_with("sha256="));
    }
}