//! UI rendering and update logic for the Flux GUI application

use eframe::egui;
use std::time::SystemTime;
use tracing::{info, Level};

use super::{AppView, FluxApp};
use crate::components::{set_theme_in_context, DropZone, FluxButton};
use crate::layout::NavItem;
use crate::task::{TaskResult, ToUi};
use crate::views::{
    draw_browser_view, draw_extracting_view, draw_packing_view_modern, draw_sync_view,
    BrowserAction, ExtractingAction, PackingAction, SyncAction,
};

impl FluxApp {
    /// Export logs to a file
    fn export_logs(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let date = format!("Timestamp: {}", secs);
        writeln!(file, "Flux GUI Logs - Exported at {}", date)?;
        writeln!(file, "=")?;
        writeln!(file)?;

        for (level, log) in &self.logs {
            writeln!(file, "[{}] {}", level, log)?;
        }

        Ok(())
    }

    /// Draw the welcome view
    pub(super) fn draw_welcome_view(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // Stylish header with gradient-like effect
            ui.heading(
                egui::RichText::new("Flux Archive Manager")
                    .size(32.0)
                    .color(self.theme.colors.primary),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Modern, fast, and intelligent file compression")
                    .size(16.0)
                    .color(self.theme.colors.text_weak),
            );
            ui.add_space(40.0);

            // Modern drop zone
            let drop_response = ui.add(
                DropZone::new("main_drop")
                    .text("Drop files or folders here")
                    .subtext("or click to browse"),
            );

            if drop_response.clicked() {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    self.analyze_dropped_files(files);
                }
            }

            ui.add_space(40.0);

            // Quick action buttons using FluxButton
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 530.0) / 2.0); // Center the buttons

                // Create Archive button
                if ui
                    .add(
                        FluxButton::new("Create Archive")
                            .primary()
                            .icon(egui_phosphor::regular::PACKAGE)
                            .min_size(egui::vec2(150.0, 40.0)),
                    )
                    .clicked()
                {
                    if let Some(files) = rfd::FileDialog::new().pick_files() {
                        self.analyze_dropped_files(files);
                    }
                }

                ui.add_space(20.0);

                // Extract Archive button
                if ui
                    .add(
                        FluxButton::new("Extract Archive")
                            .icon(egui_phosphor::regular::FOLDER_OPEN)
                            .min_size(egui::vec2(150.0, 40.0)),
                    )
                    .clicked()
                {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Archives", &["zip", "tar", "gz", "zst", "xz", "7z", "br"])
                        .pick_file()
                    {
                        self.analyze_dropped_files(vec![file]);
                    }
                }

                ui.add_space(20.0);

                // Incremental Backup button
                if ui
                    .add(
                        FluxButton::new("Incremental Sync")
                            .icon(egui_phosphor::regular::ARROW_SQUARE_OUT)
                            .min_size(egui::vec2(150.0, 40.0)),
                    )
                    .clicked()
                {
                    self.view = AppView::Syncing;
                }
            });

            ui.add_space(40.0);

            // Feature highlights
            egui::Frame::none()
                .fill(self.theme.colors.panel_bg)
                .rounding(self.theme.rounding)
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.columns(3, |columns| {
                        columns[0].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("⚡").size(32.0));
                            ui.label(egui::RichText::new("Lightning Fast").size(16.0).strong());
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(
                                    "Multi-threaded compression\nwith real-time progress",
                                )
                                .size(12.0)
                                .color(self.theme.colors.text_weak),
                            );
                        });

                        columns[1].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("🎯").size(32.0));
                            ui.label(egui::RichText::new("Smart Selection").size(16.0).strong());
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(
                                    "Automatic format detection\nand optimal compression",
                                )
                                .size(12.0)
                                .color(self.theme.colors.text_weak),
                            );
                        });

                        columns[2].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("🔒").size(32.0));
                            ui.label(egui::RichText::new("Secure & Reliable").size(16.0).strong());
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(
                                    "Safe extraction with\npath traversal protection",
                                )
                                .size(12.0)
                                .color(self.theme.colors.text_weak),
                            );
                        });
                    });
                });

            ui.add_space(30.0);

            // Quick tips section
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💡").size(16.0));
                ui.label(egui::RichText::new("Quick Tips:").size(14.0).strong());
            });

            ui.add_space(5.0);

            // Tips in a subtle style
            let tip_style = |text: &str| {
                egui::RichText::new(text)
                    .size(12.0)
                    .color(self.theme.colors.text_weak)
            };
            ui.indent("tips", |ui| {
                ui.label(tip_style(
                    "• Drag multiple files/folders to create a combined archive",
                ));
                ui.label(tip_style(
                    "• Drop an archive file to extract it automatically",
                ));
                ui.label(tip_style("• Use Incremental Sync for efficient backups"));
                ui.label(tip_style(
                    "• Check the logs panel for detailed operation info",
                ));
            });
        });
    }

    /// Process incoming messages and update UI state
    pub(super) fn process_messages(&mut self) {
        // Process log messages from tracing
        if let Some(log_receiver) = &self.log_receiver {
            while let Ok((level, log_msg)) = log_receiver.try_recv() {
                // Add timestamp to log message
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = now.as_secs() % 86400; // seconds in current day
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                let millis = now.subsec_millis();

                let timestamped_msg = format!(
                    "[{:02}:{:02}:{:02}.{:03}] {}",
                    hours, mins, secs, millis, log_msg
                );
                self.logs.push((level, timestamped_msg));

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
                    self.current_progress =
                        update.processed_bytes as f32 / update.total_bytes.max(1) as f32;
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
                            self.status_text = format!(
                                "{:.1} / {:.1} MB ({}%) - {} - ETA: {}",
                                processed_mb, total_mb, percent, speed_str, eta_str
                            );
                        } else {
                            self.status_text = format!(
                                "{:.1} / {:.1} MB ({}%) - {}",
                                processed_mb, total_mb, percent, speed_str
                            );
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
                            self.toasts.error("Operation failed - click for details");

                            // Store error details for modal
                            let summary = match self.view {
                                AppView::Packing => "Failed to create archive",
                                AppView::Extracting => "Failed to extract files",
                                _ => "Operation failed",
                            };

                            // Parse error for better formatting
                            let details = format!("Error Details:\n\n{}\n\nPlease check:\n• File permissions\n• Available disk space\n• File paths are correct\n• Archive format is supported", err);

                            self.error_details = Some((summary.to_string(), details));
                            self.show_error_modal = true;
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

                    let timestamped_msg = format!(
                        "[{:02}:{:02}:{:02}.{:03}] {}",
                        hours, mins, secs, millis, message
                    );
                    // For messages from ToUi::Log, default to INFO level
                    self.logs.push((tracing::Level::INFO, timestamped_msg));

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
        // Apply theme and set it in context for components
        self.theme.apply(ctx);
        set_theme_in_context(ctx, &self.theme);

        // Update window title based on current state
        let title = match (self.view, self.is_busy) {
            (AppView::Packing, true) => "Flux - Packing...",
            (AppView::Packing, false) => "Flux - Pack Files",
            (AppView::Extracting, true) => "Flux - Extracting...",
            (AppView::Extracting, false) => "Flux - Extract Archive",
            (AppView::Syncing, true) => "Flux - Syncing...",
            (AppView::Syncing, false) => "Flux - Incremental Backup",
            (AppView::Browsing, _) => "Flux - Archive Browser",
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

        // Navigation items
        let nav_items = NavItem::default_items();

        // Draw sidebar
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(self.sidebar.current_width())
            .show(ctx, |ui| {
                self.sidebar
                    .show(ctx, ui, &mut self.view, &self.theme, &nav_items);
            });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Add some padding
            ui.add_space(20.0);

            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(20.0, 0.0))
                .show(ui, |ui| {
                    // Render view based on current state
                    match self.view {
                        AppView::Welcome => {
                            self.draw_welcome_view(ctx, ui);
                        }
                        AppView::Packing => {
                            // Handle packing view actions
                            let mut view_ctx = crate::views::packing_view_modern::PackingViewContext {
                                ctx,
                                input_files: &self.input_files,
                                output_path: &self.output_path,
                                compression_format: &mut self.compression_format,
                                is_busy: self.is_busy,
                                theme: &self.theme,
                                current_progress: self.current_progress,
                                status_text: &self.status_text,
                            };
                            if let Some(action) = draw_packing_view_modern(
                                ui,
                                &mut view_ctx,
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
                                        let (extension, filter_name) =
                                            match self.compression_format.as_str() {
                                                "tar.gz" => ("tar.gz", "TAR GZ Archive"),
                                                "tar.zst" => ("tar.zst", "TAR ZST Archive"),
                                                "tar.xz" => ("tar.xz", "TAR XZ Archive"),
                                                "zip" => ("zip", "ZIP Archive"),
                                                _ => ("tar.gz", "Archive"),
                                            };

                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_file_name(format!("archive.{}", extension))
                                            .add_filter(filter_name, &[extension])
                                            .save_file()
                                        {
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
                                &mut self.extract_hoist,
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
                                            .add_filter(
                                                "Archives",
                                                &["zip", "tar", "gz", "zst", "xz", "7z"],
                                            )
                                            .pick_file()
                                        {
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
                                    ExtractingAction::OpenBrowser => {
                                        if let Some(archive) = archive_path {
                                            if let Err(e) = self.open_archive_browser(archive) {
                                                self.toasts.error(format!(
                                                    "Failed to open browser: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        AppView::Syncing => {
                            // Handle sync view actions
                            if let Some(action) = draw_sync_view(
                                ctx,
                                ui,
                                &self.sync_source_dir,
                                &self.sync_target_archive,
                                &self.sync_manifest_path,
                                self.is_busy,
                            ) {
                                match action {
                                    SyncAction::SelectSourceDir => {
                                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                            self.sync_source_dir = Some(dir);
                                        }
                                    }
                                    SyncAction::SelectTargetArchive => {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .set_file_name("backup.tar.zst")
                                            .add_filter(
                                                "Tar Archives",
                                                &["tar", "tar.gz", "tar.zst", "tar.xz"],
                                            )
                                            .save_file()
                                        {
                                            self.sync_target_archive = Some(file.clone());
                                            // Check for existing manifest
                                            let manifest_path =
                                                file.with_extension("manifest.json");
                                            if manifest_path.exists() {
                                                self.sync_manifest_path = Some(manifest_path);
                                            } else {
                                                self.sync_manifest_path = None;
                                            }
                                        }
                                    }
                                    SyncAction::StartSync => {
                                        self.start_sync_task();
                                    }
                                    SyncAction::ViewManifest => {
                                        if let Some(manifest_path) = &self.sync_manifest_path {
                                            // Show manifest details (could open in external editor or show in modal)
                                            self.toasts.info(format!(
                                                "Manifest at: {}",
                                                manifest_path.display()
                                            ));
                                        }
                                    }
                                    SyncAction::Clear => {
                                        self.sync_source_dir = None;
                                        self.sync_target_archive = None;
                                        self.sync_manifest_path = None;
                                        self.view = AppView::Welcome;
                                        self.current_progress = 0.0;
                                        self.status_text = "Ready".to_string();
                                    }
                                    SyncAction::Cancel => {
                                        self.cancel_task();
                                    }
                                }
                            }
                        }
                        AppView::Browsing => {
                            // Handle browser view
                            if let Some(browser_state) = &mut self.browser_state {
                                if let Some(action) =
                                    draw_browser_view(ctx, ui, browser_state, &self.theme)
                                {
                                    match action {
                                        BrowserAction::ExtractSelected(dest) => {
                                            let selected_entries =
                                                browser_state.get_selected_entries();
                                            let archive_path = browser_state.archive_path.clone();
                                            self.extract_selected_entries(
                                                selected_entries,
                                                archive_path,
                                                dest,
                                            );
                                        }
                                        BrowserAction::ExtractAll(dest) => {
                                            // Switch to extracting view with the archive
                                            self.view = AppView::Extracting;
                                            self.input_files =
                                                vec![browser_state.archive_path.clone()];
                                            self.output_path = Some(dest);
                                            self.browser_state = None;
                                            self.start_task();
                                        }
                                        BrowserAction::Close => {
                                            // Return to welcome view
                                            self.view = AppView::Welcome;
                                            self.browser_state = None;
                                            self.current_progress = 0.0;
                                            self.status_text = "Ready".to_string();
                                        }
                                        BrowserAction::ChooseDestination => {
                                            if let Some(dir) = rfd::FileDialog::new().pick_folder()
                                            {
                                                // Check if we're extracting all or selected
                                                if browser_state.selected.is_empty() {
                                                    // Extract all
                                                    self.view = AppView::Extracting;
                                                    self.input_files =
                                                        vec![browser_state.archive_path.clone()];
                                                    self.output_path = Some(dir);
                                                    self.browser_state = None;
                                                    self.start_task();
                                                } else {
                                                    // Extract selected entries
                                                    let selected_entries =
                                                        browser_state.get_selected_entries();
                                                    let archive_path =
                                                        browser_state.archive_path.clone();
                                                    self.extract_selected_entries(
                                                        selected_entries,
                                                        archive_path,
                                                        dir,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // No browser state, return to welcome
                                self.view = AppView::Welcome;
                            }
                        }
                    }
                });
        });

        // Status bar with log panel toggle at bottom
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Log panel toggle
                    ui.checkbox(&mut self.show_log_panel, "📋 Show Logs");

                    // Log count with level breakdown
                    if !self.logs.is_empty() {
                        ui.separator();

                        let error_count = self
                            .logs
                            .iter()
                            .filter(|(l, _)| matches!(l, &Level::ERROR))
                            .count();
                        let warn_count = self
                            .logs
                            .iter()
                            .filter(|(l, _)| matches!(l, &Level::WARN))
                            .count();
                        let _info_count = self
                            .logs
                            .iter()
                            .filter(|(l, _)| matches!(l, &Level::INFO))
                            .count();

                        ui.weak(format!("Total: {}", self.logs.len()));

                        if error_count > 0 {
                            ui.separator();
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 100, 100),
                                format!("Errors: {}", error_count),
                            );
                        }

                        if warn_count > 0 {
                            ui.separator();
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 200, 100),
                                format!("Warnings: {}", warn_count),
                            );
                        }

                        if self.show_log_panel {
                            ui.separator();
                            if ui.button("🗑 Clear All").clicked() {
                                self.logs.clear();
                            }
                        }
                    }

                    // Right-aligned status indicators
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Theme toggle button
                        let theme_icon = match self.theme.mode {
                            crate::theme::ThemeMode::Light => "🌞",
                            crate::theme::ThemeMode::Dark => "🌙",
                        };
                        if ui
                            .button(theme_icon)
                            .on_hover_text("Toggle theme")
                            .clicked()
                        {
                            self.theme.toggle();
                        }

                        ui.separator();

                        if self.is_busy {
                            ui.spinner();
                            ui.label("Working...");
                        } else {
                            ui.weak("Ready");
                        }
                    });
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
                    // Header with controls
                    ui.horizontal(|ui| {
                        ui.heading("📋 Logs");

                        ui.separator();

                        // Search box
                        ui.label("🔍");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.log_filter)
                                .desired_width(200.0)
                                .hint_text("Filter logs..."),
                        );

                        ui.separator();

                        // Level filter buttons
                        ui.label("Level:");

                        let all_selected = self.log_level_filter.is_none();
                        if ui.selectable_label(all_selected, "All").clicked() {
                            self.log_level_filter = None;
                        }

                        let error_selected = matches!(self.log_level_filter, Some(Level::ERROR));
                        if ui
                            .selectable_label(
                                error_selected,
                                egui::RichText::new("Error")
                                    .color(egui::Color32::from_rgb(255, 100, 100)),
                            )
                            .clicked()
                        {
                            self.log_level_filter = Some(Level::ERROR);
                        }

                        let warn_selected = matches!(self.log_level_filter, Some(Level::WARN));
                        if ui
                            .selectable_label(
                                warn_selected,
                                egui::RichText::new("Warn")
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            )
                            .clicked()
                        {
                            self.log_level_filter = Some(Level::WARN);
                        }

                        let info_selected = matches!(self.log_level_filter, Some(Level::INFO));
                        if ui.selectable_label(info_selected, "Info").clicked() {
                            self.log_level_filter = Some(Level::INFO);
                        }

                        let debug_selected = matches!(self.log_level_filter, Some(Level::DEBUG));
                        if ui
                            .selectable_label(
                                debug_selected,
                                egui::RichText::new("Debug")
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                            )
                            .clicked()
                        {
                            self.log_level_filter = Some(Level::DEBUG);
                        }

                        // Right-aligned export button
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("💾 Export").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_file_name("flux_logs.txt")
                                    .add_filter("Text files", &["txt"])
                                    .save_file()
                                {
                                    if let Err(e) = self.export_logs(&path) {
                                        self.toasts.error(format!("Failed to export logs: {}", e));
                                    } else {
                                        self.toasts.success("Logs exported successfully");
                                    }
                                }
                            }
                        });
                    });

                    ui.separator();

                    // Log content area
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                            let filter_lower = self.log_filter.to_lowercase();
                            let filtered_logs: Vec<_> = self
                                .logs
                                .iter()
                                .filter(|(level, log)| {
                                    // Apply level filter
                                    if let Some(filter_level) = &self.log_level_filter {
                                        if level != filter_level {
                                            return false;
                                        }
                                    }

                                    // Apply text filter
                                    if !filter_lower.is_empty()
                                        && !log.to_lowercase().contains(&filter_lower)
                                    {
                                        return false;
                                    }

                                    true
                                })
                                .collect();

                            if filtered_logs.is_empty() && !self.logs.is_empty() {
                                ui.weak("No logs match the current filter");
                            } else {
                                for (level, log) in filtered_logs {
                                    // Color code based on log level
                                    let color = match *level {
                                        tracing::Level::ERROR => {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        }
                                        tracing::Level::WARN => {
                                            egui::Color32::from_rgb(255, 200, 100)
                                        }
                                        tracing::Level::INFO => ui.style().visuals.text_color(),
                                        tracing::Level::DEBUG => {
                                            egui::Color32::from_rgb(150, 150, 150)
                                        }
                                        tracing::Level::TRACE => {
                                            egui::Color32::from_rgb(100, 100, 100)
                                        }
                                    };

                                    ui.colored_label(color, egui::RichText::new(log).monospace());
                                }
                            }
                        });
                });
        }

        // Show toast notifications
        self.toasts.show(ctx);

        // Error modal dialog
        if self.show_error_modal {
            let error_details_clone = self.error_details.clone();
            if let Some((summary, details)) = error_details_clone {
                let mut close_modal = false;

                egui::Window::new("❌ Error Details")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            // Error summary
                            ui.heading(&summary);
                            ui.add_space(10.0);

                            // Error details in a scrollable area
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut details.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(400.0)
                                            .desired_rows(10)
                                            .interactive(false),
                                    );
                                });

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // Action buttons
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| {
                                        o.copied_text = format!("{}\n\n{}", summary, details)
                                    });
                                    self.toasts.info("Error details copied to clipboard");
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Close").clicked() {
                                            close_modal = true;
                                        }
                                    },
                                );
                            });
                        });
                    });

                if close_modal {
                    self.show_error_modal = false;
                    self.error_details = None;
                }
            }
        }

        // About dialog
        if self.show_about_dialog {
            let mut close_dialog = false;

            egui::Window::new("About Flux")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        // App icon
                        ui.add_space(10.0);
                        ui.heading("🗜️ Flux");
                        ui.add_space(5.0);

                        // Version
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(10.0);

                        // Description
                        ui.label("A fast, modern file archiver with GUI");
                        ui.add_space(20.0);

                        // Features
                        ui.label("Features:");
                        ui.indent("features", |ui| {
                            ui.label("• Multiple archive formats (ZIP, TAR, 7Z)");
                            ui.label("• Smart compression selection");
                            ui.label("• Incremental backups");
                            ui.label("• Secure extraction");
                            ui.label("• Cross-platform support");
                        });

                        ui.add_space(20.0);

                        // Links
                        ui.horizontal(|ui| {
                            ui.hyperlink_to("GitHub", "https://github.com/your-username/flux");
                            ui.label("|");
                            ui.hyperlink_to(
                                "Documentation",
                                "https://github.com/your-username/flux/wiki",
                            );
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Close button
                        if ui.button("Close").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if close_dialog {
                self.show_about_dialog = false;
            }
        }

        // Request repaint if busy
        if self.is_busy {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.save_persistence(storage);
    }
}
