//! Application state management

use crossbeam_channel::{Receiver, Sender};
use std::{thread, path::PathBuf, sync::{Arc, atomic::AtomicBool}};
use egui_notify::Toasts;

use crate::task::TaskCommand;
use crate::task::ToUi;
use crate::theme::FluxTheme;

/// Application view states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    /// Welcome/idle view
    Welcome,
    /// Packing files into archive view
    Packing,
    /// Extracting archive view
    Extracting,
    /// Syncing/incremental backup view
    Syncing,
}

/// Main application structure
pub struct FluxApp {
    /// Current view
    pub(super) view: AppView,
    /// Sender for commands to background thread
    pub(super) task_sender: Sender<TaskCommand>,
    /// Receiver for messages from background thread
    pub(super) ui_receiver: Receiver<ToUi>,
    /// Handle to the background thread
    pub(super) _task_handle: Option<thread::JoinHandle<()>>,
    /// Current progress (0.0 to 1.0)
    pub(super) current_progress: f32,
    /// Status text to display
    pub(super) status_text: String,
    /// Current file being processed
    pub(super) current_file: String,
    /// Bytes processed
    pub(super) processed_bytes: u64,
    /// Total bytes to process
    pub(super) total_bytes: u64,
    /// Files to process
    pub(super) input_files: Vec<PathBuf>,
    /// Output path
    pub(super) output_path: Option<PathBuf>,
    /// Selected compression format for packing
    pub(super) compression_format: String,
    /// Is task running
    pub(super) is_busy: bool,
    /// Toast notifications
    pub(super) toasts: Toasts,
    /// Cancel flag for current task
    pub(super) cancel_flag: Option<Arc<AtomicBool>>,
    /// Log messages with level
    pub(super) logs: Vec<(tracing::Level, String)>,
    /// Show log panel
    pub(super) show_log_panel: bool,
    /// Receiver for log messages from tracing
    pub(super) log_receiver: Option<Receiver<(tracing::Level, String)>>,
    /// Current processing speed in bytes per second
    pub(super) current_speed_bps: f64,
    /// Estimated time remaining in seconds
    pub(super) eta_seconds: Option<f64>,
    /// Log search filter
    pub(super) log_filter: String,
    /// Selected log level filter
    pub(super) log_level_filter: Option<tracing::Level>,
    /// Current error details for modal dialog
    pub(super) error_details: Option<(String, String)>, // (summary, details)
    /// Show error modal
    pub(super) show_error_modal: bool,
    /// Application theme
    pub(super) theme: FluxTheme,
    /// Source directory for sync
    pub(super) sync_source_dir: Option<PathBuf>,
    /// Target archive for sync
    pub(super) sync_target_archive: Option<PathBuf>,
    /// Existing manifest path (if found)
    pub(super) sync_manifest_path: Option<PathBuf>,
}