//! Flux GUI Application module

mod state;
mod events;
mod ui;

pub use state::{FluxApp, AppView};

use std::thread;
use egui_notify::Toasts;

use crate::task::{TaskCommand, ToUi};

impl FluxApp {
    /// Create a new application instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create channels for communication
        let (task_sender, task_receiver) = crossbeam_channel::unbounded::<TaskCommand>();
        let (ui_sender, ui_receiver) = crossbeam_channel::unbounded::<ToUi>();
        
        // Create channel for log messages
        let (log_sender, log_receiver) = crossbeam_channel::unbounded::<(tracing::Level, String)>();
        
        // Re-initialize tracing with GUI integration
        crate::logging::init_tracing(Some(log_sender));
        
        // Load persistent state
        let persistence = Self::load_persistence(cc.storage);
        
        // Create theme based on saved preference
        let theme = if persistence.dark_mode {
            crate::theme::FluxTheme::dark()
        } else {
            crate::theme::FluxTheme::light()
        };
        
        // Spawn background thread
        let task_handle = thread::spawn(move || {
            // Background thread main loop
            loop {
                match task_receiver.recv() {
                    Ok(command) => {
                        match command {
                            TaskCommand::Pack { inputs, output, options, cancel_flag } => {
                                crate::handle_pack_task(inputs, output, options, cancel_flag, &ui_sender);
                            }
                            TaskCommand::Extract { archive, output_dir, hoist, cancel_flag } => {
                                crate::handle_extract_task(archive, output_dir, hoist, cancel_flag, &ui_sender);
                            }
                            TaskCommand::Sync { source_dir, target_archive, old_manifest, options, cancel_flag } => {
                                crate::handle_sync_task(source_dir, target_archive, old_manifest, options, cancel_flag, &ui_sender);
                            }
                        }
                    }
                    Err(_) => {
                        // Channel closed, exit thread
                        break;
                    }
                }
            }
        });
        
        Self {
            view: AppView::Welcome,
            task_sender,
            ui_receiver,
            _task_handle: Some(task_handle),
            current_progress: 0.0,
            status_text: "Ready".to_string(),
            current_file: String::new(),
            processed_bytes: 0,
            total_bytes: 0,
            input_files: Vec::new(),
            output_path: None,
            compression_format: persistence.preferred_format.unwrap_or_else(|| "tar.zst".to_string()),
            is_busy: false,
            toasts: Toasts::default(),
            cancel_flag: None,
            logs: Vec::new(),
            show_log_panel: persistence.show_log_panel,
            log_receiver: Some(log_receiver),
            current_speed_bps: 0.0,
            eta_seconds: None,
            log_filter: String::new(),
            log_level_filter: None,
            error_details: None,
            show_error_modal: false,
            theme,
            sync_source_dir: None,
            sync_target_archive: None,
            sync_manifest_path: None,
            show_about_dialog: false,
            sidebar: crate::layout::Sidebar::default(),
            browser_state: None,
            extract_hoist: false,
        }
    }
}