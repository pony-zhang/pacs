//! 消息队列集成模块
//!
//! 提供可靠的消息传递机制，支持：
//! - RabbitMQ集成
//! - 消息发布和订阅
//! - 消息持久化和重试
//! - 死信队列处理

use anyhow::Result;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Channel,
    Connection, ConnectionProperties, Queue,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 消息队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueConfig {
    pub url: String,
    pub virtual_host: Option<String>,
    pub heartbeat: u16,
    pub connection_timeout: u16,
    pub prefetch_count: u16,
}

impl Default for MessageQueueConfig {
    fn default() -> Self {
        Self {
            url: "amqp://localhost:5672".to_string(),
            virtual_host: None,
            heartbeat: 60,
            connection_timeout: 30,
            prefetch_count: 10,
        }
    }
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    PatientUpdate,
    StudyCreated,
    StudyCompleted,
    SeriesReceived,
    InstanceProcessed,
    CriticalValueAlert,
    SystemNotification,
    WorkflowEvent,
    Custom(String),
}

impl MessageType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PatientUpdate => "patient.update",
            Self::StudyCreated => "study.created",
            Self::StudyCompleted => "study.completed",
            Self::SeriesReceived => "series.received",
            Self::InstanceProcessed => "instance.processed",
            Self::CriticalValueAlert => "critical_value.alert",
            Self::SystemNotification => "system.notification",
            Self::WorkflowEvent => "workflow.event",
            Self::Custom(name) => name,
        }
    }
}

/// 消息封装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub message_type: MessageType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub data: serde_json::Value,
    pub priority: u8,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl Message {
    pub fn new(message_type: MessageType, data: serde_json::Value, source: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            timestamp: chrono::Utc::now(),
            source,
            data,
            priority: 5, // 默认优先级
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) -> bool {
        self.retry_count += 1;
        self.retry_count < self.max_retries
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

/// 队列配置
#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub name: String,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub arguments: FieldTable,
}

impl QueueConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            durable: true,
            exclusive: false,
            auto_delete: false,
            arguments: FieldTable::default(),
        }
    }

    /// 创建临时队列配置
    pub fn temporary(name: &str) -> Self {
        Self {
            name: name.to_string(),
            durable: false,
            exclusive: false,
            auto_delete: true,
            arguments: FieldTable::default(),
        }
    }

    /// 设置TTL（消息存活时间，毫秒）
    pub fn with_ttl(mut self, ttl_ms: u32) -> Self {
        self.arguments.insert("x-message-ttl".into(), ttl_ms.into());
        self
    }

    /// 设置队列长度限制
    pub fn with_max_length(mut self, max_length: u32) -> Self {
        self.arguments
            .insert("x-max-length".into(), max_length.into());
        self
    }

    /// 设置死信队列
    pub fn with_dead_letter_exchange(mut self, exchange: &str, routing_key: Option<&str>) -> Self {
        use lapin::types::AMQPValue;
        self.arguments.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(exchange.to_string()),
        );
        if let Some(key) = routing_key {
            self.arguments.insert(
                "x-dead-letter-routing-key".into(),
                AMQPValue::LongString(key.to_string()),
            );
        }
        self
    }
}

/// 消息处理器接口
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理消息
    async fn handle_message(&self, message: &Message) -> Result<()>;

    /// 获取处理器名称
    fn name(&self) -> &str;
}

/// 消息发布器
pub struct MessagePublisher {
    channel: RwLock<Option<Channel>>,
    config: MessageQueueConfig,
}

impl MessagePublisher {
    /// 创建新的消息发布器
    pub fn new(config: MessageQueueConfig) -> Self {
        Self {
            channel: RwLock::new(None),
            config,
        }
    }

    /// 连接到消息队列
    pub async fn connect(&self) -> Result<()> {
        let conn = Connection::connect(
            &self.config.url,
            ConnectionProperties::default().with_heartbeat(self.config.heartbeat),
        )
        .await?;
        let channel = conn.create_channel().await?;

        // 设置QoS
        channel
            .basic_qos(self.config.prefetch_count, BasicQosOptions::default())
            .await?;

        let mut channel_lock = self.channel.write().await;
        *channel_lock = Some(channel);

        info!("Connected to message queue: {}", self.config.url);
        Ok(())
    }

    /// 发布消息
    pub async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        message: &Message,
    ) -> Result<()> {
        let channel_lock = self.channel.read().await;
        if let Some(channel) = channel_lock.as_ref() {
            let payload = serde_json::to_vec(message)?;
            let properties = BasicProperties::default()
                .with_content_type("application/json".into())
                .with_message_id(message.id.clone().into())
                .with_timestamp(message.timestamp.timestamp() as u64)
                .with_priority(message.priority);

            let confirm = channel
                .basic_publish(
                    exchange,
                    routing_key,
                    BasicPublishOptions::default(),
                    &payload,
                    properties,
                )
                .await?
                .await?;

            match confirm {
                Confirmation::Ack(_) => {
                    debug!("Message published successfully: {}", message.id);
                    Ok(())
                }
                Confirmation::Nack(nack) => {
                    error!("Message publish rejected: {:?}", nack);
                    Err(anyhow::anyhow!("Message publish rejected"))
                }
            }
        } else {
            Err(anyhow::anyhow!("Not connected to message queue"))
        }
    }

    /// 创建交换器
    pub async fn declare_exchange(
        &self,
        exchange: &str,
        exchange_type: lapin::ExchangeKind,
    ) -> Result<()> {
        let channel_lock = self.channel.read().await;
        if let Some(channel) = channel_lock.as_ref() {
            channel
                .exchange_declare(
                    exchange,
                    exchange_type,
                    ExchangeDeclareOptions::default(),
                    FieldTable::default(),
                )
                .await?;
            info!("Exchange declared: {}", exchange);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected to message queue"))
        }
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        let mut channel_lock = self.channel.write().await;
        *channel_lock = None;
        info!("Disconnected from message queue");
        Ok(())
    }
}

/// 消息订阅器
pub struct MessageSubscriber {
    channel: RwLock<Option<Channel>>,
    config: MessageQueueConfig,
    handlers: RwLock<HashMap<String, Box<dyn MessageHandler>>>,
}

impl MessageSubscriber {
    /// 创建新的消息订阅器
    pub fn new(config: MessageQueueConfig) -> Self {
        Self {
            channel: RwLock::new(None),
            config,
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// 连接到消息队列
    pub async fn connect(&self) -> Result<()> {
        let conn = Connection::connect(
            &self.config.url,
            ConnectionProperties::default().with_heartbeat(self.config.heartbeat),
        )
        .await?;
        let channel = conn.create_channel().await?;

        // 设置QoS
        channel
            .basic_qos(self.config.prefetch_count, BasicQosOptions::default())
            .await?;

        let mut channel_lock = self.channel.write().await;
        *channel_lock = Some(channel);

        info!("Connected to message queue: {}", self.config.url);
        Ok(())
    }

    /// 注册消息处理器
    pub async fn register_handler(&self, name: &str, handler: Box<dyn MessageHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(name.to_string(), handler);
        info!("Registered message handler: {}", name);
    }

    /// 声明队列
    pub async fn declare_queue(&self, queue_config: QueueConfig) -> Result<Queue> {
        let channel_lock = self.channel.read().await;
        if let Some(channel) = channel_lock.as_ref() {
            let queue = channel
                .queue_declare(
                    &queue_config.name,
                    QueueDeclareOptions {
                        durable: queue_config.durable,
                        exclusive: queue_config.exclusive,
                        auto_delete: queue_config.auto_delete,
                        ..QueueDeclareOptions::default()
                    },
                    queue_config.arguments,
                )
                .await?;
            info!("Queue declared: {}", queue_config.name);
            Ok(queue)
        } else {
            Err(anyhow::anyhow!("Not connected to message queue"))
        }
    }

    /// 绑定队列到交换器
    pub async fn bind_queue(
        &self,
        queue_name: &str,
        exchange: &str,
        routing_key: &str,
    ) -> Result<()> {
        let channel_lock = self.channel.read().await;
        if let Some(channel) = channel_lock.as_ref() {
            channel
                .queue_bind(
                    queue_name,
                    exchange,
                    routing_key,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
            info!(
                "Queue {} bound to {} with routing key {}",
                queue_name, exchange, routing_key
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected to message queue"))
        }
    }

    /// 开始消费消息
    pub async fn start_consuming(&self, queue_name: &str) -> Result<()> {
        let channel_lock = self.channel.read().await;
        if let Some(channel) = channel_lock.as_ref() {
            let consumer = channel
                .basic_consume(
                    queue_name,
                    "pacs-consumer",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await?;

            info!("Started consuming messages from queue: {}", queue_name);

            let handlers = self.handlers.clone();
            consumer.set_delegate(move |delivery| {
                let handlers = handlers.clone();
                Box::pin(async move {
                    if let Some(delivery) = delivery {
                        match Self::process_delivery(&handlers, delivery).await {
                            Ok(_) => {
                                // 消息处理成功，发送ACK
                                delivery.ack(BasicAckOptions::default()).await?;
                            }
                            Err(e) => {
                                error!("Failed to process message: {}", e);
                                // 检查是否可以重试
                                if let Ok(message_str) = std::str::from_utf8(&delivery.data) {
                                    if let Ok(mut message) =
                                        serde_json::from_str::<Message>(message_str)
                                    {
                                        if message.increment_retry() {
                                            // 可以重试，重新入队
                                            warn!(
                                                "Message retry {}/{}: {}",
                                                message.retry_count,
                                                message.max_retries,
                                                message.id
                                            );
                                            delivery
                                                .nack(BasicNackOptions::default().requeue(true))
                                                .await?;
                                        } else {
                                            // 超过最大重试次数，拒绝并丢弃
                                            error!(
                                                "Message max retries exceeded, dropping: {}",
                                                message.id
                                            );
                                            delivery
                                                .nack(BasicNackOptions::default().requeue(false))
                                                .await?;
                                        }
                                    } else {
                                        delivery
                                            .nack(BasicNackOptions::default().requeue(false))
                                            .await?;
                                    }
                                } else {
                                    delivery
                                        .nack(BasicNackOptions::default().requeue(false))
                                        .await?;
                                }
                            }
                        }
                    }
                    Ok(())
                })
            });

            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected to message queue"))
        }
    }

    /// 处理接收到的消息
    async fn process_delivery(
        handlers: &RwLock<HashMap<String, Box<dyn MessageHandler>>>,
        delivery: lapin::message::Delivery,
    ) -> Result<()> {
        let message_str = std::str::from_utf8(&delivery.data)?;
        let message: Message = serde_json::from_str(message_str)?;

        debug!(
            "Processing message: {} ({})",
            message.id,
            message.message_type.as_str()
        );

        // 根据消息类型找到对应的处理器
        let handler_name = match message.message_type {
            MessageType::PatientUpdate => "patient_update",
            MessageType::StudyCreated => "study_created",
            MessageType::StudyCompleted => "study_completed",
            MessageType::SeriesReceived => "series_received",
            MessageType::InstanceProcessed => "instance_processed",
            MessageType::CriticalValueAlert => "critical_value_alert",
            MessageType::SystemNotification => "system_notification",
            MessageType::WorkflowEvent => "workflow_event",
            MessageType::Custom(ref name) => name,
        };

        let handlers_lock = handlers.read().await;
        if let Some(handler) = handlers_lock.get(handler_name) {
            handler.handle_message(&message).await?;
            debug!(
                "Message processed successfully by handler: {}",
                handler_name
            );
        } else {
            warn!("No handler found for message type: {}", handler_name);
        }

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        let mut channel_lock = self.channel.write().await;
        *channel_lock = None;
        info!("Disconnected from message queue");
        Ok(())
    }
}

/// 默认消息处理器实现
pub struct DefaultMessageHandler {
    name: String,
}

impl DefaultMessageHandler {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl MessageHandler for DefaultMessageHandler {
    async fn handle_message(&self, message: &Message) -> Result<()> {
        info!(
            "Handling message {} with handler: {}",
            message.id, self.name
        );
        // 默认实现只是记录消息
        debug!(
            "Message data: {}",
            serde_json::to_string_pretty(&message.data)?
        );
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
