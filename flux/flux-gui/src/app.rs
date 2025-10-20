//! Main application implementation

use crate::state::{AppState, Mode, Status};
use crate::worker::{Command, Event, Worker};
use eframe::egui;
use flux_lib::archive::PackOptions;
use flux_lib::strategy::Algorithm;
use rfd::FileDialog;
use std::path::PathBuf;
use tracing::{error, info};

/// Main application
pub struct FluxApp {
    state: AppState,
    worker: Worker,
}

impl FluxApp {
    /// Create a new application instance
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::default(),
            worker: Worker::new(),
        }
    }
    
    /// Handle drag and drop
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // Check for dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        info!("File dropped: {:?}", path);
                        self.state.add_input(path.clone());
                    }
                }
            }
        });
    }
    
    /// Process worker events
    fn process_worker_events(&mut self) {
        while let Some(event) = self.worker.try_recv() {
            match event {
                Event::Progress { current, total, message } => {
                    let progress = current as f32 / total.max(1) as f32;
                    self.state.status = Status::Working { progress, message: message.clone() };
                    self.state.logs.push(format!("[PROGRESS] {}", message));
                }
                Event::Log(message) => {
                    self.state.logs.push(format!("[INFO] {}", message));
                }
                Event::Completed(message) => {
                    self.state.status = Status::Success(message.clone());
                    self.state.logs.push(format!("[SUCCESS] {}", message));
                }
                Event::Failed(message) => {
                    self.state.status = Status::Error(message.clone());
                    self.state.logs.push(format!("[ERROR] {}", message));
                }
            }
        }
    }
    
    /// Execute the current operation
    fn execute_operation(&mut self) {
        if !self.state.is_ready() {
            return;
        }
        
        match self.state.mode {
            Mode::Pack => {
                if let Some(output) = &self.state.output_path {
                    let options = PackOptions {
                        smart: self.state.compression.smart,
                        algorithm: Some(self.state.compression.algorithm.to_string()),
                        level: Some(self.state.compression.level),
                        threads: Some(self.state.compression.threads),
                        force_compress: false,
                        follow_symlinks: self.state.compression.follow_symlinks,
                    };
                    
                    let command = Command::Pack {
                        inputs: self.state.input_paths.clone(),
                        output: output.clone(),
                        options,
                    };
                    
                    if let Err(e) = self.worker.send(command) {
                        error!("Failed to send command to worker: {}", e);
                        self.state.status = Status::Error(format!("Failed to start operation: {}", e));
                    } else {
                        self.state.status = Status::Working {
                            progress: 0.0,
                            message: "Starting...".to_string(),
                        };
                    }
                }
            }
            Mode::Extract => {
                if let (Some(archive), Some(output_dir)) = 
                    (self.state.input_paths.first(), &self.state.output_path) {
                    let command = Command::Extract {
                        archive: archive.clone(),
                        output_dir: output_dir.clone(),
                    };
                    
                    if let Err(e) = self.worker.send(command) {
                        error!("Failed to send command to worker: {}", e);
                        self.state.status = Status::Error(format!("Failed to start operation: {}", e));
                    } else {
                        self.state.status = Status::Working {
                            progress: 0.0,
                            message: "Starting...".to_string(),
                        };
                    }
                }
            }
            Mode::Idle => {}
        }
    }
}

impl eframe::App for FluxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle drag and drop
        self.handle_dropped_files(ctx);
        
        // Process worker events
        self.process_worker_events();
        
        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.add_space(10.0);
            ui.heading("Flux File Archiver");
            ui.separator();
            ui.add_space(10.0);
            
            // Drop zone or file list
            let drop_zone_response = ui.allocate_response(
                ui.available_size() * egui::vec2(1.0, 0.3),
                egui::Sense::click_and_drag(),
            );
            
            let drop_zone_rect = drop_zone_response.rect;
            ui.painter().rect(
                drop_zone_rect,
                5.0,
                egui::Color32::from_gray(50),
                egui::Stroke::new(2.0, egui::Color32::from_gray(100)),
            );
            
            if self.state.input_paths.is_empty() {
                ui.put(drop_zone_rect, egui::Label::new(
                    egui::RichText::new("Drop files here or click to browse")
                        .size(20.0)
                        .color(egui::Color32::from_gray(150))
                ));
            } else {
                // Show file list
                egui::ScrollArea::vertical()
                    .max_height(drop_zone_rect.height())
                    .show(ui, |ui| {
                        for (i, path) in self.state.input_paths.clone().iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(path.display().to_string());
                                if ui.small_button("❌").clicked() {
                                    self.state.input_paths.remove(i);
                                    if self.state.input_paths.is_empty() {
                                        self.state.mode = Mode::Idle;
                                    }
                                }
                            });
                        }
                    });
            }
            
            if drop_zone_response.clicked() {
                // Open file dialog
                let files = if self.state.mode == Mode::Extract {
                    FileDialog::new()
                        .add_filter("Archives", &["zip", "tar", "gz", "zst", "xz", "7z", "br"])
                        .pick_files()
                } else {
                    FileDialog::new().pick_files()
                };
                
                if let Some(files) = files {
                    for file in files {
                        self.state.add_input(file);
                    }
                }
            }
            
            ui.add_space(20.0);
            
            // Mode selection
            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.selectable_value(&mut self.state.mode, Mode::Pack, "Pack");
                ui.selectable_value(&mut self.state.mode, Mode::Extract, "Extract");
            });
            
            ui.add_space(10.0);
            
            // Output selection
            ui.horizontal(|ui| {
                ui.label("Output:");
                let output_text = self.state.output_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Click to select...".to_string());
                    
                if ui.button(&output_text).clicked() {
                    match self.state.mode {
                        Mode::Pack => {
                            if let Some(file) = FileDialog::new()
                                .add_filter("Archives", &["tar.gz", "tar.zst", "tar.xz", "zip", "7z"])
                                .save_file() {
                                self.state.output_path = Some(file);
                            }
                        }
                        Mode::Extract => {
                            if let Some(dir) = FileDialog::new().pick_folder() {
                                self.state.output_path = Some(dir);
                            }
                        }
                        Mode::Idle => {}
                    }
                }
            });
            
            ui.add_space(10.0);
            
            // Compression options (for pack mode)
            if self.state.mode == Mode::Pack {
                ui.collapsing("Advanced Options", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Algorithm:");
                        ui.selectable_value(&mut self.state.compression.algorithm, Algorithm::Zstd, "Zstd");
                        ui.selectable_value(&mut self.state.compression.algorithm, Algorithm::Gzip, "Gzip");
                        ui.selectable_value(&mut self.state.compression.algorithm, Algorithm::Xz, "XZ");
                        ui.selectable_value(&mut self.state.compression.algorithm, Algorithm::Brotli, "Brotli");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Compression Level:");
                        ui.add(egui::Slider::new(&mut self.state.compression.level, 1..=9));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Threads:");
                        ui.add(egui::Slider::new(&mut self.state.compression.threads, 1..=16));
                    });
                    
                    ui.checkbox(&mut self.state.compression.smart, "Smart compression");
                    ui.checkbox(&mut self.state.compression.follow_symlinks, "Follow symlinks");
                });
            }
            
            ui.add_space(20.0);
            
            // Status display
            match &self.state.status {
                Status::Ready => {}
                Status::Working { progress, message } => {
                    ui.label(message);
                    ui.add(egui::ProgressBar::new(*progress));
                }
                Status::Success(message) => {
                    ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("✓ {}", message));
                }
                Status::Error(message) => {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), format!("✗ {}", message));
                }
            }
            
            ui.add_space(20.0);
            
            // Action buttons
            ui.horizontal(|ui| {
                let is_working = matches!(self.state.status, Status::Working { .. });
                
                if ui.add_enabled(!is_working && self.state.is_ready(), 
                    egui::Button::new(match self.state.mode {
                        Mode::Pack => "Pack Files",
                        Mode::Extract => "Extract Archive",
                        Mode::Idle => "Select Mode",
                    }).min_size(egui::vec2(120.0, 30.0))
                ).clicked() {
                    self.execute_operation();
                }
                
                if ui.add_enabled(is_working,
                    egui::Button::new("Cancel").min_size(egui::vec2(80.0, 30.0))
                ).clicked() {
                    let _ = self.worker.send(Command::Cancel);
                    self.state.status = Status::Error("Operation cancelled".to_string());
                }
                
                if ui.button("Clear").clicked() {
                    self.state.clear_inputs();
                    self.state.status = Status::Ready;
                }
                
                ui.separator();
                
                if ui.toggle_value(&mut self.state.show_logs, "Show Logs").clicked() {
                    // Toggle log window
                }
            });
            
            // Log window
            if self.state.show_logs {
                ui.add_space(20.0);
                ui.separator();
                ui.label("Operation Log:");
                
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in &self.state.logs {
                            ui.small(log);
                        }
                    });
            }
        });
        
        // Request repaint if working
        if matches!(self.state.status, Status::Working { .. }) {
            ctx.request_repaint();
        }
    }
}