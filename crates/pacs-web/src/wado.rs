//! WADO服务 - DICOMweb实现

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use pacs_core::{error::PacsError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// QIDO-RS - DICOM查询服务
///
/// 实现DICOMweb的查询操作，支持搜索患者、检查、序列和实例
pub async fn qido_rs(Query(params): Query<QidoParams>) -> Result<impl IntoResponse> {
    info!("QIDO-RS query: {:?}", params);

    match params.level.as_deref() {
        Some("patient") | Some("PATIENT") => query_patients(&params).await,
        Some("study") | Some("STUDY") => query_studies(&params).await,
        Some("series") | Some("SERIES") => query_series(&params).await,
        Some("instance") | Some("INSTANCE") => query_instances(&params).await,
        _ => {
            // 默认查询检查级别
            query_studies(&params).await
        }
    }
}

/// WADO-RS - DICOM检索服务
///
/// 实现DICOMweb的检索操作，支持检索DICOM对象和元数据
pub async fn wado_rs(
    Path(path_params): Path<WadoPathParams>,
    Query(params): Query<WadoParams>,
) -> Result<impl IntoResponse> {
    info!("WADO-RS retrieve: {:?}, params: {:?}", path_params, params);

    // 根据请求类型返回不同内容
    match params.request_type.as_deref() {
        Some("metadata") => retrieve_metadata(&path_params).await,
        Some("bulkdata") => retrieve_bulkdata(&path_params, &params).await,
        None | Some("") => retrieve_dicom_object(&path_params).await,
        _ => Err(PacsError::Validation("Invalid request type".to_string())),
    }
}

/// STOW-RS - DICOM存储服务
///
/// 实现DICOMweb的存储操作，支持存储DICOM文件
pub async fn stow_rs(headers: HeaderMap, body: Bytes) -> Result<impl IntoResponse> {
    info!(
        "STOW-RS store request, content-type: {:?}",
        headers.get(header::CONTENT_TYPE)
    );

    // 检查内容类型
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| PacsError::Validation("Missing Content-Type header".to_string()))?;

    if !content_type.starts_with("application/dicom")
        && !content_type.starts_with("multipart/related")
    {
        return Err(PacsError::Validation(
            "Invalid Content-Type for STOW-RS".to_string(),
        ));
    }

    // TODO: 解析和存储DICOM文件
    let stored_instances = store_dicom_data(&body, content_type).await?;

    Ok(Json(json!({
        "status": "success",
        "stored_instances": stored_instances,
        "count": stored_instances.len()
    })))
}

/// QIDO-RS查询参数
#[derive(Debug, Deserialize)]
pub struct QidoParams {
    pub qido_level: Option<String>,
    pub level: Option<String>, // patient, study, series, instance
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub accession_number: Option<String>,
    pub study_instance_uid: Option<String>,
    pub series_instance_uid: Option<String>,
    pub sop_instance_uid: Option<String>,
    pub study_date: Option<String>,
    pub modality: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub fuzzymatching: Option<bool>,
    pub includefield: Option<Vec<String>>,
}

/// WADO-RS路径参数
#[derive(Debug, Deserialize)]
pub struct WadoPathParams {
    pub study_uid: String,
    pub series_uid: Option<String>,
    pub instance_uid: Option<String>,
}

/// WADO-RS查询参数
#[derive(Debug, Deserialize)]
pub struct WadoParams {
    pub request_type: Option<String>, // metadata, bulkdata
    pub media_type: Option<String>,   // application/dicom, application/octet-stream
    pub transfer_syntax: Option<String>,
    pub quality: Option<u8>, // JPEG质量
}

/// 存储结果
#[derive(Debug, Serialize)]
pub struct StoredInstance {
    pub study_instance_uid: String,
    pub series_instance_uid: String,
    pub sop_instance_uid: String,
    pub sop_class_uid: String,
    pub transfer_syntax_uid: String,
    pub success: bool,
    pub error_message: Option<String>,
}

// ========== 查询实现 ==========

async fn query_patients(params: &QidoParams) -> Result<Value> {
    // TODO: 从数据库查询患者数据
    let patients = vec![json!({
        "00100010": {"vr": "PN", "Value": [{"Alphabetic": "Doe^John"}]},
        "00100020": {"vr": "LO", "Value": ["PAT001"]},
        "00100030": {"vr": "DA", "Value": ["19800101"]},
        "00100040": {"vr": "CS", "Value": ["M"]},
    })];

    Ok(json!(patients))
}

async fn query_studies(params: &QidoParams) -> Result<Value> {
    // TODO: 从数据库查询检查数据
    let studies = vec![json!({
        "0020000D": {"vr": "UI", "Value": ["1.2.3.4.5.6.7.8.9.1"]},
        "00080020": {"vr": "DA", "Value": ["20231015"]},
        "00080030": {"vr": "TM", "Value": ["143000"]},
        "00080050": {"vr": "SH", "Value": ["ACC001"]},
        "00100010": {"vr": "PN", "Value": [{"Alphabetic": "Doe^John"}]},
        "00100020": {"vr": "LO", "Value": ["PAT001"]},
        "00081030": {"vr": "LO", "Value": ["CT Chest"]},
        "00201206": {"vr": "IS", "Value": ["2"]},
        "00201208": {"vr": "IS", "Value": ["250"]},
    })];

    Ok(json!(studies))
}

async fn query_series(params: &QidoParams) -> Result<Value> {
    // TODO: 从数据库查询序列数据
    let series = vec![json!({
        "0020000E": {"vr": "UI", "Value": ["1.2.3.4.5.6.7.8.9.1.1"]},
        "00200011": {"vr": "IS", "Value": ["1"]},
        "0008103E": {"vr": "LO", "Value": ["Axial CT"]},
        "00080060": {"vr": "CS", "Value": ["CT"]},
        "00180015": {"vr": "CS", "Value": ["CHEST"]},
        "00201209": {"vr": "IS", "Value": ["125"]},
    })];

    Ok(json!(series))
}

async fn query_instances(params: &QidoParams) -> Result<Value> {
    // TODO: 从数据库查询实例数据
    let instances = vec![json!({
        "00080018": {"vr": "UI", "Value": ["1.2.3.4.5.6.7.8.9.1.1.1"]},
        "00080016": {"vr": "UI", "Value": ["1.2.840.10008.5.1.4.1.1.2"]},
        "00200013": {"vr": "IS", "Value": ["1"]},
        "00280010": {"vr": "US", "Value": [512]},
        "00280011": {"vr": "US", "Value": [512]},
        "00280100": {"vr": "US", "Value": [16]},
        "00280101": {"vr": "US", "Value": [12]},
        "00280102": {"vr": "US", "Value": [11]},
        "00280103": {"vr": "US", "Value": [0]},
        "00280004": {"vr": "CS", "Value": ["MONOCHROME2"]},
        "00280002": {"vr": "US", "Value": [1]},
        "00280006": {"vr": "US", "Value": [0]},
    })];

    Ok(json!(instances))
}

// ========== WADO-RS实现 ==========

async fn retrieve_metadata(path_params: &WadoPathParams) -> Result<Response> {
    // TODO: 从存储检索DICOM元数据
    let metadata = json!({
        "0020000D": {"vr": "UI", "Value": [path_params.study_uid]},
        "00100010": {"vr": "PN", "Value": [{"Alphabetic": "Doe^John"}]},
        "00080060": {"vr": "CS", "Value": ["CT"]},
        // ... 更多元数据标签
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/dicom+json")
        .body(Body::from(metadata.to_string()))
        .unwrap();

    Ok(response)
}

async fn retrieve_bulkdata(path_params: &WadoPathParams, params: &WadoParams) -> Result<Response> {
    // TODO: 检索像素数据
    let bulk_data = Bytes::from_static(&[0u8; 1024]); // 模拟数据

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, bulk_data.len())
        .body(Body::from(bulk_data))
        .unwrap();

    Ok(response)
}

async fn retrieve_dicom_object(path_params: &WadoPathParams) -> Result<Response> {
    // TODO: 检索完整DICOM文件
    let dicom_data = Bytes::from_static(&[0u8; 2048]); // 模拟DICOM文件

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/dicom")
        .header(header::CONTENT_LENGTH, dicom_data.len())
        .body(Body::from(dicom_data))
        .unwrap();

    Ok(response)
}

// ========== STOW-RS实现 ==========

async fn store_dicom_data(data: &Bytes, content_type: &str) -> Result<Vec<StoredInstance>> {
    info!(
        "Storing DICOM data, content_type: {}, size: {} bytes",
        content_type,
        data.len()
    );

    // TODO: 解析DICOM文件并存储
    // 这里简单返回模拟的存储结果
    let instance = StoredInstance {
        study_instance_uid: "1.2.3.4.5.6.7.8.9.1".to_string(),
        series_instance_uid: "1.2.3.4.5.6.7.8.9.1.1".to_string(),
        sop_instance_uid: format!(
            "{}.{}",
            "1.2.3.4.5.6.7.8.9.1.1.1",
            Uuid::new_v4().to_string().replace("-", "")[..32].to_string()
        ),
        sop_class_uid: "1.2.840.10008.5.1.4.1.1.2".to_string(),
        transfer_syntax_uid: "1.2.840.10008.1.2.1".to_string(),
        success: true,
        error_message: None,
    };

    Ok(vec![instance])
}
