use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid cloud path: {0}")]
    InvalidPath(String),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("Buffer size exceeded: {0} bytes")]
    BufferSizeExceeded(usize),
}

pub type Result<T> = std::result::Result<T, CloudError>;

impl From<CloudError> for std::io::Error {
    fn from(err: CloudError) -> Self {
        match err {
            CloudError::Io(io_err) => io_err,
            other => std::io::Error::new(std::io::ErrorKind::Other, other),
        }
    }
}