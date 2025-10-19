//! Flux - A cross-platform file archiver and compressor library
//!
//! This library provides core functionality for archiving, extracting,
//! and compressing files with intelligent compression strategies.

pub mod archive;
pub mod error;
pub mod metadata;

pub use error::{Error, Result};

// Re-export commonly used types
pub use archive::{extract, pack};
