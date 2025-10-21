//! Error types for flux-core

use std::path::PathBuf;
use thiserror::Error;

/// Core error types for the flux library
#[derive(Error, Debug)]
pub enum Error {
    /// I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Archive-related error occurred
    #[error("Archive error: {0}")]
    Archive(String),

    /// Compression/decompression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Invalid file or directory path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Unsupported archive format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Configuration-related error
    #[error("Configuration error: {0}")]
    Config(String),

    /// ZIP-specific error
    #[error("Zip error: {0}")]
    Zip(String),

    /// File already exists at destination
    #[error("File exists: {0}")]
    FileExists(PathBuf),

    /// Operation not supported
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Some operations failed during batch processing
    #[error("Partial failure: {count} operations failed")]
    PartialFailure { count: u32 },

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Security violation detected
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Generic error for other cases
    #[error("Other error: {0}")]
    Other(String),
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        Error::Zip(err.to_string())
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        Error::Io(err.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
