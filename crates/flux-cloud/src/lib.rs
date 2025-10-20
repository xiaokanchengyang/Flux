//! Cloud storage adaptation layer for Flux
//!
//! This crate provides adapters that make cloud storage objects (S3, GCS, Azure Blob)
//! appear as standard `Read`, `Write`, and `Seek` implementations that can be used
//! directly with flux-core's synchronous APIs.

pub mod error;
pub mod reader;
pub mod writer;
pub mod store;

pub use error::{CloudError, Result};
pub use reader::CloudReader;
pub use writer::{CloudWriter, CloudWriterGuard};
pub use store::{CloudStore, CloudPath};

// Re-export commonly used types
pub use object_store::{ObjectStore, ObjectMeta};

/// Configuration for cloud storage operations
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Size of the read buffer in bytes (default: 8MB)
    pub read_buffer_size: usize,
    /// Size of the write buffer in bytes (default: 8MB)
    pub write_buffer_size: usize,
    /// Number of chunks to cache for read operations (default: 4)
    pub read_cache_size: usize,
    /// Whether to use multipart upload for large files (default: true)
    pub use_multipart_upload: bool,
    /// Threshold for multipart upload in bytes (default: 64MB)
    pub multipart_threshold: usize,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            read_buffer_size: 8 * 1024 * 1024,    // 8MB
            write_buffer_size: 8 * 1024 * 1024,   // 8MB
            read_cache_size: 4,
            use_multipart_upload: true,
            multipart_threshold: 64 * 1024 * 1024, // 64MB
        }
    }
}

/// Parse a cloud URL and create a store and path
pub fn parse_cloud_url(url: &str) -> Result<(CloudStore, object_store::path::Path)> {
    let cloud_path = CloudPath::parse(url)?;
    let store = CloudStore::new(&cloud_path)?;
    Ok((store, cloud_path.path))
}