//! # DICOM服务模块
//!
//! 提供DICOM协议的实现，包括C-STORE、C-FIND、C-MOVE、C-ECHO等服务。

pub mod server;
pub mod services;
pub mod association;
pub mod dimse;
pub mod parser;
pub mod transfer_syntax;
pub mod validator;

pub use server::{DicomServer, DicomServerConfig};
pub use services::*;
pub use parser::{DicomParser, ParsedDicomObject};
pub use transfer_syntax::{TransferSyntaxManager, TransferSyntaxInfo};
pub use validator::{DicomValidator, ValidationResult};