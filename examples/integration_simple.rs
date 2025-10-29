//! PACS集成模块简化演示程序
//!
//! 展示集成模块的核心功能：
//! - HL7消息解析
//! - 基础Webhook功能

use anyhow::Result;
use pacs_integration::hl7::Hl7Interface;
use serde_json::json;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 启动PACS集成模块简化演示");

    // HL7接口演示
    demo_hl7_interface().await?;

    info!("✅ 集成模块简化演示完成");
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
        },
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
        },
        Err(e) => {
            warn!("❌ ORM消息解析失败: {}", e);
        }
    }

    Ok(())
}