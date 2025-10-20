//! Sync view for incremental backups

use eframe::egui;
use std::path::PathBuf;

/// Draw the sync/incremental backup view
pub fn draw_sync_view(
    _ctx: &egui::Context,
    ui: &mut egui::Ui,
    source_dir: &Option<PathBuf>,
    target_archive: &Option<PathBuf>,
    existing_manifest: &Option<PathBuf>,
    is_busy: bool,
) -> Option<SyncAction> {
    let mut action = None;
    
    ui.heading("🔄 Incremental Backup / Sync");
    ui.separator();
    ui.add_space(10.0);
    
    // Explanation
    ui.label("Create incremental backups that only include changed files since the last backup.");
    ui.add_space(10.0);
    
    // Source directory selection
    ui.horizontal(|ui| {
        ui.label("Source Directory:");
        ui.add_space(10.0);
        
        let source_text = source_dir.as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("No directory selected");
        
        ui.add(
            egui::TextEdit::singleline(&mut source_text.to_string())
                .desired_width(300.0)
                .interactive(false)
        );
        
        if ui.add_enabled(!is_busy, egui::Button::new("Browse...")).clicked() {
            action = Some(SyncAction::SelectSourceDir);
        }
    });
    
    ui.add_space(10.0);
    
    // Target archive selection
    ui.horizontal(|ui| {
        ui.label("Target Archive:");
        ui.add_space(10.0);
        
        let target_text = target_archive.as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("No archive selected");
        
        ui.add(
            egui::TextEdit::singleline(&mut target_text.to_string())
                .desired_width(300.0)
                .interactive(false)
        );
        
        if ui.add_enabled(!is_busy, egui::Button::new("Browse...")).clicked() {
            action = Some(SyncAction::SelectTargetArchive);
        }
    });
    
    ui.add_space(10.0);
    
    // Manifest detection
    if let Some(manifest_path) = existing_manifest {
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(90, 198, 90),
                "✓ Existing manifest found:"
            );
            ui.weak(manifest_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("manifest.json"));
        });
        ui.label("This will be an incremental sync - only changed files will be backed up.");
    } else if target_archive.is_some() {
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(255, 200, 100),
                "⚠ No existing manifest found"
            );
        });
        ui.label("This will be a full backup - all files will be included.");
    }
    
    ui.add_space(20.0);
    
    // Advanced options (collapsible)
    ui.collapsing("Advanced Options", |ui| {
        ui.checkbox(&mut true, "Follow symbolic links");
        ui.checkbox(&mut true, "Include file permissions");
        ui.checkbox(&mut true, "Generate deleted files list");
        
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.label("Compression level:");
            ui.add(egui::Slider::new(&mut 6, 1..=9));
        });
    });
    
    ui.add_space(20.0);
    
    // Action buttons
    ui.horizontal(|ui| {
        // Start sync button
        let can_start = !is_busy && source_dir.is_some() && target_archive.is_some();
        
        let button_text = if existing_manifest.is_some() {
            "🔄 Start Incremental Sync"
        } else {
            "📦 Start Full Backup"
        };
        
        if ui.add_enabled(
            can_start, 
            egui::Button::new(button_text)
                .min_size(egui::vec2(180.0, 35.0))
        ).clicked() {
            action = Some(SyncAction::StartSync);
        }
        
        // Cancel/Clear button
        let cancel_text = if is_busy { "Cancel" } else { "Clear" };
        if ui.button(cancel_text).clicked() {
            action = Some(if is_busy { SyncAction::Cancel } else { SyncAction::Clear });
        }
        
        ui.add_space(20.0);
        
        // View manifest button (if exists)
        if existing_manifest.is_some() {
            if ui.add_enabled(!is_busy, egui::Button::new("📋 View Manifest")).clicked() {
                action = Some(SyncAction::ViewManifest);
            }
        }
    });
    
    ui.add_space(20.0);
    
    // Info box
    egui::Frame::none()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(10.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label("ℹ️ How incremental backup works:");
            ui.add_space(5.0);
            ui.label("• First backup creates a full archive and manifest");
            ui.label("• Subsequent backups only include changed/new files");
            ui.label("• Each backup updates the manifest with current state");
            ui.label("• Deleted files are tracked in a separate list");
        });
    
    action
}

/// Actions that can be triggered from the sync view
#[derive(Debug, Clone)]
pub enum SyncAction {
    /// Select source directory
    SelectSourceDir,
    /// Select target archive
    SelectTargetArchive,
    /// Start the sync operation
    StartSync,
    /// View existing manifest
    ViewManifest,
    /// Clear selections
    Clear,
    /// Cancel operation
    Cancel,
}