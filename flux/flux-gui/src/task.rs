//! Background task handling for flux-gui

use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};

/// Commands sent from UI to background thread
pub enum TaskCommand {
    /// Pack files into an archive
    Pack {
        /// Input files/directories to pack
        inputs: Vec<PathBuf>,
        /// Output archive path
        output: PathBuf,
        /// Packing options
        options: flux_lib::archive::PackOptions,
        /// Cancel flag
        cancel_flag: Arc<AtomicBool>,
    },
    /// Extract an archive
    Extract {
        /// Archive file to extract
        archive: PathBuf,
        /// Directory to extract to
        output_dir: PathBuf,
        /// Enable smart directory hoisting
        hoist: bool,
        /// Cancel flag
        cancel_flag: Arc<AtomicBool>,
    },
    /// Sync/incremental backup
    Sync {
        /// Source directory
        source_dir: PathBuf,
        /// Target archive
        target_archive: PathBuf,
        /// Previous manifest path (if exists)
        old_manifest: Option<PathBuf>,
        /// Pack options
        options: flux_lib::archive::PackOptions,
        /// Cancel flag
        cancel_flag: Arc<AtomicBool>,
    },
}

/// Progress update from background thread
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Bytes processed so far
    pub processed_bytes: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current file being processed
    pub current_file: String,
    /// Processing speed in bytes per second
    pub speed_bps: f64,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<f64>,
}

/// Result of a background task
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Task completed successfully
    Success,
    /// Task failed with error message
    Error(String),
    /// Task was cancelled by user
    Cancelled,
}

/// Messages sent from background thread to UI
#[derive(Debug, Clone)]
pub enum ToUi {
    /// Progress update
    Progress(ProgressUpdate),
    /// Task finished
    Finished(TaskResult),
    /// Log message
    Log(String),
}