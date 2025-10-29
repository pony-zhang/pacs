//! DICOM数据验证模块
//!
//! 提供DICOM文件和数据的完整性与合规性验证功能

use crate::parser::ParsedDicomObject;
use crate::transfer_syntax::TransferSyntaxManager;
use tracing::{debug, info, warn};

/// DICOM数据验证器
pub struct DicomValidator {
    transfer_syntax_manager: TransferSyntaxManager,
}

impl Default for DicomValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DicomValidator {
    /// 创建新的DICOM验证器
    pub fn new() -> Self {
        Self {
            transfer_syntax_manager: TransferSyntaxManager::new(),
        }
    }

    /// 验证DICOM对象的完整性和合规性
    pub fn validate_dicom_object(&self, obj: &ParsedDicomObject) -> ValidationResult {
        let mut result = ValidationResult::new();

        info!("开始验证DICOM对象: {}", obj.get_summary());

        // 1. 验证必需的UID
        self.validate_required_uids(obj, &mut result);

        // 2. 验证患者信息
        self.validate_patient_info(obj, &mut result);

        // 3. 验证检查信息
        self.validate_study_info(obj, &mut result);

        // 4. 验证序列信息
        self.validate_series_info(obj, &mut result);

        // 5. 验证实例信息
        self.validate_instance_info(obj, &mut result);

        // 6. 验证传输语法
        self.validate_transfer_syntax(obj, &mut result);

        // 7. 验证图像信息
        self.validate_image_info(obj, &mut result);

        // 8. 验证日期时间格式
        self.validate_datetime_format(obj, &mut result);

        // 9. 验证UID格式
        self.validate_uid_format(obj, &mut result);

        info!("DICOM对象验证完成: {} 个错误, {} 个警告",
              result.errors.len(), result.warnings.len());

        result
    }

    /// 验证必需的UID
    fn validate_required_uids(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        let required_uids = [
            ("SOP Class UID", obj.sop_class_uid.as_ref()),
            ("SOP Instance UID", obj.sop_instance_uid.as_ref()),
            ("Study Instance UID", obj.study_instance_uid.as_ref()),
            ("Series Instance UID", obj.series_instance_uid.as_ref()),
        ];

        for (name, uid) in required_uids {
            match uid {
                Some(uid_value) if !uid_value.trim().is_empty() => {
                    if self.is_valid_uid(uid_value) {
                        debug!("{} 验证通过: {}", name, uid_value);
                    } else {
                        result.add_error(format!("{} 格式无效: {}", name, uid_value));
                    }
                }
                Some(_) => {
                    result.add_error(format!("{} 不能为空", name));
                }
                None => {
                    result.add_error(format!("{} 缺失", name));
                }
            }
        }
    }

    /// 验证患者信息
    fn validate_patient_info(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 患者ID验证
        match &obj.patient_id {
            Some(id) if !id.trim().is_empty() => {
                if id.len() > 64 {
                    result.add_warning("患者ID长度超过64字符".to_string());
                }
            }
            Some(_) => {
                result.add_warning("患者ID为空".to_string());
            }
            None => {
                result.add_error("患者ID缺失".to_string());
            }
        }

        // 患者姓名验证
        match &obj.patient_name {
            Some(name) if !name.trim().is_empty() => {
                if name.len() > 64 {
                    result.add_warning("患者姓名长度超过64字符".to_string());
                }
            }
            Some(_) => {
                result.add_warning("患者姓名为空".to_string());
            }
            None => {
                result.add_warning("患者姓名缺失".to_string());
            }
        }

        // 患者性别验证
        if let Some(sex) = &obj.patient_sex {
            if !["M", "F", "O"].contains(&sex.as_str()) {
                result.add_warning(format!("患者性别值无效: {}，应为M/F/O", sex));
            }
        }

        // 出生日期验证
        if let Some(birth_date) = &obj.patient_birth_date {
            if !self.is_valid_dicom_date(birth_date) {
                result.add_error(format!("患者出生日期格式无效: {}", birth_date));
            }
        }
    }

    /// 验证检查信息
    fn validate_study_info(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 检查日期验证
        if let Some(study_date) = &obj.study_date {
            if !self.is_valid_dicom_date(study_date) {
                result.add_error(format!("检查日期格式无效: {}", study_date));
            }
        }

        // 检查时间验证
        if let Some(study_time) = &obj.study_time {
            if !self.is_valid_dicom_time(study_time) {
                result.add_error(format!("检查时间格式无效: {}", study_time));
            }
        }

        // 检查号验证
        if let Some(accession_number) = &obj.accession_number {
            if accession_number.len() > 16 {
                result.add_warning("检查号长度超过16字符".to_string());
            }
        }
    }

    /// 验证序列信息
    fn validate_series_info(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 模态验证
        if let Some(modality) = &obj.modality {
            if !self.is_valid_modality(modality) {
                result.add_warning(format!("模态代码可能无效: {}", modality));
            }
        } else {
            result.add_error("模态信息缺失".to_string());
        }

        // 序列号验证
        if let Some(series_number) = &obj.series_number {
            if let Err(_) = series_number.parse::<i32>() {
                result.add_error(format!("序列号格式无效: {}", series_number));
            }
        }
    }

    /// 验证实例信息
    fn validate_instance_info(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 实例号验证
        if let Some(instance_number) = &obj.instance_number {
            if let Err(_) = instance_number.parse::<i32>() {
                result.add_error(format!("实例号格式无效: {}", instance_number));
            }
        }
    }

    /// 验证传输语法
    fn validate_transfer_syntax(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        if let Some(transfer_syntax_uid) = &obj.transfer_syntax_uid {
            if !self.transfer_syntax_manager.is_supported(transfer_syntax_uid) {
                result.add_error(format!("不支持的传输语法: {}", transfer_syntax_uid));
            } else {
                debug!("传输语法验证通过: {}", transfer_syntax_uid);
            }
        } else {
            result.add_warning("传输语法信息缺失".to_string());
        }
    }

    /// 验证图像信息
    fn validate_image_info(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 检查图像尺寸
        match (obj.rows, obj.columns) {
            (Some(rows), Some(columns)) => {
                if rows <= 0 || columns <= 0 {
                    result.add_error("图像尺寸必须为正数".to_string());
                } else if rows > 32768 || columns > 32768 {
                    result.add_warning("图像尺寸异常大，可能存在错误".to_string());
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                result.add_error("图像尺寸信息不完整，缺少行数或列数".to_string());
            }
            (None, None) => {
                // 可能是没有像素数据的DICOM对象，不报错
                debug!("未找到图像尺寸信息");
            }
        }

        // 检查像素数据相关字段
        if let (Some(bits_allocated), Some(bits_stored), Some(high_bit)) =
            (obj.bits_allocated, obj.bits_stored, obj.high_bit) {

            if bits_stored > bits_allocated {
                result.add_error("存储位数不能大于分配位数".to_string());
            }

            if high_bit + 1 != bits_stored {
                result.add_warning("最高位与存储位数不匹配".to_string());
            }

            if bits_allocated > 32 {
                result.add_warning("分配位数超过32位，可能存在错误".to_string());
            }
        }
    }

    /// 验证日期时间格式
    fn validate_datetime_format(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        // 验证所有日期字段
        let date_fields = [
            ("患者出生日期", &obj.patient_birth_date),
            ("检查日期", &obj.study_date),
        ];

        for (name, date_field) in date_fields {
            if let Some(date) = date_field {
                if !self.is_valid_dicom_date(date) {
                    result.add_error(format!("{}格式无效: {}", name, date));
                }
            }
        }

        // 验证所有时间字段
        let time_fields = [
            ("检查时间", &obj.study_time),
        ];

        for (name, time_field) in time_fields {
            if let Some(time) = time_field {
                if !self.is_valid_dicom_time(time) {
                    result.add_error(format!("{}格式无效: {}", name, time));
                }
            }
        }
    }

    /// 验证UID格式
    fn validate_uid_format(&self, obj: &ParsedDicomObject, result: &mut ValidationResult) {
        let uid_fields = [
            ("SOP类UID", &obj.sop_class_uid),
            ("SOP实例UID", &obj.sop_instance_uid),
            ("检查实例UID", &obj.study_instance_uid),
            ("序列实例UID", &obj.series_instance_uid),
            ("传输语法UID", &obj.transfer_syntax_uid),
        ];

        for (name, uid_field) in uid_fields {
            if let Some(uid) = uid_field {
                if !self.is_valid_uid(uid) {
                    result.add_error(format!("{}格式无效: {}", name, uid));
                }
            }
        }
    }

    /// 检查是否为有效的DICOM日期 (YYYYMMDD)
    fn is_valid_dicom_date(&self, date: &str) -> bool {
        if date.len() != 8 {
            return false;
        }

        if !date.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // 解析年月日
        if let (Ok(year), Ok(month), Ok(day)) = (
            date[0..4].parse::<u32>(),
            date[4..6].parse::<u32>(),
            date[6..8].parse::<u32>(),
        ) {
            match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => day <= 31,
                4 | 6 | 9 | 11 => day <= 30,
                2 => {
                    // 简单的闰年检查
                    if (year % 400 == 0) || (year % 100 != 0 && year % 4 == 0) {
                        day <= 29
                    } else {
                        day <= 28
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// 检查是否为有效的DICOM时间 (HHMMSS.FFFFFF)
    fn is_valid_dicom_time(&self, time: &str) -> bool {
        // 基本格式检查，允许小数部分
        let time_without_fraction = if let Some(dot_pos) = time.find('.') {
            &time[..dot_pos]
        } else {
            time
        };

        if time_without_fraction.len() < 2 || time_without_fraction.len() > 6 {
            return false;
        }

        if !time_without_fraction.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // 解析时分秒
        let hour = time_without_fraction[0..2].parse::<u32>();
        if hour.is_err() || hour.unwrap() > 23 {
            return false;
        }

        if time_without_fraction.len() >= 4 {
            let minute = time_without_fraction[2..4].parse::<u32>();
            if minute.is_err() || minute.unwrap() > 59 {
                return false;
            }
        }

        if time_without_fraction.len() >= 6 {
            let second = time_without_fraction[4..6].parse::<u32>();
            if second.is_err() || second.unwrap() > 60 { // 允许60秒（闰秒）
                return false;
            }
        }

        true
    }

    /// 检查是否为有效的UID格式
    fn is_valid_uid(&self, uid: &str) -> bool {
        if uid.is_empty() || uid.len() > 64 {
            return false;
        }

        // 基本格式检查：数字和点
        if !uid.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return false;
        }

        // 不能以点开头或结尾
        if uid.starts_with('.') || uid.ends_with('.') {
            return false;
        }

        // 不能有连续的点
        if uid.contains("..") {
            return false;
        }

        true
    }

    /// 检查是否为有效的DICOM模态代码
    fn is_valid_modality(&self, modality: &str) -> bool {
        // 常见的DICOM模态代码
        let valid_modalities = [
            "CR", "CT", "DX", "ES", "MG", "MR", "NM", "OT", "PT", "RF", "SC", "US", "XA",
            "XC", "RTIMAGE", "RTDOSE", "RTSTRUCT", "RTPLAN", "RTRECORD", "HC", "ST", "SEG",
            "VF", "BMD", "FID", "LEN", "DOC", "REG", "OAM", "OP", "OPT", "OPR", "PLAN",
            "RTION", "RWV", "SEG", "SMR", "TID", "VA", "XC", "XRT"
        ];

        valid_modalities.contains(&modality)
    }
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 验证错误列表
    pub errors: Vec<String>,
    /// 验证警告列表
    pub warnings: Vec<String>,
    /// 是否通过验证
    pub is_valid: bool,
}

impl ValidationResult {
    /// 创建新的验证结果
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// 添加警告
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 检查是否有警告
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 获取警告数量
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// 获取验证报告摘要
    pub fn get_summary(&self) -> String {
        if self.is_valid {
            if self.has_warnings() {
                format!("验证通过，但有 {} 个警告", self.warning_count())
            } else {
                "验证完全通过".to_string()
            }
        } else {
            format!("验证失败：{} 个错误，{} 个警告", self.error_count(), self.warning_count())
        }
    }

    /// 获取详细的验证报告
    pub fn get_detailed_report(&self) -> String {
        let mut report = String::new();

        if self.has_errors() {
            report.push_str("=== 验证错误 ===\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, error));
            }
            report.push('\n');
        }

        if self.has_warnings() {
            report.push_str("=== 验证警告 ===\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, warning));
            }
            report.push('\n');
        }

        report.push_str(&format!("=== 验证结果 ===\n{}\n", self.get_summary()));

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dicom_date_validation() {
        let validator = DicomValidator::new();

        assert!(validator.is_valid_dicom_date("20230101"));
        assert!(validator.is_valid_dicom_date("20240229")); // 闰年
        assert!(!validator.is_valid_dicom_date("20230229")); // 非闰年
        assert!(!validator.is_valid_dicom_date("20231301")); // 无效月份
        assert!(!validator.is_valid_dicom_date("20230132")); // 无效日期
        assert!(!validator.is_valid_dicom_date("2023011"));  // 长度错误
    }

    #[test]
    fn test_dicom_time_validation() {
        let validator = DicomValidator::new();

        assert!(validator.is_valid_dicom_time("123045"));
        assert!(validator.is_valid_dicom_time("123045.123456"));
        assert!(validator.is_valid_dicom_time("12"));
        assert!(validator.is_valid_dicom_time("1230"));
        assert!(!validator.is_valid_dicom_time("253045")); // 无效小时
        assert!(!validator.is_valid_dicom_time("126045")); // 无效分钟
        assert!(!validator.is_valid_dicom_time("123061")); // 无效秒数
    }

    #[test]
    fn test_uid_validation() {
        let validator = DicomValidator::new();

        assert!(validator.is_valid_uid("1.2.840.10008.1.2"));
        assert!(!validator.is_valid_uid(""));
        assert!(!validator.is_valid_uid(".1.2.3"));
        assert!(!validator.is_valid_uid("1.2.3."));
        assert!(!validator.is_valid_uid("1..2.3"));
        assert!(!validator.is_valid_uid("1.2.abc.3"));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();

        result.add_warning("测试警告".to_string());
        assert!(result.has_warnings());
        assert!(result.is_valid);

        result.add_error("测试错误".to_string());
        assert!(result.has_errors());
        assert!(!result.is_valid);

        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 1);
    }
}