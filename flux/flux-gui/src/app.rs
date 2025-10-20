//! Flux GUI Application structure and logic

use eframe::egui;
use egui_notify::Toasts;
use crossbeam_channel::{Receiver, Sender};
use std::{thread, path::PathBuf, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::SystemTime};

use crate::task::{TaskCommand, ToUi, TaskResult};
use crate::views::{draw_packing_view, PackingAction, draw_extracting_view, ExtractingAction};

/// Application view states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    /// Welcome/idle view
    Welcome,
    /// Packing files into archive view
    Packing,
    /// Extracting archive view
    Extracting,
}

/// Main application structure
pub struct FluxApp {
    /// Current view
    view: AppView,
    /// Sender for commands to background thread
    task_sender: Sender<TaskCommand>,
    /// Receiver for messages from background thread
    ui_receiver: Receiver<ToUi>,
    /// Handle to the background thread
    _task_handle: Option<thread::JoinHandle<()>>,
    /// Current progress (0.0 to 1.0)
    current_progress: f32,
    /// Status text to display
    status_text: String,
    /// Current file being processed
    current_file: String,
    /// Bytes processed
    processed_bytes: u64,
    /// Total bytes to process
    total_bytes: u64,
    /// Files to process
    input_files: Vec<PathBuf>,
    /// Output path
    output_path: Option<PathBuf>,
    /// Selected compression format for packing
    compression_format: String,
    /// Is task running
    is_busy: bool,
    /// Toast notifications
    toasts: Toasts,
    /// Cancel flag for current task
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Log messages
    logs: Vec<String>,
    /// Show log panel
    show_log_panel: bool,
}

impl FluxApp {
    /// Create a new application instance
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Create channels for communication
        let (task_sender, task_receiver) = crossbeam_channel::unbounded::<TaskCommand>();
        let (ui_sender, ui_receiver) = crossbeam_channel::unbounded::<ToUi>();
        
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
        }
    }
    
    /// Analyze dropped files and switch view accordingly
    fn analyze_dropped_files(&mut self, files: Vec<PathBuf>) {
        if files.is_empty() {
            return;
        }
        
        // Check if it's a single archive file
        if files.len() == 1 {
            let file = &files[0];
            let file_name = file.file_name().map(|n| n.to_string_lossy().to_string());
            
            // Check for common archive extensions
            if let Some(ext) = file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "br") {
                    // Switch to extracting view
                    self.view = AppView::Extracting;
                    self.input_files = files;
                    self.toasts.info(format!("Ready to extract: {}", file_name.as_deref().unwrap_or("archive")));
                    return;
                }
            }
            
            // Also check for compound extensions like .tar.gz
            if let Some(name) = &file_name {
                let name_lower = name.to_lowercase();
                if name_lower.ends_with(".tar.gz") || name_lower.ends_with(".tar.zst") || 
                   name_lower.ends_with(".tar.xz") || name_lower.ends_with(".tar.br") {
                    // Switch to extracting view
                    self.view = AppView::Extracting;
                    self.input_files = files;
                    self.toasts.info(format!("Ready to extract: {}", name));
                    return;
                }
            }
        }
        
        // Multiple files, single non-archive file, or directories - switch to packing view
        self.view = AppView::Packing;
        let count = files.len();
        self.input_files = files;
        self.toasts.info(format!("Ready to pack {} file{}", count, if count == 1 { "" } else { "s" }));
    }
    
    /// Draw the welcome view
    fn draw_welcome_view(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Welcome to Flux");
            ui.add_space(20.0);
            ui.label("Drop files or archives here to get started");
            ui.add_space(40.0);
            
            // Large drop zone
            let drop_zone_size = egui::vec2(ui.available_width() * 0.8, 200.0);
            let (rect, response) = ui.allocate_exact_size(drop_zone_size, egui::Sense::click());
            
            // Draw drop zone
            let painter = ui.painter();
            painter.rect_filled(rect, 10.0, egui::Color32::from_gray(40));
            painter.rect_stroke(rect, 10.0, egui::Stroke::new(2.0, egui::Color32::from_gray(80)));
            
            // Drop zone text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "🎯 Drop files here",
                egui::FontId::proportional(24.0),
                egui::Color32::from_gray(150),
            );
            
            // Subtitle
            painter.text(
                rect.center() + egui::vec2(0.0, 40.0),
                egui::Align2::CENTER_CENTER,
                "or click to browse",
                egui::FontId::proportional(16.0),
                egui::Color32::from_gray(120),
            );
            
            // Handle click on drop zone
            if response.clicked() {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    self.analyze_dropped_files(files);
                }
            }
            
            ui.add_space(40.0);
            
            // Info section
            ui.separator();
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                ui.label("📦");
                ui.label("Drop multiple files to create an archive");
            });
            ui.horizontal(|ui| {
                ui.label("📂");
                ui.label("Drop an archive file to extract it");
            });
        });
    }
    
    /// Cancel the current task
    fn cancel_task(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
            self.toasts.info("Cancelling task...");
        }
    }
    
    /// Reset to welcome view
    fn reset_to_welcome(&mut self) {
        self.view = AppView::Welcome;
        self.input_files.clear();
        self.output_path = None;
        self.current_progress = 0.0;
        self.status_text = "Ready".to_string();
        self.cancel_flag = None;
    }
    
    /// Start the task based on current view and inputs
    fn start_task(&mut self) {
        match self.view {
            AppView::Packing => {
                if let Some(output) = &self.output_path {
                    // Validate output path
                    if let Some(parent) = output.parent() {
                        if !parent.exists() {
                            self.toasts.error("Output directory does not exist");
                            return;
                        }
                    }
                    
                    // Determine the algorithm based on the selected compression format
                    let algorithm = match self.compression_format.as_str() {
                        "tar.gz" => Some("gz".to_string()),
                        "tar.zst" => Some("zst".to_string()),
                        "tar.xz" => Some("xz".to_string()),
                        "zip" => Some("zip".to_string()),
                        _ => None,
                    };
                    
                    let options = flux_lib::archive::PackOptions {
                        smart: false, // Disable smart mode since user explicitly selected format
                        algorithm,
                        level: None,
                        threads: None,
                        force_compress: false,
                        follow_symlinks: false,
                    };
                    
                    // Create cancel flag
                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());
                    
                    let command = TaskCommand::Pack {
                        inputs: self.input_files.clone(),
                        output: output.clone(),
                        options,
                        cancel_flag,
                    };
                    
                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting pack operation...".to_string();
                        self.toasts.info("Starting to create archive...");
                    } else {
                        self.toasts.error("Failed to start task: background thread not responding");
                    }
                } else {
                    self.toasts.error("Please select an output path first");
                }
            }
            AppView::Extracting => {
                if let (Some(archive), Some(output_dir)) = (self.input_files.first(), &self.output_path) {
                    // Validate archive exists
                    if !archive.exists() {
                        self.toasts.error("Archive file not found");
                        return;
                    }
                    
                    // Validate output directory exists
                    if !output_dir.exists() {
                        self.toasts.error("Output directory does not exist");
                        return;
                    }
                    
                    // Create cancel flag
                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());
                    
                    let command = TaskCommand::Extract {
                        archive: archive.clone(),
                        output_dir: output_dir.clone(),
                        cancel_flag,
                    };
                    
                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting extraction...".to_string();
                        self.toasts.info("Starting extraction...");
                    } else {
                        self.toasts.error("Failed to start task: background thread not responding");
                    }
                } else {
                    self.toasts.error("Please select an archive and output directory first");
                }
            }
            AppView::Welcome => {}
        }
    }
}

impl eframe::App for FluxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update window title based on current state
        let title = match (self.view, self.is_busy) {
            (AppView::Packing, true) => "Flux - Packing...",
            (AppView::Packing, false) => "Flux - Pack Files",
            (AppView::Extracting, true) => "Flux - Extracting...",
            (AppView::Extracting, false) => "Flux - Extract Archive",
            (AppView::Welcome, _) => "Flux - File Archiver",
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.to_string()));
        // Check for dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut files = Vec::new();
                for dropped in &i.raw.dropped_files {
                    if let Some(path) = &dropped.path {
                        files.push(path.clone());
                    }
                }
                self.analyze_dropped_files(files);
            }
        });
        
        // Process all pending UI messages
        while let Ok(msg) = self.ui_receiver.try_recv() {
            match msg {
                ToUi::Progress(update) => {
                    self.current_progress = update.processed_bytes as f32 / update.total_bytes.max(1) as f32;
                    self.current_file = update.current_file.clone();
                    self.processed_bytes = update.processed_bytes;
                    self.total_bytes = update.total_bytes;
                    
                    // Format status text with size information
                    let processed_mb = update.processed_bytes as f64 / (1024.0 * 1024.0);
                    let total_mb = update.total_bytes as f64 / (1024.0 * 1024.0);
                    
                    if update.total_bytes > 0 {
                        let percent = (self.current_progress * 100.0) as u32;
                        self.status_text = format!("{:.1} / {:.1} MB ({}%)", processed_mb, total_mb, percent);
                    } else {
                        self.status_text = "Processing...".to_string();
                    }
                }
                ToUi::Finished(result) => {
                    self.is_busy = false;
                    self.cancel_flag = None; // Clear cancel flag
                    match result {
                        TaskResult::Success => {
                            self.status_text = "Task completed successfully!".to_string();
                            self.current_progress = 1.0;
                            
                            // Add success notification
                            let message = match self.view {
                                AppView::Packing => "Archive created successfully!",
                                AppView::Extracting => "Files extracted successfully!",
                                _ => "Operation completed successfully!",
                            };
                            self.toasts.success(message);
                        }
                        TaskResult::Error(err) => {
                            self.status_text = format!("Error: {}", err);
                            self.current_progress = 0.0;
                            
                            // Add error notification
                            self.toasts.error(format!("Operation failed: {}", err));
                        }
                        TaskResult::Cancelled => {
                            self.status_text = "Operation cancelled".to_string();
                            self.current_progress = 0.0;
                            
                            // Add info notification
                            self.toasts.info("Operation cancelled by user");
                        }
                    }
                }
                ToUi::Log(message) => {
                    // Add timestamp to log message (simple format for now)
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default();
                    let secs = now.as_secs() % 86400; // seconds in current day
                    let hours = secs / 3600;
                    let mins = (secs % 3600) / 60;
                    let secs = secs % 60;
                    let millis = now.subsec_millis();
                    
                    self.logs.push(format!("[{:02}:{:02}:{:02}.{:03}] {}", hours, mins, secs, millis, message));
                    
                    // Keep log size reasonable (max 1000 entries)
                    if self.logs.len() > 1000 {
                        self.logs.drain(0..100); // Remove oldest 100 entries
                    }
                }
            }
        }
        
        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            // Always show progress/status at the top if there's activity
            if self.is_busy || self.current_progress > 0.0 {
                ui.horizontal(|ui| {
                    ui.label(&self.status_text);
                    
                    // Show "New Task" button when task is complete
                    if !self.is_busy && self.current_progress >= 1.0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🆕 Start New Task").clicked() {
                                self.reset_to_welcome();
                            }
                        });
                    }
                });
                
                // Show progress bar
                ui.add(egui::ProgressBar::new(self.current_progress).show_percentage());
                
                // Show current file being processed
                if !self.current_file.is_empty() && self.is_busy {
                    ui.horizontal(|ui| {
                        ui.weak("Processing:");
                        ui.monospace(&self.current_file);
                    });
                }
                ui.separator();
                ui.add_space(10.0);
            }
            
            // Render view based on current state
            match self.view {
                AppView::Welcome => {
                    self.draw_welcome_view(ctx, ui);
                }
                AppView::Packing => {
                    // Handle packing view actions
                    if let Some(action) = draw_packing_view(
                        ctx,
                        ui,
                        &self.input_files,
                        &self.output_path,
                        &mut self.compression_format,
                        self.is_busy,
                    ) {
                        match action {
                            PackingAction::RemoveFile(idx) => {
                                if idx < self.input_files.len() {
                                    self.input_files.remove(idx);
                                    // If no files left, go back to welcome
                                    if self.input_files.is_empty() {
                                        self.view = AppView::Welcome;
                                    }
                                }
                            }
                            PackingAction::SelectOutput => {
                                // Determine file extension and filter based on compression format
                                let (extension, filter_name) = match self.compression_format.as_str() {
                                    "tar.gz" => ("tar.gz", "TAR GZ Archive"),
                                    "tar.zst" => ("tar.zst", "TAR ZST Archive"),
                                    "tar.xz" => ("tar.xz", "TAR XZ Archive"),
                                    "zip" => ("zip", "ZIP Archive"),
                                    _ => ("tar.gz", "Archive"),
                                };
                                
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_file_name(&format!("archive.{}", extension))
                                    .add_filter(filter_name, &[extension])
                                    .save_file() {
                                    self.output_path = Some(path);
                                }
                            }
                            PackingAction::AddMoreFiles => {
                                if let Some(files) = rfd::FileDialog::new().pick_files() {
                                    self.input_files.extend(files);
                                }
                            }
                            PackingAction::StartPacking => {
                                self.start_task();
                            }
                            PackingAction::ClearAll => {
                                self.input_files.clear();
                                self.output_path = None;
                                self.view = AppView::Welcome;
                                self.current_progress = 0.0;
                                self.status_text = "Ready".to_string();
                            }
                            PackingAction::Cancel => {
                                self.cancel_task();
                            }
                        }
                    }
                }
                AppView::Extracting => {
                    // Get the archive path for the view
                    let archive_path = self.input_files.first().cloned();
                    
                    // Handle extracting view actions
                    if let Some(action) = draw_extracting_view(
                        ctx,
                        ui,
                        &archive_path,
                        &self.output_path,
                        self.is_busy,
                    ) {
                        match action {
                            ExtractingAction::SelectOutputDir => {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    self.output_path = Some(path);
                                }
                            }
                            ExtractingAction::StartExtracting => {
                                self.start_task();
                            }
                            ExtractingAction::BrowseArchive => {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Archives", &["zip", "tar", "gz", "zst", "xz", "7z"])
                                    .pick_file() {
                                    self.input_files = vec![path];
                                }
                            }
                            ExtractingAction::Clear => {
                                self.input_files.clear();
                                self.output_path = None;
                                self.view = AppView::Welcome;
                                self.current_progress = 0.0;
                                self.status_text = "Ready".to_string();
                            }
                            ExtractingAction::Cancel => {
                                self.cancel_task();
                            }
                        }
                    }
                }
            }
        });
        
        // Log panel toggle at bottom
        egui::TopBottomPanel::bottom("log_panel_toggle").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_log_panel, "📋 Show Logs");
                if self.show_log_panel {
                    ui.separator();
                    if ui.button("Clear Logs").clicked() {
                        self.logs.clear();
                    }
                }
            });
        });
        
        // Log panel
        if self.show_log_panel {
            egui::TopBottomPanel::bottom("log_panel")
                .resizable(true)
                .default_height(200.0)
                .min_height(100.0)
                .max_height(400.0)
                .show(ctx, |ui| {
                    ui.heading("Logs");
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            
                            for log in &self.logs {
                                ui.monospace(log);
                            }
                        });
                });
        }
        
        // Show toast notifications
        self.toasts.show(ctx);
        
        // Request repaint if busy
        if self.is_busy {
            ctx.request_repaint();
        }
    }
}