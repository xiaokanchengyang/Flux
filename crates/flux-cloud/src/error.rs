//! Error types for flux-cloud

use std::fmt;

/// Result type for flux-cloud operations
pub type Result<T> = std::result::Result<T, CloudError>;

/// Errors that can occur during cloud storage operations
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    /// Object store error
    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    /// Runtime error
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    /// Buffer overflow
    #[error("Buffer overflow: attempted to write {attempted} bytes to buffer with {available} bytes available")]
    BufferOverflow {
        /// Number of bytes attempted to write
        attempted: usize,
        /// Number of bytes available in buffer
        available: usize,
    },
    
    /// Invalid seek position
    #[error("Invalid seek position: {0}")]
    InvalidSeek(String),
}

impl From<CloudError> for std::io::Error {
    fn from(err: CloudError) -> Self {
        match err {
            CloudError::Io(e) => e,
            other => std::io::Error::new(std::io::ErrorKind::Other, other),
        }
    }
}