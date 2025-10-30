//! DICOM传输语法支持模块
//!
//! 提供多种DICOM传输语法的支持和处理功能

use dicom::encoding::TransferSyntax;
use pacs_core::{PacsError, Result};
use tracing::warn;

/// DICOM传输语法管理器
pub struct TransferSyntaxManager;

impl Default for TransferSyntaxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferSyntaxManager {
    /// 创建新的传输语法管理器
    pub fn new() -> Self {
        Self
    }

    /// 根据UID获取传输语法
    pub fn get_transfer_syntax(&self, uid: &str) -> Result<TransferSyntax> {
        // 简化实现，暂时不创建实际的TransferSyntax对象
        match uid {
            "1.2.840.10008.1.2.1" | "1.2.840.10008.1.2" | "1.2.840.10008.1.2.2" => Err(
                PacsError::DicomParseError("传输语法对象创建暂未实现".to_string()),
            ),
            _ => {
                warn!("不支持的传输语法: {}", uid);
                Err(PacsError::DicomParseError(format!(
                    "不支持的传输语法: {}",
                    uid
                )))
            }
        }
    }

    /// 检查传输语法是否支持
    pub fn is_supported(&self, uid: &str) -> bool {
        matches!(
            uid,
            "1.2.840.10008.1.2.1" | "1.2.840.10008.1.2" | "1.2.840.10008.1.2.2"
        )
    }

    /// 获取所有支持的传输语法
    pub fn get_supported_syntaxes(&self) -> Vec<&'static str> {
        vec![
            "1.2.840.10008.1.2.1", // Explicit VR Little Endian
            "1.2.840.10008.1.2",   // Implicit VR Little Endian
            "1.2.840.10008.1.2.2", // Explicit VR Big Endian
        ]
    }

    /// 检查传输语法是否支持压缩
    pub fn is_compressed(&self, uid: &str) -> Result<bool> {
        // 检查是否为压缩的传输语法
        let compressed_uids = [
            "1.2.840.10008.1.2.4.50", // JPEG Baseline
            "1.2.840.10008.1.2.4.51", // JPEG Extended
            "1.2.840.10008.1.2.4.57", // JPEG Lossless
            "1.2.840.10008.1.2.4.70", // JPEG Lossless SV1
            "1.2.840.10008.1.2.4.80", // JPEG-LS Lossless
            "1.2.840.10008.1.2.4.81", // JPEG-LS Near Lossless
            "1.2.840.10008.1.2.4.90", // JPEG 2000 Lossless
            "1.2.840.10008.1.2.4.91", // JPEG 2000
            "1.2.840.10008.1.2.5",    // RLE Lossless
        ];

        Ok(compressed_uids.contains(&uid))
    }

    /// 检查传输语法是否为隐式VR little endian
    pub fn is_implicit_vr_little_endian(&self, uid: &str) -> Result<bool> {
        Ok(uid == "1.2.840.10008.1.2")
    }

    /// 检查传输语法是否为显式VR little endian
    pub fn is_explicit_vr_little_endian(&self, uid: &str) -> Result<bool> {
        Ok(uid == "1.2.840.10008.1.2.1")
    }

    /// 检查传输语法是否为显式VR big endian
    pub fn is_explicit_vr_big_endian(&self, uid: &str) -> Result<bool> {
        Ok(uid == "1.2.840.10008.1.2.2")
    }

    /// 获取传输语法的描述信息
    pub fn get_transfer_syntax_info(&self, uid: &str) -> Result<TransferSyntaxInfo> {
        Ok(TransferSyntaxInfo {
            uid: uid.to_string(),
            name: self.get_transfer_syntax_name(uid),
            is_compressed: self.is_compressed(uid)?,
            is_implicit_vr: self.is_implicit_vr_little_endian(uid)?,
            is_explicit_vr: self.is_explicit_vr_little_endian(uid)?
                || self.is_explicit_vr_big_endian(uid)?,
            is_big_endian: self.is_explicit_vr_big_endian(uid)?,
        })
    }

    /// 获取传输语法的名称
    fn get_transfer_syntax_name(&self, uid: &str) -> String {
        match uid {
            "1.2.840.10008.1.2" => "Implicit VR Little Endian".to_string(),
            "1.2.840.10008.1.2.1" => "Explicit VR Little Endian".to_string(),
            "1.2.840.10008.1.2.2" => "Explicit VR Big Endian".to_string(),
            "1.2.840.10008.1.2.4.50" => "JPEG Baseline (Process 1)".to_string(),
            "1.2.840.10008.1.2.4.51" => "JPEG Extended (Process 2 & 4)".to_string(),
            "1.2.840.10008.1.2.4.57" => "JPEG Lossless (Process 14)".to_string(),
            "1.2.840.10008.1.2.4.70" => {
                "JPEG Lossless, Non-Hierarchical, First-Order Prediction".to_string()
            }
            "1.2.840.10008.1.2.4.80" => "JPEG-LS Lossless Image Compression".to_string(),
            "1.2.840.10008.1.2.4.81" => "JPEG-LS Near Lossless Image Compression".to_string(),
            "1.2.840.10008.1.2.4.90" => "JPEG 2000 Image Compression (Lossless Only)".to_string(),
            "1.2.840.10008.1.2.4.91" => "JPEG 2000 Image Compression".to_string(),
            "1.2.840.10008.1.2.5" => "RLE Lossless".to_string(),
            "1.2.840.10008.1.2.99" => "Deflated Explicit VR Little Endian".to_string(),
            _ => format!("Unknown Transfer Syntax ({})", uid),
        }
    }
}

/// 传输语法信息
#[derive(Debug, Clone)]
pub struct TransferSyntaxInfo {
    /// 传输语法UID
    pub uid: String,
    /// 传输语法名称
    pub name: String,
    /// 是否为压缩格式
    pub is_compressed: bool,
    /// 是否为隐式VR
    pub is_implicit_vr: bool,
    /// 是否为显式VR
    pub is_explicit_vr: bool,
    /// 是否为大端序
    pub is_big_endian: bool,
}

/// 常用的传输语法UID常量
pub mod transfer_syntax_uids {
    /// 隐式VR Little Endian (默认传输语法)
    pub const IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";

    /// 显式VR Little Endian
    pub const EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";

    /// 显式VR Big Endian
    pub const EXPLICIT_VR_BIG_ENDIAN: &str = "1.2.840.10008.1.2.2";

    /// JPEG Baseline (Process 1)
    pub const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";

    /// JPEG Extended (Process 2 & 4)
    pub const JPEG_EXTENDED: &str = "1.2.840.10008.1.2.4.51";

    /// JPEG Lossless (Process 14)
    pub const JPEG_LOSSLESS: &str = "1.2.840.10008.1.2.4.57";

    /// JPEG Lossless, Non-Hierarchical, First-Order Prediction
    pub const JPEG_LOSSLESS_SV1: &str = "1.2.840.10008.1.2.4.70";

    /// JPEG 2000 Image Compression (Lossless Only)
    pub const JPEG_2000_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";

    /// JPEG 2000 Image Compression
    pub const JPEG_2000: &str = "1.2.840.10008.1.2.4.91";

    /// RLE Lossless
    pub const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

    /// Deflated Explicit VR Little Endian
    pub const DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.99";
}

/// 传输语法工具函数
pub mod utils {
    use super::*;

    /// 检查UID是否为有效的传输语法
    pub fn is_valid_transfer_syntax(uid: &str) -> bool {
        // 基本的UID格式检查
        if uid.is_empty() || !uid.starts_with('1') || !uid.contains('.') {
            return false;
        }

        // 检查是否为已知的传输语法
        let known_syntaxes = [
            transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN,
            transfer_syntax_uids::EXPLICIT_VR_LITTLE_ENDIAN,
            transfer_syntax_uids::EXPLICIT_VR_BIG_ENDIAN,
            transfer_syntax_uids::JPEG_BASELINE,
            transfer_syntax_uids::JPEG_EXTENDED,
            transfer_syntax_uids::JPEG_LOSSLESS,
            transfer_syntax_uids::JPEG_LOSSLESS_SV1,
            transfer_syntax_uids::JPEG_2000_LOSSLESS,
            transfer_syntax_uids::JPEG_2000,
            transfer_syntax_uids::RLE_LOSSLESS,
            transfer_syntax_uids::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN,
        ];

        known_syntaxes.contains(&uid)
    }

    /// 获取推荐的传输语法（用于存储）
    pub fn get_recommended_transfer_syntax() -> &'static str {
        // 默认推荐使用显式VR Little Endian
        transfer_syntax_uids::EXPLICIT_VR_LITTLE_ENDIAN
    }

    /// 获取兼容性最好的传输语法
    pub fn get_most_compatible_transfer_syntax() -> &'static str {
        // 隐式VR Little Endian兼容性最好
        transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_syntax_manager() {
        let manager = TransferSyntaxManager::new();

        // 测试支持的标准传输语法
        assert!(manager.is_supported(transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN));
        assert!(manager.is_supported(transfer_syntax_uids::EXPLICIT_VR_LITTLE_ENDIAN));
        assert!(manager.is_supported(transfer_syntax_uids::EXPLICIT_VR_BIG_ENDIAN));

        // 测试不支持的传输语法
        assert!(!manager.is_supported("1.2.3.4.5.6.7.8.9"));
    }

    #[test]
    fn test_transfer_syntax_info() {
        let manager = TransferSyntaxManager::new();

        let info = manager
            .get_transfer_syntax_info(transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN)
            .unwrap();
        assert_eq!(info.uid, transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN);
        assert!(info.is_implicit_vr);
        assert!(!info.is_compressed);
    }

    #[test]
    fn test_utils() {
        assert!(utils::is_valid_transfer_syntax(
            transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN
        ));
        assert!(!utils::is_valid_transfer_syntax("invalid.uid"));

        assert_eq!(
            utils::get_recommended_transfer_syntax(),
            transfer_syntax_uids::EXPLICIT_VR_LITTLE_ENDIAN
        );
        assert_eq!(
            utils::get_most_compatible_transfer_syntax(),
            transfer_syntax_uids::IMPLICIT_VR_LITTLE_ENDIAN
        );
    }
}
