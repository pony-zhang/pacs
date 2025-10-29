//! 通用工具函数

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 生成唯一的DICOM标识符
pub fn generate_dicom_uid() -> String {
    format!("{}.{}.{}.{}",
        "1.2.826.0.1.3680043.9.7382", // 企业根标识符
        Uuid::new_v4().simple(),
        Utc::now().timestamp(),
        std::process::id()
    )
}

/// 验证DICOM UID格式
pub fn is_valid_dicom_uid(uid: &str) -> bool {
    // 简单的DICOM UID验证逻辑
    !uid.is_empty() && uid.len() <= 64 && uid.chars().all(|c| c.is_numeric() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dicom_uid() {
        let uid = generate_dicom_uid();
        assert!(is_valid_dicom_uid(&uid));
    }

    #[test]
    fn test_is_valid_dicom_uid() {
        assert!(is_valid_dicom_uid("1.2.840.10008.5.1.4.1.1.4"));
        assert!(!is_valid_dicom_uid(""));
        assert!(!is_valid_dicom_uid("invalid.uid.with.letters"));
    }
}