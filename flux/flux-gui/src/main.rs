//! Flux GUI - A modern graphical interface for the Flux archiver

use eframe::egui;
use crossbeam_channel::{Receiver, Sender};
use std::{thread, path::PathBuf};

mod task;
use task::{TaskCommand, ToUi, ProgressUpdate, TaskResult};

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    /// No mode selected
    Idle,
    /// Pack files into archive
    Pack,
    /// Extract archive
    Extract,
}

/// Main application structure
pub struct FluxApp {
    /// Sender for commands to background thread
    task_sender: Sender<TaskCommand>,
    /// Receiver for messages from background thread
    ui_receiver: Receiver<ToUi>,
    /// Handle to the background thread
    task_handle: Option<thread::JoinHandle<()>>,
    /// Current progress (0.0 to 1.0)
    current_progress: f32,
    /// Status text to display
    status_text: String,
    /// Current mode
    mode: AppMode,
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
                                handle_pack_task(inputs, output, options, &ui_sender);
                            }
                            TaskCommand::Extract { archive, output_dir } => {
                                handle_extract_task(archive, output_dir, &ui_sender);
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
            task_sender,
            ui_receiver,
            task_handle: Some(task_handle),
            current_progress: 0.0,
            status_text: "Ready".to_string(),
            mode: AppMode::Idle,
            input_files: Vec::new(),
            output_path: None,
            compression_format: "tar.zst".to_string(),
            is_busy: false,
        }
    }
    
    /// Analyze dropped files to determine mode
    fn analyze_dropped_files(&mut self, files: Vec<PathBuf>) {
        if files.is_empty() {
            return;
        }
        
        // Check if it's a single archive file
        if files.len() == 1 {
            let file = &files[0];
            if let Some(ext) = file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                // Check for archive extensions
                if matches!(ext_str.as_str(), "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "br") {
                    self.mode = AppMode::Extract;
                    self.input_files = files;
                    return;
                }
            }
            // Also check for compound extensions like .tar.gz
            if let Some(name) = file.file_name() {
                let name_str = name.to_string_lossy().to_lowercase();
                if name_str.ends_with(".tar.gz") || name_str.ends_with(".tar.zst") || 
                   name_str.ends_with(".tar.xz") || name_str.ends_with(".tar.br") {
                    self.mode = AppMode::Extract;
                    self.input_files = files;
                    return;
                }
            }
        }
        
        // Multiple files or non-archive file - pack mode
        self.mode = AppMode::Pack;
        self.input_files = files;
    }
    
    /// Start the task based on current mode and inputs
    fn start_task(&mut self) {
        match self.mode {
            AppMode::Pack => {
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
            AppMode::Extract => {
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
            AppMode::Idle => {}
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
            ui.heading("Flux File Archiver");
            ui.separator();
            ui.add_space(10.0);
            
            // Drop zone
            let drop_zone_size = egui::vec2(ui.available_width(), 150.0);
            let (rect, response) = ui.allocate_exact_size(drop_zone_size, egui::Sense::click());
            
            // Draw drop zone
            let painter = ui.painter();
            painter.rect_filled(rect, 5.0, egui::Color32::from_gray(40));
            painter.rect_stroke(rect, 5.0, egui::Stroke::new(2.0, egui::Color32::from_gray(80)));
            
            // Drop zone content
            if self.input_files.is_empty() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop files here or click to browse",
                    egui::FontId::proportional(18.0),
                    egui::Color32::from_gray(150),
                );
            } else {
                // Show file list
                let mut child_ui = ui.child_ui(rect, egui::Layout::top_down(egui::Align::Min), None);
                egui::ScrollArea::vertical().show(&mut child_ui, |ui| {
                    for file in &self.input_files {
                        ui.horizontal(|ui| {
                            ui.label("📄");
                            ui.label(file.file_name().unwrap_or_default().to_string_lossy());
                        });
                    }
                });
            }
            
            // Handle click on drop zone
            if response.clicked() && !self.is_busy {
                if self.mode == AppMode::Extract || self.input_files.is_empty() {
                    // Browse for files
                    if let Some(files) = rfd::FileDialog::new().pick_files() {
                        self.analyze_dropped_files(files);
                    }
                } else {
                    // Add more files in pack mode
                    if let Some(files) = rfd::FileDialog::new().pick_files() {
                        self.input_files.extend(files);
                    }
                }
            }
            
            ui.add_space(20.0);
            
            // Mode display
            match self.mode {
                AppMode::Pack => {
                    ui.heading("📦 Pack Mode");
                    ui.label("Create an archive from selected files");
                    
                    // Compression format selection
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        ui.selectable_value(&mut self.compression_format, "tar.gz".to_string(), "tar.gz");
                        ui.selectable_value(&mut self.compression_format, "tar.zst".to_string(), "tar.zst");
                        ui.selectable_value(&mut self.compression_format, "tar.xz".to_string(), "tar.xz");
                        ui.selectable_value(&mut self.compression_format, "zip".to_string(), "zip");
                    });
                    
                    // Output selection
                    ui.horizontal(|ui| {
                        ui.label("Output:");
                        let output_text = self.output_path.as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Click to select output location...".to_string());
                        
                        if ui.button(&output_text).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(&format!("archive.{}", self.compression_format))
                                .add_filter("Archive", &[&self.compression_format])
                                .save_file() {
                                self.output_path = Some(path);
                            }
                        }
                    });
                }
                AppMode::Extract => {
                    ui.heading("📂 Extract Mode");
                    ui.label("Extract archive contents");
                    
                    // Output directory selection
                    ui.horizontal(|ui| {
                        ui.label("Extract to:");
                        let output_text = self.output_path.as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Click to select extraction directory...".to_string());
                        
                        if ui.button(&output_text).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.output_path = Some(path);
                            }
                        }
                    });
                }
                AppMode::Idle => {
                    ui.label("Drop files to get started");
                }
            }
            
            ui.add_space(20.0);
            
            // Progress and status
            if self.is_busy || self.current_progress > 0.0 {
                ui.label(&self.status_text);
                ui.add(egui::ProgressBar::new(self.current_progress).show_percentage());
            }
            
            ui.add_space(20.0);
            
            // Action buttons
            ui.horizontal(|ui| {
                let can_start = !self.is_busy && 
                    !self.input_files.is_empty() && 
                    self.output_path.is_some() &&
                    self.mode != AppMode::Idle;
                
                if ui.add_enabled(can_start, egui::Button::new("Start Task").min_size(egui::vec2(100.0, 30.0)))
                    .clicked() {
                    self.start_task();
                }
                
                if ui.button("Clear").clicked() && !self.is_busy {
                    self.input_files.clear();
                    self.output_path = None;
                    self.mode = AppMode::Idle;
                    self.current_progress = 0.0;
                    self.status_text = "Ready".to_string();
                }
            });
        });
        
        // Request repaint if busy
        if self.is_busy {
            ctx.request_repaint();
        }
    }
}

/// Handle pack task in background thread
fn handle_pack_task(
    inputs: Vec<std::path::PathBuf>,
    output: std::path::PathBuf,
    options: flux_lib::archive::PackOptions,
    ui_sender: &Sender<ToUi>,
) {
    // For simplicity, if multiple inputs, pack the first one
    // In a real implementation, you might want to create a temporary directory
    // and copy all files there first
    if let Some(input) = inputs.first() {
        // Use flux_lib::archive::pack_with_strategy function
        match flux_lib::archive::pack_with_strategy(input, &output, None, options) {
            Ok(_) => {
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
            }
            Err(e) => {
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            }
        }
    } else {
        let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("No input files".to_string())));
    }
}

/// Handle extract task in background thread
fn handle_extract_task(
    archive: std::path::PathBuf,
    output_dir: std::path::PathBuf,
    ui_sender: &Sender<ToUi>,
) {
    use flux_lib::archive::extractor::{Extractor, ExtractEntryOptions};
    
    // Create extractor
    let extractor = match flux_lib::archive::create_extractor(&archive) {
        Ok(ex) => ex,
        Err(e) => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            return;
        }
    };
    
    // Get entries to calculate total size
    let entries: Vec<_> = match extractor.entries(&archive) {
        Ok(entries) => {
            // Collect entries first to calculate total size
            entries.filter_map(|e| e.ok()).collect()
        }
        Err(e) => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            return;
        }
    };
    
    // Calculate total size
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let mut processed_size: u64 = 0;
    
    // Extract options
    let extract_options = ExtractEntryOptions {
        overwrite: true,
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
    };
    
    // Extract each entry
    for entry in &entries {
        // Send progress update
        let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: processed_size,
            total_bytes: total_size,
            current_file: entry.path.display().to_string(),
        }));
        
        // Extract the entry
        if let Err(e) = extractor.extract_entry(&archive, entry, &output_dir, extract_options.clone()) {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
                format!("Failed to extract {}: {}", entry.path.display(), e)
            )));
            return;
        }
        
        processed_size += entry.size;
    }
    
    // Send completion
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: total_size,
        total_bytes: total_size,
        current_file: "Extraction complete".to_string(),
    }));
    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

fn main() -> Result<(), eframe::Error> {
    // Set up eframe options with default values
    let options = eframe::NativeOptions::default();

    // Run the native app
    eframe::run_native(
        "Flux - File Archiver",
        options,
        Box::new(|cc| Ok(Box::new(FluxApp::new(cc)))),
    )
}