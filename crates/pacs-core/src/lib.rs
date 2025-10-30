//! # PACS Core
//!
//! PACS系统的核心模块，提供基础数据结构、错误定义和通用工具。

pub mod error;
pub mod models;
pub mod utils;

pub use error::{PacsError, Result};
pub use models::*;
