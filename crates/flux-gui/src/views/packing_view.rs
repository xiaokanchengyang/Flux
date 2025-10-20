//! Packing view for the Flux GUI
//! This module handles the UI rendering for packing multiple files into an archive

use eframe::egui;
use std::path::PathBuf;

/// Draw the packing view UI
pub fn draw_packing_view(
    _ctx: &egui::Context,
    ui: &mut egui::Ui,
    input_files: &[PathBuf],
    output_path: &Option<PathBuf>,
    compression_format: &mut String,
    is_busy: bool,
) -> Option<PackingAction> {
    let mut action = None;

    ui.heading("📦 Pack Files into Archive");
    ui.separator();
    ui.add_space(10.0);

    // Show the list of files to be packed
    ui.label(format!("Files to pack: {}", input_files.len()));

    // Create a scrollable area for the file list
    let available_height = ui.available_height() * 0.4;
    let mut files_to_remove = Vec::new();

    egui::ScrollArea::vertical()
        .max_height(available_height)
        .show(ui, |ui| {
            for (idx, file) in input_files.iter().enumerate() {
                ui.horizontal(|ui| {
                    // File icon based on whether it's a file or directory
                    let icon = if file.is_dir() { "📁" } else { "📄" };
                    ui.label(icon);

                    // Display the file name or full path
                    let display_name = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_else(|| file.to_str().unwrap_or("Unknown"));
                    ui.label(display_name);

                    // Add remove button if not busy
                    if !is_busy {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("❌").clicked() {
                                files_to_remove.push(idx);
                            }
                        });
                    }
                });
            }
        });

    // Handle file removals
    if !files_to_remove.is_empty() {
        action = Some(PackingAction::RemoveFile(files_to_remove[0]));
    }

    ui.add_space(20.0);

    // Compression format selection using ComboBox
    ui.horizontal(|ui| {
        ui.label("Archive format:");
        ui.add_enabled_ui(!is_busy, |ui| {
            egui::ComboBox::from_label("")
                .selected_text(compression_format.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(compression_format, "tar.gz".to_string(), "tar.gz");
                    ui.selectable_value(
                        compression_format,
                        "tar.zst".to_string(),
                        "tar.zst (recommended)",
                    );
                    ui.selectable_value(compression_format, "tar.xz".to_string(), "tar.xz");
                    ui.selectable_value(compression_format, "zip".to_string(), "zip");
                });
        });
    });

    ui.add_space(10.0);

    // Output path selection with visual indicator
    ui.horizontal(|ui| {
        ui.label("Output path:");

        // Display current output path or placeholder text
        let has_output = output_path.is_some();
        let output_text = output_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "⚠️ No output path selected".to_string());

        if has_output {
            ui.label(&output_text);
        } else {
            ui.colored_label(egui::Color32::from_rgb(255, 152, 0), &output_text);
        }

        // Browse button to select output location
        if ui
            .add_enabled(!is_busy, egui::Button::new("Browse..."))
            .clicked()
        {
            action = Some(PackingAction::SelectOutput);
        }
    });

    // Show helpful tip if output not selected
    if output_path.is_none() && !is_busy {
        ui.add_space(5.0);
        ui.indent("output_tip", |ui| {
            ui.label(
                egui::RichText::new("💡 Select where to save your archive")
                    .size(12.0)
                    .color(ui.style().visuals.weak_text_color()),
            );
        });
    }

    ui.add_space(20.0);

    // Action buttons
    ui.horizontal(|ui| {
        // Start packing button
        let can_start = !is_busy && !input_files.is_empty() && output_path.is_some();
        if ui
            .add_enabled(
                can_start,
                egui::Button::new("Start Packing").min_size(egui::vec2(120.0, 35.0)),
            )
            .clicked()
        {
            action = Some(PackingAction::StartPacking);
        }

        // Cancel button - enabled when busy (cancels task) or when not busy (clears selection)
        let cancel_text = if is_busy { "Cancel Task" } else { "Clear" };
        if ui
            .add(egui::Button::new(cancel_text).min_size(egui::vec2(80.0, 35.0)))
            .clicked()
        {
            action = Some(if is_busy {
                PackingAction::Cancel
            } else {
                PackingAction::ClearAll
            });
        }

        ui.add_space(20.0);

        // Add more files button
        if ui
            .add_enabled(!is_busy, egui::Button::new("➕ Add More Files"))
            .clicked()
        {
            action = Some(PackingAction::AddMoreFiles);
        }
    });

    action
}

/// Actions that can be triggered from the packing view
#[derive(Debug, Clone)]
pub enum PackingAction {
    /// Remove a file at the given index
    RemoveFile(usize),
    /// Select output location
    SelectOutput,
    /// Add more files to pack
    AddMoreFiles,
    /// Start the packing operation
    StartPacking,
    /// Clear all selections
    ClearAll,
    /// Cancel the current operation
    Cancel,
}
