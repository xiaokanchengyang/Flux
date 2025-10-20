//! Extracting view for the Flux GUI
//! This module handles the UI rendering for extracting archives

use eframe::egui;
use std::path::PathBuf;

/// Draw the extracting view UI
pub fn draw_extracting_view(
    _ctx: &egui::Context,
    ui: &mut egui::Ui,
    archive_path: &Option<PathBuf>,
    output_dir: &Option<PathBuf>,
    is_busy: bool,
) -> Option<ExtractingAction> {
    let mut action = None;
    
    ui.heading("📂 Extract Archive");
    ui.separator();
    ui.add_space(10.0);

    // Display the archive to extract prominently
    if let Some(archive) = archive_path {
        // Show archive name in a highlighted box
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("📦");
                ui.strong("Archive to extract:");
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(archive.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| archive.to_str().unwrap_or("Unknown")));
            });
        });
        
        ui.add_space(10.0);

        // Try to display file size
        if let Ok(metadata) = std::fs::metadata(archive) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.label(format!("{:.2} MB", size_mb));
            });
        }

        // Display archive type
        if let Some(ext) = archive.extension() {
            let archive_type = match ext.to_str().unwrap_or("").to_lowercase().as_str() {
                "zip" => "ZIP Archive",
                "gz" => {
                    if archive.to_str().unwrap_or("").ends_with(".tar.gz") {
                        "TAR.GZ Archive"
                    } else {
                        "GZIP Archive"
                    }
                },
                "zst" => {
                    if archive.to_str().unwrap_or("").ends_with(".tar.zst") {
                        "TAR.ZST Archive (Zstandard)"
                    } else {
                        "Zstandard Archive"
                    }
                },
                "xz" => {
                    if archive.to_str().unwrap_or("").ends_with(".tar.xz") {
                        "TAR.XZ Archive"
                    } else {
                        "XZ Archive"
                    }
                },
                "7z" => "7-Zip Archive",
                "tar" => "TAR Archive",
                _ => "Archive",
            };
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.label(archive_type);
            });
        }
    } else {
        ui.label("No archive selected");
    }

    ui.add_space(20.0);

    // Output directory selection
    ui.horizontal(|ui| {
        ui.label("Output directory:");
        
        // Display current output directory or placeholder text
        let output_text = output_dir.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "No directory selected".to_string());
        
        ui.label(&output_text);
        
        // Browse button to select output directory
        if ui.add_enabled(!is_busy, egui::Button::new("Browse...")).clicked() {
            action = Some(ExtractingAction::SelectOutputDir);
        }
    });

    // Show a note about extraction
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label("ℹ️");
        ui.weak("Files will be extracted to the selected directory");
    });

    ui.add_space(20.0);

    // Action buttons
    ui.horizontal(|ui| {
        // Start extraction button
        let can_start = !is_busy && archive_path.is_some() && output_dir.is_some();
        if ui.add_enabled(can_start, egui::Button::new("Start Extracting")
                .min_size(egui::vec2(140.0, 35.0)))
            .clicked() {
            action = Some(ExtractingAction::StartExtracting);
        }

        // Cancel button
        if ui.add_enabled(!is_busy, egui::Button::new("Cancel")
                .min_size(egui::vec2(80.0, 35.0)))
            .clicked() {
            action = Some(ExtractingAction::Clear);
        }
        
        ui.add_space(20.0);
        
        // Browse for different archive
        if ui.add_enabled(!is_busy, egui::Button::new("📁 Browse Archive"))
            .clicked() {
            action = Some(ExtractingAction::BrowseArchive);
        }
    });

    action
}

/// Actions that can be triggered from the extracting view
#[derive(Debug, Clone)]
pub enum ExtractingAction {
    /// Select output directory
    SelectOutputDir,
    /// Start the extraction operation
    StartExtracting,
    /// Browse for a different archive
    BrowseArchive,
    /// Clear current selection
    Clear,
}