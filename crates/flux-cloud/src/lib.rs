//! # flux-cloud
//!
//! Cloud storage adaptation layer for Flux. This crate provides synchronous `Read`, `Write`, and `Seek`
//! interfaces for cloud storage objects, allowing `flux-core` to work with cloud storage seamlessly
//! without dealing with async complexity.
//!
//! ## Architecture
//!
//! The main abstractions are:
//! - `CloudReader`: Implements `std::io::Read` and `std::io::Seek` for reading from cloud objects
//! - `CloudWriter`: Implements `std::io::Write` for writing to cloud objects
//!
//! These adapters use an internal Tokio runtime to bridge the async `object_store` API with
//! synchronous std::io traits.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

mod error;
mod reader;
mod writer;
mod runtime;
mod buffer;

pub use error::{CloudError, Result};
pub use reader::CloudReader;
pub use writer::CloudWriter;

// Re-export commonly used types from object_store
pub use object_store::{ObjectStore, ObjectMeta, path::Path as ObjectPath};

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

/// Creates a cloud store from a URL string
///
/// Supports URLs like:
/// - `s3://bucket/path/to/object`
/// - `gs://bucket/path/to/object`
/// - `az://container/path/to/object`
pub fn parse_cloud_url(url: &str) -> Result<(Box<dyn ObjectStore>, ObjectPath)> {
    use object_store::parse_url;
    
    let (store, path) = parse_url(&url.parse()?)?;
    Ok((store.into(), path))
}