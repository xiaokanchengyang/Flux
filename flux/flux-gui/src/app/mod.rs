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
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Create channels for communication
        let (task_sender, task_receiver) = crossbeam_channel::unbounded::<TaskCommand>();
        let (ui_sender, ui_receiver) = crossbeam_channel::unbounded::<ToUi>();
        
        // Create channel for log messages
        let (log_sender, log_receiver) = crossbeam_channel::unbounded::<String>();
        
        // Re-initialize tracing with GUI integration
        crate::logging::init_tracing(Some(log_sender));
        
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
                            TaskCommand::Extract { archive, output_dir, cancel_flag } => {
                                crate::handle_extract_task(archive, output_dir, cancel_flag, &ui_sender);
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
            compression_format: "tar.zst".to_string(),
            is_busy: false,
            toasts: Toasts::default(),
            cancel_flag: None,
            logs: Vec::new(),
            show_log_panel: false,
            log_receiver: Some(log_receiver),
            current_speed_bps: 0.0,
            eta_seconds: None,
        }
    }
}