//! 数据库模型

use pacs_core::models::*;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate, NaiveTime};

// 数据库表模型 - 使用FromRow trait用于SQL查询

/// 数据库患者表
#[derive(Debug, FromRow)]
pub struct DbPatient {
    pub id: Uuid,
    pub patient_id: String,
    pub name: String,
    pub sex: Option<String>, // 存储为字符串，转换为Sex枚举
    pub birth_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DbPatient> for Patient {
    fn from(db_patient: DbPatient) -> Self {
        Patient {
            id: db_patient.id,
            patient_id: db_patient.patient_id,
            name: db_patient.name,
            sex: db_patient.sex.and_then(|s| match s.as_str() {
                "M" => Some(Sex::Male),
                "F" => Some(Sex::Female),
                "O" => Some(Sex::Other),
                _ => None,
            }),
            birth_date: db_patient.birth_date,
            created_at: db_patient.created_at,
            updated_at: db_patient.updated_at,
        }
    }
}

/// 数据库检查表
#[derive(Debug, FromRow)]
pub struct DbStudy {
    pub id: Uuid,
    pub study_uid: String,
    pub patient_id: Uuid,
    pub accession_number: String,
    pub study_date: NaiveDate,
    pub study_time: Option<NaiveTime>,
    pub modality: String,
    pub description: Option<String>,
    pub status: String, // 存储为字符串，转换为StudyStatus枚举
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DbStudy> for Study {
    fn from(db_study: DbStudy) -> Self {
        Study {
            id: db_study.id,
            study_uid: db_study.study_uid,
            patient_id: db_study.patient_id,
            accession_number: db_study.accession_number,
            study_date: db_study.study_date,
            study_time: db_study.study_time,
            modality: db_study.modality,
            description: db_study.description,
            status: match db_study.status.as_str() {
                "SCHEDULED" => StudyStatus::Scheduled,
                "IN_PROGRESS" => StudyStatus::InProgress,
                "COMPLETED" => StudyStatus::Completed,
                "PRELIMINARY" => StudyStatus::Preliminary,
                "FINAL" => StudyStatus::Final,
                "CANCELED" => StudyStatus::Canceled,
                _ => StudyStatus::Scheduled, // 默认状态
            },
            created_at: db_study.created_at,
            updated_at: db_study.updated_at,
        }
    }
}

/// 数据库系列表
#[derive(Debug, FromRow)]
pub struct DbSeries {
    pub id: Uuid,
    pub series_uid: String,
    pub study_id: Uuid,
    pub modality: String,
    pub series_number: i32,
    pub description: Option<String>,
    pub images_count: i32,
    pub created_at: DateTime<Utc>,
}

impl From<DbSeries> for Series {
    fn from(db_series: DbSeries) -> Self {
        Series {
            id: db_series.id,
            series_uid: db_series.series_uid,
            study_id: db_series.study_id,
            modality: db_series.modality,
            series_number: db_series.series_number,
            description: db_series.description,
            images_count: db_series.images_count,
            created_at: db_series.created_at,
        }
    }
}

/// 数据库实例表
#[derive(Debug, FromRow)]
pub struct DbInstance {
    pub id: Uuid,
    pub sop_instance_uid: String,
    pub series_id: Uuid,
    pub instance_number: i32,
    pub file_path: String,
    pub file_size: i64,
    pub transfer_syntax_uid: String,
    pub created_at: DateTime<Utc>,
}

impl From<DbInstance> for Instance {
    fn from(db_instance: DbInstance) -> Self {
        Instance {
            id: db_instance.id,
            sop_instance_uid: db_instance.sop_instance_uid,
            series_id: db_instance.series_id,
            instance_number: db_instance.instance_number,
            file_path: db_instance.file_path,
            file_size: db_instance.file_size,
            transfer_syntax_uid: db_instance.transfer_syntax_uid,
            created_at: db_instance.created_at,
        }
    }
}

// 插入模型 - 用于创建新记录

/// 新患者插入模型
#[derive(Debug)]
pub struct NewPatient {
    pub id: Uuid,
    pub patient_id: String,
    pub name: String,
    pub sex: Option<Sex>,
    pub birth_date: Option<NaiveDate>,
}

impl NewPatient {
    pub fn from_patient(patient: &Patient) -> Self {
        Self {
            id: patient.id,
            patient_id: patient.patient_id.clone(),
            name: patient.name.clone(),
            sex: patient.sex.clone(),
            birth_date: patient.birth_date,
        }
    }
}

/// 新检查插入模型
#[derive(Debug)]
pub struct NewStudy {
    pub id: Uuid,
    pub study_uid: String,
    pub patient_id: Uuid,
    pub accession_number: String,
    pub study_date: NaiveDate,
    pub study_time: Option<NaiveTime>,
    pub modality: String,
    pub description: Option<String>,
    pub status: StudyStatus,
}

impl NewStudy {
    pub fn from_study(study: &Study) -> Self {
        Self {
            id: study.id,
            study_uid: study.study_uid.clone(),
            patient_id: study.patient_id,
            accession_number: study.accession_number.clone(),
            study_date: study.study_date,
            study_time: study.study_time,
            modality: study.modality.clone(),
            description: study.description.clone(),
            status: study.status.clone(),
        }
    }
}

/// 新系列插入模型
#[derive(Debug)]
pub struct NewSeries {
    pub id: Uuid,
    pub series_uid: String,
    pub study_id: Uuid,
    pub modality: String,
    pub series_number: i32,
    pub description: Option<String>,
    pub images_count: i32,
}

impl NewSeries {
    pub fn from_series(series: &Series) -> Self {
        Self {
            id: series.id,
            series_uid: series.series_uid.clone(),
            study_id: series.study_id,
            modality: series.modality.clone(),
            series_number: series.series_number,
            description: series.description.clone(),
            images_count: series.images_count,
        }
    }
}

/// 新实例插入模型
#[derive(Debug)]
pub struct NewInstance {
    pub id: Uuid,
    pub sop_instance_uid: String,
    pub series_id: Uuid,
    pub instance_number: i32,
    pub file_path: String,
    pub file_size: i64,
    pub transfer_syntax_uid: String,
}

impl NewInstance {
    pub fn from_instance(instance: &Instance) -> Self {
        Self {
            id: instance.id,
            sop_instance_uid: instance.sop_instance_uid.clone(),
            series_id: instance.series_id,
            instance_number: instance.instance_number,
            file_path: instance.file_path.clone(),
            file_size: instance.file_size,
            transfer_syntax_uid: instance.transfer_syntax_uid.clone(),
        }
    }
}