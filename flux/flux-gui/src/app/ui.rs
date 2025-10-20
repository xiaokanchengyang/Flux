//! UI rendering and update logic for the Flux GUI application

use eframe::egui;
use std::time::SystemTime;
use tracing::info;

use crate::task::{ToUi, TaskResult};
use crate::views::{draw_packing_view, PackingAction, draw_extracting_view, ExtractingAction};
use super::{FluxApp, AppView};

impl FluxApp {
    /// Draw the welcome view
    pub(super) fn draw_welcome_view(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
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
    
    /// Process incoming messages and update UI state
    pub(super) fn process_messages(&mut self) {
        // Process log messages from tracing
        if let Some(log_receiver) = &self.log_receiver {
            while let Ok(log_msg) = log_receiver.try_recv() {
                // Add timestamp to log message
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = now.as_secs() % 86400; // seconds in current day
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                let millis = now.subsec_millis();
                
                self.logs.push(format!("[{:02}:{:02}:{:02}.{:03}] {}", hours, mins, secs, millis, log_msg));
                
                // Keep log size reasonable (max 1000 entries)
                if self.logs.len() > 1000 {
                    self.logs.drain(0..100); // Remove oldest 100 entries
                }
            }
        }
        
        // Process all pending UI messages
        while let Ok(msg) = self.ui_receiver.try_recv() {
            match msg {
                ToUi::Progress(update) => {
                    self.current_progress = update.processed_bytes as f32 / update.total_bytes.max(1) as f32;
                    self.current_file = update.current_file.clone();
                    self.processed_bytes = update.processed_bytes;
                    self.total_bytes = update.total_bytes;
                    self.current_speed_bps = update.speed_bps;
                    self.eta_seconds = update.eta_seconds;
                    
                    // Format status text with size information
                    let processed_mb = update.processed_bytes as f64 / (1024.0 * 1024.0);
                    let total_mb = update.total_bytes as f64 / (1024.0 * 1024.0);
                    
                    if update.total_bytes > 0 {
                        let percent = (self.current_progress * 100.0) as u32;
                        let speed_str = crate::progress_tracker::format_speed(update.speed_bps);
                        
                        if let Some(eta_seconds) = update.eta_seconds {
                            let eta_str = crate::progress_tracker::format_duration(eta_seconds);
                            self.status_text = format!("{:.1} / {:.1} MB ({}%) - {} - ETA: {}", 
                                processed_mb, total_mb, percent, speed_str, eta_str);
                        } else {
                            self.status_text = format!("{:.1} / {:.1} MB ({}%) - {}", 
                                processed_mb, total_mb, percent, speed_str);
                        }
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
                            info!("Task completed successfully");
                            
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
                            info!("Task failed: {}", err);
                            
                            // Add error notification
                            self.toasts.error(format!("Operation failed: {}", err));
                        }
                        TaskResult::Cancelled => {
                            self.status_text = "Operation cancelled".to_string();
                            self.current_progress = 0.0;
                            info!("Task cancelled");
                            
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
        
        // Process incoming messages
        self.process_messages();
        
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
                    
                    // Show speed and ETA on a separate line
                    if self.current_speed_bps > 0.0 {
                        ui.horizontal(|ui| {
                            ui.weak("Speed:");
                            ui.label(crate::progress_tracker::format_speed(self.current_speed_bps));
                            
                            if let Some(eta) = self.eta_seconds {
                                ui.separator();
                                ui.weak("ETA:");
                                ui.label(crate::progress_tracker::format_duration(eta));
                            }
                        });
                    }
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