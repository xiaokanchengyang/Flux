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
pub use writer::CloudWriter;
pub use store::{CloudStore, CloudPath};

// Re-export commonly used types
pub use object_store::{ObjectStore, ObjectMeta};