//! Error types for flux-lib

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Zip error: {0}")]
    Zip(String),
    
    #[error("Archive error: {0}")]
    ArchiveError(String),
    
    #[error("File exists: {0}")]
    FileExists(std::path::PathBuf),
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    
    #[error("Partial failure: {count} operations failed")]
    PartialFailure { count: u32 },
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
