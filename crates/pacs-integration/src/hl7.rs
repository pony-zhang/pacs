//! HL7接口模块
//!
//! 实现与HIS/RIS系统的HL7 v2.x标准通信接口，支持：
//! - ADT消息（患者入院、出院、转院）
//! - ORM消息（检查申请）
//! - ORU消息（观察结果）
//! - SIU消息（预约信息）

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum Hl7Error {
    #[error("Invalid HL7 message format: {0}")]
    InvalidFormat(String),
    #[error("Unsupported message type: {0}")]
    UnsupportedMessageType(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// HL7消息类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hl7MessageType {
    ADT, // 患者管理
    ORM, // 检查申请
    ORU, // 观察结果
    SIU, // 预约信息
}

impl TryFrom<&str> for Hl7MessageType {
    type Error = Hl7Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ADT" => Ok(Hl7MessageType::ADT),
            "ORM" => Ok(Hl7MessageType::ORM),
            "ORU" => Ok(Hl7MessageType::ORU),
            "SIU" => Ok(Hl7MessageType::SIU),
            _ => Err(Hl7Error::UnsupportedMessageType(value.to_string())),
        }
    }
}

/// HL7消息解析后的结构化数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hl7Message {
    pub message_type: Hl7MessageType,
    pub trigger_event: String,
    pub message_control_id: String,
    pub processing_id: String,
    pub version_id: String,
    pub timestamp: DateTime<Utc>,
    pub segments: Vec<Hl7Segment>,
}

/// HL7段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hl7Segment {
    pub segment_type: String,
    pub fields: Vec<Vec<String>>,
}

/// 患者信息（从ADT消息提取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientInfo {
    pub patient_id: String,
    pub patient_name: String,
    pub birth_date: Option<chrono::NaiveDate>,
    pub sex: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

/// 检查申请信息（从ORM消息提取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub placer_order_number: String,
    pub filler_order_number: Option<String>,
    pub procedure_code: String,
    pub procedure_description: String,
    pub ordering_physician: Option<String>,
    pub priority: String,
    pub scheduled_time: Option<DateTime<Utc>>,
}

/// HL7解析器
pub struct Hl7Parser {
    field_separator: char,
    component_separator: char,
    repetition_separator: char,
    escape_character: char,
    subcomponent_separator: char,
}

impl Default for Hl7Parser {
    fn default() -> Self {
        Self {
            field_separator: '|',
            component_separator: '^',
            repetition_separator: '~',
            escape_character: '\\',
            subcomponent_separator: '&',
        }
    }
}

impl Hl7Parser {
    /// 创建新的HL7解析器
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析HL7消息
    pub fn parse(&self, message: &str) -> Result<Hl7Message> {
        info!("Parsing HL7 message");

        let lines: Vec<&str> = message.trim().split('\n').collect();
        if lines.is_empty() {
            return Err(Hl7Error::InvalidFormat("Empty message".to_string()));
        }

        // 解析MSH段
        let msh_segment = self.parse_segment(lines[0])?;
        if msh_segment.segment_type != "MSH" {
            return Err(Hl7Error::InvalidFormat("Message must start with MSH segment".to_string()));
        }

        let message_type = self.extract_message_type(&msh_segment)?;
        let timestamp = self.extract_timestamp(&msh_segment)?;

        let mut segments = Vec::new();
        for line in lines.iter().skip(1) {
            if !line.trim().is_empty() {
                segments.push(self.parse_segment(line)?);
            }
        }

        Ok(Hl7Message {
            message_type,
            trigger_event: msh_segment.fields.get(9).and_then(|f| f.first()).cloned().unwrap_or_default(),
            message_control_id: msh_segment.fields.get(10).and_then(|f| f.first()).cloned().unwrap_or_default(),
            processing_id: msh_segment.fields.get(11).and_then(|f| f.first()).cloned().unwrap_or_default(),
            version_id: msh_segment.fields.get(12).and_then(|f| f.first()).cloned().unwrap_or_default(),
            timestamp,
            segments,
        })
    }

    /// 解析单个段
    fn parse_segment(&self, line: &str) -> Result<Hl7Segment> {
        let parts: Vec<&str> = line.split(self.field_separator).collect();
        if parts.is_empty() {
            return Err(Hl7Error::InvalidFormat("Empty segment".to_string()));
        }

        let segment_type = parts[0].to_string();
        let mut fields = Vec::new();

        for part in parts.iter().skip(1) {
            let field_parts: Vec<String> = part
                .split(self.repetition_separator)
                .map(|r| {
                    r.split(self.component_separator)
                        .map(|c| c.to_string())
                        .collect()
                })
                .collect();
            fields.push(field_parts);
        }

        Ok(Hl7Segment {
            segment_type,
            fields,
        })
    }

    /// 提取消息类型
    fn extract_message_type(&self, msh_segment: &Hl7Segment) -> Result<Hl7MessageType> {
        let msg_type = msh_segment
            .fields
            .get(8)
            .and_then(|f| f.first())
            .ok_or_else(|| Hl7Error::MissingField("Message Type (MSH-9)".to_string()))?;

        let type_parts: Vec<&str> = msg_type.split(self.component_separator).collect();
        if type_parts.is_empty() {
            return Err(Hl7Error::InvalidFormat("Invalid message type format".to_string()));
        }

        Hl7MessageType::try_from(type_parts[0])
    }

    /// 提取时间戳
    fn extract_timestamp(&self, msh_segment: &Hl7Segment) -> Result<DateTime<Utc>> {
        let timestamp_str = msh_segment
            .fields
            .get(6)
            .and_then(|f| f.first())
            .ok_or_else(|| Hl7Error::MissingField("Timestamp (MSH-7)".to_string()))?;

        // HL7时间格式: YYYYMMDDHHMMSS[.SSSS][+/-ZZZZ]
        let cleaned = timestamp_str.split(&['+', '-'][..]).next().unwrap_or(timestamp_str);
        let datetime = if cleaned.len() >= 14 {
            let year: i32 = cleaned[0..4].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid year".to_string()))?;
            let month: u32 = cleaned[4..6].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid month".to_string()))?;
            let day: u32 = cleaned[6..8].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid day".to_string()))?;
            let hour: u32 = cleaned[8..10].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid hour".to_string()))?;
            let minute: u32 = cleaned[10..12].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid minute".to_string()))?;
            let second: u32 = if cleaned.len() >= 14 {
                cleaned[12..14].parse()
                    .map_err(|_| Hl7Error::ParseError("Invalid second".to_string()))?
            } else {
                0
            };

            chrono::Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
                .single()
                .ok_or_else(|| Hl7Error::ParseError("Invalid datetime".to_string()))?
        } else {
            chrono::Utc::now()
        };

        Ok(datetime)
    }

    /// 从消息中提取患者信息
    pub fn extract_patient_info(&self, message: &Hl7Message) -> Result<Option<PatientInfo>> {
        if message.message_type != Hl7MessageType::ADT {
            return Ok(None);
        }

        let pid_segment = message.segments.iter()
            .find(|s| s.segment_type == "PID")
            .ok_or_else(|| Hl7Error::MissingField("PID segment".to_string()))?;

        let patient_id = pid_segment.fields.get(3)
            .and_then(|f| f.first())
            .ok_or_else(|| Hl7Error::MissingField("Patient ID (PID-3)".to_string()))?
            .clone();

        let patient_name = pid_segment.fields.get(5)
            .and_then(|f| f.first())
            .map(|name| name.replace(&self.component_separator.to_string(), " "))
            .unwrap_or_default();

        let birth_date = pid_segment.fields.get(7)
            .and_then(|f| f.first())
            .and_then(|date_str| {
                if date_str.len() >= 8 {
                    chrono::NaiveDate::from_ymd_opt(
                        date_str[0..4].parse().ok()?,
                        date_str[4..6].parse().ok()?,
                        date_str[6..8].parse().ok()?,
                    )
                } else {
                    None
                }
            });

        let sex = pid_segment.fields.get(8)
            .and_then(|f| f.first())
            .cloned();

        let address = pid_segment.fields.get(11)
            .and_then(|f| f.first())
            .map(|addr| addr.replace(&self.component_separator.to_string(), " "));

        let phone = pid_segment.fields.get(13)
            .and_then(|f| f.first())
            .cloned();

        Ok(Some(PatientInfo {
            patient_id,
            patient_name,
            birth_date,
            sex,
            address,
            phone,
        }))
    }

    /// 从消息中提取检查申请信息
    pub fn extract_order_info(&self, message: &Hl7Message) -> Result<Option<OrderInfo>> {
        if message.message_type != Hl7MessageType::ORM {
            return Ok(None);
        }

        let orc_segment = message.segments.iter()
            .find(|s| s.segment_type == "ORC")
            .ok_or_else(|| Hl7Error::MissingField("ORC segment".to_string()))?;

        let placer_order_number = orc_segment.fields.get(2)
            .and_then(|f| f.first())
            .ok_or_else(|| Hl7Error::MissingField("Placer Order Number (ORC-2)".to_string()))?
            .clone();

        let filler_order_number = orc_segment.fields.get(3)
            .and_then(|f| f.first())
            .cloned();

        // 查找OBR段获取更多信息
        let obr_segment = message.segments.iter()
            .find(|s| s.segment_type == "OBR");

        let (procedure_code, procedure_description) = if let Some(obr) = obr_segment {
            let code = obr.fields.get(4)
                .and_then(|f| f.first())
                .cloned()
                .unwrap_or_default();
            let description = obr.fields.get(4)
                .and_then(|f| f.get(1))
                .cloned()
                .unwrap_or_default();
            (code, description)
        } else {
            (String::new(), String::new())
        };

        let ordering_physician = obr_segment
            .and_then(|obr| obr.fields.get(16))
            .and_then(|f| f.first())
            .cloned();

        let priority = obr_segment
            .and_then(|obr| obr.fields.get(5))
            .and_then(|f| f.first())
            .cloned()
            .unwrap_or_else(|| "R".to_string()); // 默认Routine

        let scheduled_time = obr_segment
            .and_then(|obr| obr.fields.get(8))
            .and_then(|f| f.first())
            .and_then(|time_str| self.parse_hl7_datetime(time_str).ok());

        Ok(Some(OrderInfo {
            placer_order_number,
            filler_order_number,
            procedure_code,
            procedure_description,
            ordering_physician,
            priority,
            scheduled_time,
        }))
    }

    /// 解析HL7日期时间
    fn parse_hl7_datetime(&self, datetime_str: &str) -> Result<DateTime<Utc>> {
        if datetime_str.len() >= 14 {
            let year: i32 = datetime_str[0..4].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid year".to_string()))?;
            let month: u32 = datetime_str[4..6].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid month".to_string()))?;
            let day: u32 = datetime_str[6..8].parse()
                .map_err(|_| Hl7Error::ParseError("Invalid day".to_string()))?;
            let hour: u32 = if datetime_str.len() >= 10 {
                datetime_str[8..10].parse()
                    .map_err(|_| Hl7Error::ParseError("Invalid hour".to_string()))?
            } else {
                0
            };
            let minute: u32 = if datetime_str.len() >= 12 {
                datetime_str[10..12].parse()
                    .map_err(|_| Hl7Error::ParseError("Invalid minute".to_string()))?
            } else {
                0
            };
            let second: u32 = if datetime_str.len() >= 14 {
                datetime_str[12..14].parse()
                    .map_err(|_| Hl7Error::ParseError("Invalid second".to_string()))?
            } else {
                0
            };

            chrono::Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
                .single()
                .ok_or_else(|| Hl7Error::ParseError("Invalid datetime".to_string()))
        } else {
            Err(Hl7Error::ParseError("Datetime too short".to_string()))
        }
    }
}

/// HL7接口处理器
pub struct Hl7Interface {
    parser: Hl7Parser,
}

impl Hl7Interface {
    /// 创建新的HL7接口
    pub fn new() -> Self {
        Self {
            parser: Hl7Parser::new(),
        }
    }

    /// 处理接收到的HL7消息
    pub async fn process_message(&self, message: &str) -> Result<Hl7Message> {
        debug!("Processing HL7 message: {}", message.chars().take(100).collect::<String>());

        let parsed_message = self.parser.parse(message)?;

        match parsed_message.message_type {
            Hl7MessageType::ADT => {
                if let Ok(Some(patient_info)) = self.parser.extract_patient_info(&parsed_message) {
                    self.handle_patient_update(&patient_info).await?;
                }
            },
            Hl7MessageType::ORM => {
                if let Ok(Some(order_info)) = self.parser.extract_order_info(&parsed_message) {
                    self.handle_order_request(&order_info).await?;
                }
            },
            _ => {
                warn!("Unhandled HL7 message type: {:?}", parsed_message.message_type);
            }
        }

        info!("Successfully processed HL7 message type: {:?}", parsed_message.message_type);
        Ok(parsed_message)
    }

    /// 处理患者信息更新
    async fn handle_patient_update(&self, patient_info: &PatientInfo) -> Result<()> {
        info!("Updating patient information for ID: {}", patient_info.patient_id);
        // TODO: 集成到数据库模块
        // 这里应该调用数据库模块更新患者信息
        Ok(())
    }

    /// 处理检查申请
    async fn handle_order_request(&self, order_info: &OrderInfo) -> Result<()> {
        info!("Processing order request: {}", order_info.placer_order_number);
        // TODO: 集成到工作流引擎和数据库模块
        // 这里应该创建新的检查记录并触发工作流
        Ok(())
    }

    /// 生成HL7 ACK消息
    pub fn generate_ack(&self, original_message: &Hl7Message, success: bool, error_message: Option<&str>) -> String {
        let now = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let ack_code = if success { "AA" } else { "AE" };
        let error_text = error_message.unwrap_or("").replace('|', "\\E\\");

        format!(
            "MSH|^~\\&|PACS|HOSPITAL|HIS|HOSPITAL|{timestamp}||ACK|{control_id}|P|2.5\r\nMSA|{ack_code}|{original_control_id}|{error_text}",
            timestamp = now,
            control_id = uuid::Uuid::new_v4().to_string().chars().take(20).collect::<String>(),
            original_control_id = original_message.message_control_id,
            error_text = error_text
        )
    }
}

impl Default for Hl7Interface {
    fn default() -> Self {
        Self::new()
    }
}
