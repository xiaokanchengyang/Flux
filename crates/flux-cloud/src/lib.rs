//! Cloud storage adaptation layer for Flux
//!
//! This crate provides adapters that make cloud storage objects (S3, GCS, Azure Blob)
//! appear as standard `Read`, `Write`, and `Seek` implementations that can be used
//! directly with flux-core's synchronous APIs.

pub mod error;
pub mod reader;
pub mod store;
pub mod writer;

pub use error::{CloudError, Result};
pub use reader::CloudReader;
pub use store::{CloudPath, CloudStore};
pub use writer::CloudWriter;

// Re-export commonly used types
pub use object_store::{ObjectMeta, ObjectStore};
