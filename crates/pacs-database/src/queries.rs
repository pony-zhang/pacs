//! 数据库查询操作

use crate::models::*;
use crate::connection::DatabasePool;
use pacs_core::{PacsError, Result, Patient, Study, Series, Instance, Sex, StudyStatus};
use sqlx::Row;
use uuid::Uuid;

/// 数据库查询操作接口
pub struct DatabaseQueries<'a> {
    pool: &'a DatabasePool,
}

impl<'a> DatabaseQueries<'a> {
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// 创建数据库表
    pub async fn create_tables(&self) -> Result<()> {
        let pool = self.pool.pool();

        // 创建患者表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS patients (
                id UUID PRIMARY KEY,
                patient_id VARCHAR(64) UNIQUE NOT NULL,
                name VARCHAR(255) NOT NULL,
                sex CHAR(1),
                birth_date DATE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
        "#).execute(pool).await.map_err(|e| PacsError::Database(e.to_string()))?;

        // 创建检查表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS studies (
                id UUID PRIMARY KEY,
                study_uid VARCHAR(64) UNIQUE NOT NULL,
                patient_id UUID NOT NULL REFERENCES patients(id),
                accession_number VARCHAR(64) NOT NULL,
                study_date DATE NOT NULL,
                study_time TIME,
                modality VARCHAR(16) NOT NULL,
                description TEXT,
                status VARCHAR(20) NOT NULL DEFAULT 'SCHEDULED',
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
        "#).execute(pool).await.map_err(|e| PacsError::Database(e.to_string()))?;

        // 创建系列表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS series (
                id UUID PRIMARY KEY,
                series_uid VARCHAR(64) UNIQUE NOT NULL,
                study_id UUID NOT NULL REFERENCES studies(id),
                modality VARCHAR(16) NOT NULL,
                series_number INTEGER NOT NULL,
                description TEXT,
                images_count INTEGER DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
        "#).execute(pool).await.map_err(|e| PacsError::Database(e.to_string()))?;

        // 创建实例表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS instances (
                id UUID PRIMARY KEY,
                sop_instance_uid VARCHAR(64) UNIQUE NOT NULL,
                series_id UUID NOT NULL REFERENCES series(id),
                instance_number INTEGER NOT NULL,
                file_path VARCHAR(512) NOT NULL,
                file_size BIGINT NOT NULL,
                transfer_syntax_uid VARCHAR(64) NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
        "#).execute(pool).await.map_err(|e| PacsError::Database(e.to_string()))?;

        // 创建索引以优化查询性能
        self.create_indexes().await?;

        tracing::info!("Database tables created successfully");
        Ok(())
    }

    /// 创建数据库索引
    async fn create_indexes(&self) -> Result<()> {
        let pool = self.pool.pool();

        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_patients_patient_id ON patients(patient_id)",
            "CREATE INDEX IF NOT EXISTS idx_patients_name ON patients(name)",
            "CREATE INDEX IF NOT EXISTS idx_studies_study_uid ON studies(study_uid)",
            "CREATE INDEX IF NOT EXISTS idx_studies_patient_id ON studies(patient_id)",
            "CREATE INDEX IF NOT EXISTS idx_studies_accession_number ON studies(accession_number)",
            "CREATE INDEX IF NOT EXISTS idx_studies_study_date ON studies(study_date)",
            "CREATE INDEX IF NOT EXISTS idx_studies_modality ON studies(modality)",
            "CREATE INDEX IF NOT EXISTS idx_series_series_uid ON series(series_uid)",
            "CREATE INDEX IF NOT EXISTS idx_series_study_id ON series(study_id)",
            "CREATE INDEX IF NOT EXISTS idx_instances_sop_instance_uid ON instances(sop_instance_uid)",
            "CREATE INDEX IF NOT EXISTS idx_instances_series_id ON instances(series_id)",
        ];

        for index_sql in indexes {
            sqlx::query(index_sql)
                .execute(pool)
                .await
                .map_err(|e| PacsError::Database(e.to_string()))?;
        }

        tracing::info!("Database indexes created successfully");
        Ok(())
    }

    // ========== 患者相关操作 ==========

    /// 创建新患者
    pub async fn create_patient(&self, patient: &NewPatient) -> Result<Uuid> {
        let pool = self.pool.pool();

        let sex_str = patient.sex.as_ref().map(|s| match s {
            Sex::Male => "M",
            Sex::Female => "F",
            Sex::Other => "O",
        });

        sqlx::query(r#"
            INSERT INTO patients (id, patient_id, name, sex, birth_date)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
        "#)
        .bind(patient.id)
        .bind(&patient.patient_id)
        .bind(&patient.name)
        .bind(sex_str)
        .bind(patient.birth_date)
        .fetch_one(pool)
        .await
        .map(|row| row.get("id"))
        .map_err(|e| PacsError::Database(e.to_string()))
    }

    /// 根据ID查找患者
    pub async fn get_patient_by_id(&self, id: &Uuid) -> Result<Option<Patient>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbPatient>(
            "SELECT * FROM patients WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_patient| Patient::from(db_patient)))
    }

    /// 根据患者ID查找患者
    pub async fn get_patient_by_patient_id(&self, patient_id: &str) -> Result<Option<Patient>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbPatient>(
            "SELECT * FROM patients WHERE patient_id = $1"
        )
        .bind(patient_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_patient| Patient::from(db_patient)))
    }

    /// 根据姓名搜索患者
    pub async fn search_patients_by_name(&self, name: &str, limit: i64) -> Result<Vec<Patient>> {
        let pool = self.pool.pool();

        let results = sqlx::query_as::<_, DbPatient>(
            "SELECT * FROM patients WHERE name ILIKE $1 ORDER BY updated_at DESC LIMIT $2"
        )
        .bind(format!("%{}%", name))
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(results.into_iter().map(Patient::from).collect())
    }

    // ========== 检查相关操作 ==========

    /// 创建新检查
    pub async fn create_study(&self, study: &NewStudy) -> Result<Uuid> {
        let pool = self.pool.pool();

        let status_str = match study.status {
            StudyStatus::Scheduled => "SCHEDULED",
            StudyStatus::InProgress => "IN_PROGRESS",
            StudyStatus::Completed => "COMPLETED",
            StudyStatus::Preliminary => "PRELIMINARY",
            StudyStatus::Final => "FINAL",
            StudyStatus::Canceled => "CANCELED",
        };

        sqlx::query(r#"
            INSERT INTO studies (id, study_uid, patient_id, accession_number, study_date, study_time, modality, description, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
        "#)
        .bind(study.id)
        .bind(&study.study_uid)
        .bind(study.patient_id)
        .bind(&study.accession_number)
        .bind(study.study_date)
        .bind(study.study_time)
        .bind(&study.modality)
        .bind(&study.description)
        .bind(status_str)
        .fetch_one(pool)
        .await
        .map(|row| row.get("id"))
        .map_err(|e| PacsError::Database(e.to_string()))
    }

    /// 根据检查UID查找检查
    pub async fn get_study_by_uid(&self, study_uid: &str) -> Result<Option<Study>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbStudy>(
            "SELECT * FROM studies WHERE study_uid = $1"
        )
        .bind(study_uid)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_study| Study::from(db_study)))
    }

    /// 根据患者ID获取所有检查
    pub async fn get_studies_by_patient_id(&self, patient_id: &Uuid) -> Result<Vec<Study>> {
        let pool = self.pool.pool();

        let results = sqlx::query_as::<_, DbStudy>(
            "SELECT * FROM studies WHERE patient_id = $1 ORDER BY study_date DESC, study_time DESC"
        )
        .bind(patient_id)
        .fetch_all(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(results.into_iter().map(Study::from).collect())
    }

    /// 根据检查号查找检查
    pub async fn get_study_by_accession_number(&self, accession_number: &str) -> Result<Option<Study>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbStudy>(
            "SELECT * FROM studies WHERE accession_number = $1"
        )
        .bind(accession_number)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_study| Study::from(db_study)))
    }

    // ========== 系列相关操作 ==========

    /// 创建新系列
    pub async fn create_series(&self, series: &NewSeries) -> Result<Uuid> {
        let pool = self.pool.pool();

        sqlx::query(r#"
            INSERT INTO series (id, series_uid, study_id, modality, series_number, description, images_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
        "#)
        .bind(series.id)
        .bind(&series.series_uid)
        .bind(series.study_id)
        .bind(&series.modality)
        .bind(series.series_number)
        .bind(&series.description)
        .bind(series.images_count)
        .fetch_one(pool)
        .await
        .map(|row| row.get("id"))
        .map_err(|e| PacsError::Database(e.to_string()))
    }

    /// 根据系列UID查找系列
    pub async fn get_series_by_uid(&self, series_uid: &str) -> Result<Option<Series>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbSeries>(
            "SELECT * FROM series WHERE series_uid = $1"
        )
        .bind(series_uid)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_series| Series::from(db_series)))
    }

    /// 根据检查ID获取所有系列
    pub async fn get_series_by_study_id(&self, study_id: &Uuid) -> Result<Vec<Series>> {
        let pool = self.pool.pool();

        let results = sqlx::query_as::<_, DbSeries>(
            "SELECT * FROM series WHERE study_id = $1 ORDER BY series_number"
        )
        .bind(study_id)
        .fetch_all(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(results.into_iter().map(Series::from).collect())
    }

    // ========== 实例相关操作 ==========

    /// 创建新实例
    pub async fn create_instance(&self, instance: &NewInstance) -> Result<Uuid> {
        let pool = self.pool.pool();

        sqlx::query(r#"
            INSERT INTO instances (id, sop_instance_uid, series_id, instance_number, file_path, file_size, transfer_syntax_uid)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
        "#)
        .bind(instance.id)
        .bind(&instance.sop_instance_uid)
        .bind(instance.series_id)
        .bind(instance.instance_number)
        .bind(&instance.file_path)
        .bind(instance.file_size)
        .bind(&instance.transfer_syntax_uid)
        .fetch_one(pool)
        .await
        .map(|row| row.get("id"))
        .map_err(|e| PacsError::Database(e.to_string()))
    }

    /// 根据SOP实例UID查找实例
    pub async fn get_instance_by_uid(&self, sop_instance_uid: &str) -> Result<Option<Instance>> {
        let pool = self.pool.pool();

        let result = sqlx::query_as::<_, DbInstance>(
            "SELECT * FROM instances WHERE sop_instance_uid = $1"
        )
        .bind(sop_instance_uid)
        .fetch_optional(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(result.map(|db_instance| Instance::from(db_instance)))
    }

    /// 根据系列ID获取所有实例
    pub async fn get_instances_by_series_id(&self, series_id: &Uuid) -> Result<Vec<Instance>> {
        let pool = self.pool.pool();

        let results = sqlx::query_as::<_, DbInstance>(
            "SELECT * FROM instances WHERE series_id = $1 ORDER BY instance_number"
        )
        .bind(series_id)
        .fetch_all(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(results.into_iter().map(Instance::from).collect())
    }

    /// 更新系列的图像计数
    pub async fn update_series_images_count(&self, series_id: &Uuid, count: i32) -> Result<()> {
        let pool = self.pool.pool();

        sqlx::query(
            "UPDATE series SET images_count = $1 WHERE id = $2"
        )
        .bind(count)
        .bind(series_id)
        .execute(pool)
        .await
        .map_err(|e| PacsError::Database(e.to_string()))?;

        Ok(())
    }
}