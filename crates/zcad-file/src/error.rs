//! 文件操作错误定义

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MessagePack encode error: {0}")]
    MsgPackEncode(#[from] rmp_serde::encode::Error),

    #[error("MessagePack decode error: {0}")]
    MsgPackDecode(#[from] rmp_serde::decode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DXF error: {0}")]
    Dxf(String),

    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(u64),

    #[error("Corruption detected: {0}")]
    Corruption(String),
}
