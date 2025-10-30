//! 核心数据模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 患者基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: Uuid,
    pub patient_id: String,                    // 医院内部患者ID
    pub name: String,                          // 患者姓名
    pub sex: Option<Sex>,                      // 性别
    pub birth_date: Option<chrono::NaiveDate>, // 出生日期
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 性别枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
    Other,
}

/// 检查信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Study {
    pub id: Uuid,
    pub study_uid: String, // DICOM Study Instance UID
    pub patient_id: Uuid,
    pub accession_number: String, // 检查号
    pub study_date: chrono::NaiveDate,
    pub study_time: Option<chrono::NaiveTime>,
    pub modality: String,            // 检查设备类型 (CT, MR, DR等)
    pub description: Option<String>, // 检查描述
    pub status: StudyStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 检查状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StudyStatus {
    Scheduled,   // 已预约
    InProgress,  // 检查中
    Completed,   // 已完成
    Preliminary, // 初步报告
    Final,       // 最终报告
    Canceled,    // 已取消
}

/// 系列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: Uuid,
    pub series_uid: String, // DICOM Series Instance UID
    pub study_id: Uuid,
    pub modality: String,
    pub series_number: i32,
    pub description: Option<String>,
    pub images_count: i32,
    pub created_at: DateTime<Utc>,
}

/// 影像实例信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
    pub sop_instance_uid: String, // DICOM SOP Instance UID
    pub series_id: Uuid,
    pub instance_number: i32,
    pub file_path: String,
    pub file_size: i64,
    pub transfer_syntax_uid: String,
    pub created_at: DateTime<Utc>,
}
