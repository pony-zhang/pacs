//! DICOM数据解析器
//!
//! 提供完整的DICOM文件解析和元数据提取功能

use pacs_core::{PacsError, Result};
use dicom::core::value::{Value, PrimitiveValue};
use dicom::encoding::{TransferSyntax};
use dicom::object::{open_file, DefaultDicomObject, InMemDicomObject};
use dicom::dictionary_std::{tags};
use std::io::Cursor;
use tracing::{debug, info, warn, error};
use std::path::Path;

/// DICOM数据解析器
pub struct DicomParser;

impl Default for DicomParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DicomParser {
    /// 创建新的DICOM解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析DICOM文件
    pub async fn parse_file<P: AsRef<Path> + std::fmt::Debug>(file_path: P) -> Result<ParsedDicomObject> {
        let file_path = file_path.as_ref();
        info!("开始解析DICOM文件: {:?}", file_path);

        // 使用dicom-rs解析文件
        let obj = open_file(file_path)
            .map_err(|e| {
                error!("DICOM文件解析失败: {:?}", e);
                PacsError::DicomParseError(format!("无法解析DICOM文件: {:?}", e))
            })?;

        debug!("成功解析DICOM文件，开始提取元数据");
        Self::extract_metadata(obj)
    }

    /// 解析DICOM字节数据
    pub async fn parse_bytes(data: &[u8]) -> Result<ParsedDicomObject> {
        info!("开始解析DICOM字节数据，大小: {} bytes", data.len());

        // 简化实现：暂时不支持字节数据直接解析
        // 可以先写入临时文件再解析，或者使用其他方法
        Err(PacsError::DicomParseError("字节数据解析暂未实现，请使用parse_file方法".to_string()))
    }

    /// 验证DICOM文件完整性
    pub async fn validate_file<P: AsRef<Path> + std::fmt::Debug>(file_path: P) -> Result<bool> {
        let file_path = file_path.as_ref();
        debug!("验证DICOM文件完整性: {:?}", file_path);

        match open_file(file_path) {
            Ok(obj) => {
                // 检查必要的DICOM标签
                let required_tags = [
                    tags::SOP_CLASS_UID,
                    tags::SOP_INSTANCE_UID,
                    tags::STUDY_INSTANCE_UID,
                    tags::SERIES_INSTANCE_UID,
                ];

                for tag in &required_tags {
                    if obj.element(*tag).is_err() {
                        warn!("DICOM文件缺少必要标签: {:?}", tag);
                        return Ok(false);
                    }
                }

                info!("DICOM文件验证通过: {:?}", file_path);
                Ok(true)
            }
            Err(e) => {
                warn!("DICOM文件验证失败: {:?}, 错误: {:?}", file_path, e);
                Ok(false)
            }
        }
    }

    /// 从DICOM对象中提取元数据
    fn extract_metadata(obj: impl Into<DefaultDicomObject>) -> Result<ParsedDicomObject> {
        let obj = obj.into();
        let mut parsed = ParsedDicomObject::new();

        // 提取患者信息
        parsed.patient_id = Self::get_string_element(&obj, tags::PATIENT_ID);
        parsed.patient_name = Self::get_string_element(&obj, tags::PATIENT_NAME);
        parsed.patient_birth_date = Self::get_string_element(&obj, tags::PATIENT_BIRTH_DATE);
        parsed.patient_sex = Self::get_string_element(&obj, tags::PATIENT_SEX);

        // 提取检查信息
        parsed.study_instance_uid = Self::get_string_element(&obj, tags::STUDY_INSTANCE_UID);
        parsed.study_date = Self::get_string_element(&obj, tags::STUDY_DATE);
        parsed.study_time = Self::get_string_element(&obj, tags::STUDY_TIME);
        parsed.study_description = Self::get_string_element(&obj, tags::STUDY_DESCRIPTION);
        parsed.accession_number = Self::get_string_element(&obj, tags::ACCESSION_NUMBER);

        // 提取序列信息
        parsed.series_instance_uid = Self::get_string_element(&obj, tags::SERIES_INSTANCE_UID);
        parsed.series_number = Self::get_string_element(&obj, tags::SERIES_NUMBER);
        parsed.series_description = Self::get_string_element(&obj, tags::SERIES_DESCRIPTION);
        parsed.modality = Self::get_string_element(&obj, tags::MODALITY);

        // 提取实例信息
        parsed.sop_instance_uid = Self::get_string_element(&obj, tags::SOP_INSTANCE_UID);
        parsed.sop_class_uid = Self::get_string_element(&obj, tags::SOP_CLASS_UID);
        parsed.instance_number = Self::get_string_element(&obj, tags::INSTANCE_NUMBER);

        // 提取设备信息
        parsed.institution_name = Self::get_string_element(&obj, tags::INSTITUTION_NAME);
        parsed.manufacturer = Self::get_string_element(&obj, tags::MANUFACTURER);
        parsed.manufacturer_model_name = Self::get_string_element(&obj, tags::MANUFACTURER_MODEL_NAME);

        // 提取图像信息
        parsed.rows = Self::get_integer_element(&obj, tags::ROWS);
        parsed.columns = Self::get_integer_element(&obj, tags::COLUMNS);
        parsed.bits_allocated = Self::get_integer_element(&obj, tags::BITS_ALLOCATED);
        parsed.bits_stored = Self::get_integer_element(&obj, tags::BITS_STORED);
        parsed.high_bit = Self::get_integer_element(&obj, tags::HIGH_BIT);
        parsed.pixel_representation = Self::get_integer_element(&obj, tags::PIXEL_REPRESENTATION);

        // 提取传输语法信息
        parsed.transfer_syntax_uid = Self::get_string_element(&obj, tags::TRANSFER_SYNTAX_UID);

        // 提取其他重要信息
        parsed.patient_age = Self::get_string_element(&obj, tags::PATIENT_AGE);
        parsed.patient_weight = Self::get_string_element(&obj, tags::PATIENT_WEIGHT);
        parsed.body_part_examined = Self::get_string_element(&obj, tags::BODY_PART_EXAMINED);

        info!("成功提取DICOM元数据，患者ID: {:?}, 检查UID: {:?}",
              parsed.patient_id, parsed.study_instance_uid);

        Ok(parsed)
    }

    /// 获取字符串类型元素的值
    fn get_string_element(obj: &DefaultDicomObject, tag: dicom::core::Tag) -> Option<String> {
        match obj.element(tag) {
            Ok(element) => {
                match element.value() {
                    Value::Primitive(PrimitiveValue::Str(s)) => Some(s.to_string()),
                    Value::Primitive(PrimitiveValue::Strs(strings)) => strings.first().map(|s| s.to_string()),
                    _ => {
                        debug!("标签 {:?} 不是字符串类型", tag);
                        None
                    }
                }
            }
            Err(_) => {
                debug!("未找到标签: {:?}", tag);
                None
            }
        }
    }

    /// 获取整数类型元素的值
    fn get_integer_element(obj: &DefaultDicomObject, tag: dicom::core::Tag) -> Option<i32> {
        match obj.element(tag) {
            Ok(element) => {
                match element.value() {
                    Value::Primitive(PrimitiveValue::I32(i)) => i.iter().next().copied(),
                    Value::Primitive(PrimitiveValue::U32(u)) => u.iter().next().map(|&v| v as i32),
                    Value::Primitive(PrimitiveValue::I16(i)) => i.iter().next().map(|&v| v as i32),
                    Value::Primitive(PrimitiveValue::U16(u)) => u.iter().next().map(|&v| v as i32),
                    _ => {
                        debug!("标签 {:?} 不是整数类型", tag);
                        None
                    }
                }
            }
            Err(_) => {
                debug!("未找到标签: {:?}", tag);
                None
            }
        }
    }

    /// 获取DICOM传输语法
    pub fn get_transfer_syntax(transfer_syntax_uid: &str) -> Result<TransferSyntax> {
        // 简化实现，暂时不支持具体的传输语法对象
        // 只检查是否为已知的传输语法UID
        match transfer_syntax_uid {
            "1.2.840.10008.1.2.1" | "1.2.840.10008.1.2" | "1.2.840.10008.1.2.2" => {
                // 返回一个默认的传输语法，实际应用中需要创建正确的TransferSyntax对象
                Err(PacsError::DicomParseError("传输语法对象创建暂未实现".to_string()))
            }
            _ => {
                Err(PacsError::DicomParseError(format!("不支持的传输语法: {}", transfer_syntax_uid)))
            }
        }
    }
}

/// 解析后的DICOM对象
#[derive(Debug, Clone)]
pub struct ParsedDicomObject {
    // === 患者信息 ===
    /// 患者ID
    pub patient_id: Option<String>,
    /// 患者姓名
    pub patient_name: Option<String>,
    /// 患者出生日期
    pub patient_birth_date: Option<String>,
    /// 患者性别
    pub patient_sex: Option<String>,
    /// 患者年龄
    pub patient_age: Option<String>,
    /// 患者体重
    pub patient_weight: Option<String>,

    // === 检查信息 ===
    /// 检查实例UID
    pub study_instance_uid: Option<String>,
    /// 检查日期
    pub study_date: Option<String>,
    /// 检查时间
    pub study_time: Option<String>,
    /// 检查描述
    pub study_description: Option<String>,
    /// 检查号
    pub accession_number: Option<String>,

    // === 序列信息 ===
    /// 序列实例UID
    pub series_instance_uid: Option<String>,
    /// 序列号
    pub series_number: Option<String>,
    /// 序列描述
    pub series_description: Option<String>,
    /// 模态
    pub modality: Option<String>,

    // === 实例信息 ===
    /// SOP实例UID
    pub sop_instance_uid: Option<String>,
    /// SOP类UID
    pub sop_class_uid: Option<String>,
    /// 实例号
    pub instance_number: Option<String>,

    // === 设备信息 ===
    /// 机构名称
    pub institution_name: Option<String>,
    /// 制造商
    pub manufacturer: Option<String>,
    /// 制造商型号
    pub manufacturer_model_name: Option<String>,

    // === 图像信息 ===
    /// 图像行数
    pub rows: Option<i32>,
    /// 图像列数
    pub columns: Option<i32>,
    /// 分配位数
    pub bits_allocated: Option<i32>,
    /// 存储位数
    pub bits_stored: Option<i32>,
    /// 最高位
    pub high_bit: Option<i32>,
    /// 像素表示
    pub pixel_representation: Option<i32>,

    // === 传输语法 ===
    /// 传输语法UID
    pub transfer_syntax_uid: Option<String>,

    // === 其他信息 ===
    /// 检查部位
    pub body_part_examined: Option<String>,
}

impl Default for ParsedDicomObject {
    fn default() -> Self {
        Self::new()
    }
}

impl ParsedDicomObject {
    /// 创建新的空DICOM对象
    pub fn new() -> Self {
        Self {
            patient_id: None,
            patient_name: None,
            patient_birth_date: None,
            patient_sex: None,
            patient_age: None,
            patient_weight: None,
            study_instance_uid: None,
            study_date: None,
            study_time: None,
            study_description: None,
            accession_number: None,
            series_instance_uid: None,
            series_number: None,
            series_description: None,
            modality: None,
            sop_instance_uid: None,
            sop_class_uid: None,
            instance_number: None,
            institution_name: None,
            manufacturer: None,
            manufacturer_model_name: None,
            rows: None,
            columns: None,
            bits_allocated: None,
            bits_stored: None,
            high_bit: None,
            pixel_representation: None,
            transfer_syntax_uid: None,
            body_part_examined: None,
        }
    }

    // === 患者信息访问器 ===
    /// 获取患者ID
    pub fn get_patient_id(&self) -> Option<String> {
        self.patient_id.clone()
    }

    /// 获取患者姓名
    pub fn get_patient_name(&self) -> Option<String> {
        self.patient_name.clone()
    }

    /// 获取患者出生日期
    pub fn get_patient_birth_date(&self) -> Option<String> {
        self.patient_birth_date.clone()
    }

    /// 获取患者性别
    pub fn get_patient_sex(&self) -> Option<String> {
        self.patient_sex.clone()
    }

    /// 获取患者年龄
    pub fn get_patient_age(&self) -> Option<String> {
        self.patient_age.clone()
    }

    // === 检查信息访问器 ===
    /// 获取检查实例UID
    pub fn get_study_instance_uid(&self) -> Option<String> {
        self.study_instance_uid.clone()
    }

    /// 获取检查日期
    pub fn get_study_date(&self) -> Option<String> {
        self.study_date.clone()
    }

    /// 获取检查描述
    pub fn get_study_description(&self) -> Option<String> {
        self.study_description.clone()
    }

    /// 获取检查号
    pub fn get_accession_number(&self) -> Option<String> {
        self.accession_number.clone()
    }

    // === 序列信息访问器 ===
    /// 获取序列实例UID
    pub fn get_series_instance_uid(&self) -> Option<String> {
        self.series_instance_uid.clone()
    }

    /// 获取序列号
    pub fn get_series_number(&self) -> Option<String> {
        self.series_number.clone()
    }

    /// 获取序列描述
    pub fn get_series_description(&self) -> Option<String> {
        self.series_description.clone()
    }

    /// 获取模态
    pub fn get_modality(&self) -> Option<String> {
        self.modality.clone()
    }

    // === 实例信息访问器 ===
    /// 获取SOP实例UID
    pub fn get_sop_instance_uid(&self) -> Option<String> {
        self.sop_instance_uid.clone()
    }

    /// 获取SOP类UID
    pub fn get_sop_class_uid(&self) -> Option<String> {
        self.sop_class_uid.clone()
    }

    /// 获取实例号
    pub fn get_instance_number(&self) -> Option<String> {
        self.instance_number.clone()
    }

    // === 设备信息访问器 ===
    /// 获取机构名称
    pub fn get_institution_name(&self) -> Option<String> {
        self.institution_name.clone()
    }

    /// 获取制造商
    pub fn get_manufacturer(&self) -> Option<String> {
        self.manufacturer.clone()
    }

    // === 图像信息访问器 ===
    /// 获取图像尺寸 (行数, 列数)
    pub fn get_image_size(&self) -> Option<(i32, i32)> {
        match (self.rows, self.columns) {
            (Some(rows), Some(columns)) => Some((rows, columns)),
            _ => None,
        }
    }

    /// 获取图像行数
    pub fn get_rows(&self) -> Option<i32> {
        self.rows
    }

    /// 获取图像列数
    pub fn get_columns(&self) -> Option<i32> {
        self.columns
    }

    // === 传输语法访问器 ===
    /// 获取传输语法UID
    pub fn get_transfer_syntax_uid(&self) -> Option<String> {
        self.transfer_syntax_uid.clone()
    }

    // === 数据验证 ===
    /// 验证DICOM对象的完整性
    pub fn validate(&self) -> bool {
        // 检查必要的UID是否存在
        let required_uids = [
            &self.sop_class_uid,
            &self.sop_instance_uid,
            &self.study_instance_uid,
            &self.series_instance_uid,
        ];

        required_uids.iter().all(|uid| uid.is_some())
    }

    /// 检查是否包含像素数据
    pub fn has_pixel_data(&self) -> bool {
        self.rows.is_some() && self.columns.is_some()
    }

    /// 获取DICOM对象的摘要信息
    pub fn get_summary(&self) -> String {
        format!(
            "DICOM对象: 患者ID={}, 检查UID={}, 序列UID={}, 模态={}",
            self.patient_id.as_deref().unwrap_or("未知"),
            self.study_instance_uid.as_deref().unwrap_or("未知"),
            self.series_instance_uid.as_deref().unwrap_or("未知"),
            self.modality.as_deref().unwrap_or("未知")
        )
    }
}