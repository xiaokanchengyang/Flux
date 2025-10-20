//! Flux - A cross-platform file archiver and compressor library
//!
//! This library provides core functionality for archiving, extracting,
//! and compressing files with intelligent compression strategies.

pub mod archive;
pub mod config;
pub mod error;
pub mod metadata;
pub mod progress;
pub mod strategy;
pub mod interactive;
pub mod manifest;

pub use error::{Error, Result};

// Re-export commonly used types
pub use archive::{
    extract, extract_with_options, inspect, pack, pack_with_strategy, ArchiveEntry, ExtractOptions,
    PackOptions, create_extractor,
};
