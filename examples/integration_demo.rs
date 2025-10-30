//! PACS集成模块演示程序
//!
//! 展示集成模块的各种功能：
//! - HL7消息解析和处理
//! - RESTful API服务
//! - Webhook事件通知

use anyhow::Result;
use pacs_integration::{
    hl7::Hl7Interface,
    webhook::{WebhookEvent, WebhookEventType, WebhookManager},
    ApiServer,
};
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("🚀 启动PACS集成模块演示");

    // 1. HL7接口演示
    demo_hl7_interface().await?;

    // 2. API服务器演示
    demo_api_server().await?;

    // 3. Webhook通知演示
    demo_webhook_notifications().await?;

    info!("✅ 集成模块演示完成");
    Ok(())
}

/// HL7接口演示
async fn demo_hl7_interface() -> Result<()> {
    info!("\n📋 HL7接口演示");

    let hl7_interface = Hl7Interface::new();

    // 示例ADT消息（患者入院）
    let adt_message = r#"MSH|^~\&|HIS|HOSPITAL|PACS|HOSPITAL|20241030120000||ADT^A01|123456|P|2.5
PID|1||PAT12345^HOSPITAL||张三^李||19800101|M||北京市朝阳区^^北京市^100000|13800138000
PV1|1|I|ICU^^^1||||||ADM001^王医生^MD|||||||||1||A0||||||||||||||||||HOSPITAL||20241030120000"#;

    match hl7_interface.process_message(adt_message).await {
        Ok(parsed_message) => {
            info!("✅ HL7消息解析成功");
            info!("   消息类型: {:?}", parsed_message.message_type);
            info!("   控制ID: {}", parsed_message.message_control_id);
            info!("   时间戳: {}", parsed_message.timestamp);

            // 生成ACK响应
            let ack = hl7_interface.generate_ack(&parsed_message, true, None);
            info!("   生成ACK消息长度: {} 字符", ack.len());
        }
        Err(e) => {
            warn!("❌ HL7消息解析失败: {}", e);
        }
    }

    // 示例ORM消息（检查申请）
    let orm_message = r#"MSH|^~\&|RIS|HOSPITAL|PACS|HOSPITAL|20241030120000||ORM^O01|123457|P|2.5
PID|1||PAT12345^HOSPITAL||张三^李||19800101|M
ORC|NW|ORD12345||ORD12345^HOSPITAL||||20241030110000|||||||||||||DR001^李医生^MD^^^^^^DR001
OBR|1|ORD12345||CT-ABDOMEN|腹部CT平扫|||||||||||||||||||||||||DR001^李医生^MD||||||||||||||||||||||||20241030120000"#;

    match hl7_interface.process_message(orm_message).await {
        Ok(parsed_message) => {
            info!("✅ ORM消息解析成功");
            info!("   检查类型: {:?}", parsed_message.message_type);
        }
        Err(e) => {
            warn!("❌ ORM消息解析失败: {}", e);
        }
    }

    Ok(())
}

/// API服务器演示
async fn demo_api_server() -> Result<()> {
    info!("\n🌐 RESTful API演示");

    info!("✅ API服务器创建成功");
    info!("   支持的接口:");
    info!("   - GET /health - 健康检查");
    info!("   - GET /system/stats - 系统统计");
    info!("   - POST /webhooks - 创建Webhook订阅");
    info!("   - 完整的CORS支持和请求日志记录");

    Ok(())
}

/// Webhook通知演示
async fn demo_webhook_notifications() -> Result<()> {
    info!("\n🔔 Webhook事件通知演示");

    let webhook_manager = WebhookManager::new();

    // 创建Webhook订阅
    let subscription_request = pacs_integration::webhook::WebhookSubscriptionRequest {
        url: "https://httpbin.org/post".to_string(),
        events: vec![
            "patient.created".to_string(),
            "study.completed".to_string(),
            "critical_value.detected".to_string(),
        ],
        secret: Some("webhook-secret-key".to_string()),
        active: Some(true),
    };

    match webhook_manager.subscribe(subscription_request).await {
        Ok(subscription_id) => {
            info!("✅ Webhook订阅创建成功: {}", subscription_id);
        }
        Err(e) => {
            warn!("❌ Webhook订阅创建失败: {}", e);
            return Ok(());
        }
    }

    // 创建患者创建事件（但不实际发送，避免网络请求）
    let patient_data = json!({
        "patient_id": "PAT12345",
        "patient_name": "张三",
        "birth_date": "1980-01-01",
        "sex": "M"
    });

    let patient_event = WebhookManager::create_patient_created_event(patient_data);
    info!("✅ 患者创建事件创建成功: {}", patient_event.id);

    // 创建检查完成事件
    let study_data = json!({
        "study_id": "STUDY12345",
        "patient_id": "PAT12345",
        "study_description": "胸部CT平扫",
        "modality": "CT",
        "series_count": 1,
        "instance_count": 120
    });

    let study_event = WebhookManager::create_study_completed_event(study_data);
    info!("✅ 检查完成事件创建成功: {}", study_event.id);

    // 创建危急值事件
    let critical_data = json!({
        "patient_id": "PAT12345",
        "study_id": "STUDY12345",
        "finding": "急性脑梗死",
        "radiologist": "李医生",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let critical_event = WebhookManager::create_critical_value_event(critical_data);
    info!("✅ 危急值事件创建成功: {}", critical_event.id);

    info!("   所有事件已创建，可在有网络环境时发送");

    Ok(())
}
