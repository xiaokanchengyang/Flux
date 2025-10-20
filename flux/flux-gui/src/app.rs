//! Flux GUI Application structure and logic

use eframe::egui;
use crossbeam_channel::{Receiver, Sender};
use std::{thread, path::PathBuf};

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
    /// Files to process
    input_files: Vec<PathBuf>,
    /// Output path
    output_path: Option<PathBuf>,
    /// Selected compression format for packing
    compression_format: String,
    /// Is task running
    is_busy: bool,
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
                            TaskCommand::Pack { inputs, output, options } => {
                                crate::handle_pack_task(inputs, output, options, &ui_sender);
                            }
                            TaskCommand::Extract { archive, output_dir } => {
                                crate::handle_extract_task(archive, output_dir, &ui_sender);
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
            input_files: Vec::new(),
            output_path: None,
            compression_format: "tar.zst".to_string(),
            is_busy: false,
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
            
            // Check for common archive extensions
            if let Some(ext) = file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "br") {
                    // Switch to extracting view
                    self.view = AppView::Extracting;
                    self.input_files = files;
                    return;
                }
            }
            
            // Also check for compound extensions like .tar.gz
            if let Some(name) = file.file_name() {
                let name_str = name.to_string_lossy().to_lowercase();
                if name_str.ends_with(".tar.gz") || name_str.ends_with(".tar.zst") || 
                   name_str.ends_with(".tar.xz") || name_str.ends_with(".tar.br") {
                    // Switch to extracting view
                    self.view = AppView::Extracting;
                    self.input_files = files;
                    return;
                }
            }
        }
        
        // Multiple files, single non-archive file, or directories - switch to packing view
        self.view = AppView::Packing;
        self.input_files = files;
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
    
    /// Start the task based on current view and inputs
    fn start_task(&mut self) {
        match self.view {
            AppView::Packing => {
                if let Some(output) = &self.output_path {
                    let options = flux_lib::archive::PackOptions {
                        smart: true,
                        algorithm: None,
                        level: None,
                        threads: None,
                        force_compress: false,
                        follow_symlinks: false,
                    };
                    
                    let command = TaskCommand::Pack {
                        inputs: self.input_files.clone(),
                        output: output.clone(),
                        options,
                    };
                    
                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting pack operation...".to_string();
                    }
                }
            }
            AppView::Extracting => {
                if let (Some(archive), Some(output_dir)) = (self.input_files.first(), &self.output_path) {
                    let command = TaskCommand::Extract {
                        archive: archive.clone(),
                        output_dir: output_dir.clone(),
                    };
                    
                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting extraction...".to_string();
                    }
                }
            }
            AppView::Welcome => {}
        }
    }
}

impl eframe::App for FluxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    self.status_text = format!("Processing: {}", update.current_file);
                }
                ToUi::Finished(result) => {
                    self.is_busy = false;
                    match result {
                        TaskResult::Success => {
                            self.status_text = "Task completed successfully!".to_string();
                            self.current_progress = 1.0;
                        }
                        TaskResult::Error(err) => {
                            self.status_text = format!("Error: {}", err);
                            self.current_progress = 0.0;
                        }
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
                });
                ui.add(egui::ProgressBar::new(self.current_progress).show_percentage());
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
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_file_name(&format!("archive.{}", self.compression_format))
                                    .add_filter("Archive", &[&self.compression_format])
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
                        }
                    }
                }
            }
        });
        
        // Request repaint if busy
        if self.is_busy {
            ctx.request_repaint();
        }
    }
}