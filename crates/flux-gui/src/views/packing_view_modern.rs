//! Modern packing view with card-based UI
use eframe::egui;
use std::path::PathBuf;
use crate::layout::{Card, draw_file_card};
use crate::components::{FluxButton, DropZone, FluxProgress};
use crate::theme::FluxTheme;
use egui_phosphor::regular;

/// Draw the modern packing view
pub fn draw_packing_view_modern(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    input_files: &[PathBuf],
    output_path: &Option<PathBuf>,
    compression_format: &mut String,
    is_busy: bool,
    theme: &FluxTheme,
    current_progress: f32,
    status_text: &str,
) -> Option<super::PackingAction> {
    let mut action = None;
    
    // Header section
    ui.horizontal(|ui| {
        ui.heading("Create Archive");
        
        // Action buttons in header
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if is_busy {
                if ui.add(
                    FluxButton::new("Cancel")
                        .danger()
                        .icon(regular::X_CIRCLE)
                ).clicked() {
                    action = Some(super::PackingAction::Cancel);
                }
            } else if input_files.is_empty() {
                // Show nothing
            } else {
                if ui.add(
                    FluxButton::new("Start Packing")
                        .primary()
                        .icon(regular::PLAY)
                ).clicked() && output_path.is_some() {
                    action = Some(super::PackingAction::StartPacking);
                }
                
                ui.add_space(8.0);
                
                if ui.add(
                    FluxButton::new("Clear All")
                        .ghost()
                        .icon(regular::TRASH)
                ).clicked() {
                    action = Some(super::PackingAction::ClearAll);
                }
            }
        });
    });
    
    ui.add_space(20.0);
    
    // Progress section if busy
    if is_busy {
        Card::show(ui, theme, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Packing in progress...").size(16.0).strong());
                ui.add_space(10.0);
                
                ui.add(FluxProgress::new(current_progress).text(status_text));
            });
        });
        
        ui.add_space(20.0);
    }
    
    // Files section
    if input_files.is_empty() {
        // Show drop zone when no files
        let drop_response = ui.add(
            DropZone::new("pack_drop")
                .text("Drop files or folders to pack")
                .subtext("You can add multiple items")
        );
        
        if drop_response.clicked() {
            if let Some(_files) = rfd::FileDialog::new().pick_files() {
                // This would be handled by the parent through a different mechanism
                // For now, we'll use the AddMoreFiles action
                action = Some(super::PackingAction::AddMoreFiles);
            }
        }
    } else {
        // Configuration card
        Card::show(ui, theme, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Archive Settings").size(16.0).strong());
                ui.add_space(10.0);
                
                // Format selection
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    ui.add_space(10.0);
                    
                    let formats = [
                        ("zip", "ZIP", "Universal compatibility"),
                        ("tar.gz", "TAR.GZ", "Good compression"),
                        ("tar.zst", "TAR.ZST", "Best performance"),
                        ("tar.xz", "TAR.XZ", "Best compression"),
                    ];
                    
                    for (value, label, desc) in formats {
                        let is_selected = compression_format == value;
                        let format_id = ui.make_persistent_id(("format", value));
                        
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(120.0, 60.0),
                            egui::Sense::click(),
                        );
                        
                        if response.clicked() && !is_busy {
                            *compression_format = value.to_string();
                        }
                        
                        let hover_anim = ctx.animate_bool_with_time(
                            format_id,
                            response.hovered() || is_selected,
                            0.15
                        );
                        
                        // Draw format option card
                        let bg_color = if is_selected {
                            theme.colors.primary.gamma_multiply(0.2)
                        } else {
                            theme.colors.panel_bg.lerp_to_gamma(
                                theme.colors.primary.gamma_multiply(0.1),
                                hover_anim * 0.5
                            )
                        };
                        
                        ui.painter().rect_filled(rect, theme.rounding, bg_color);
                        
                        if is_selected {
                            ui.painter().rect_stroke(
                                rect,
                                theme.rounding,
                                egui::Stroke::new(2.0, theme.colors.primary),
                            );
                        }
                        
                        // Draw content
                        ui.painter().text(
                            rect.center() - egui::vec2(0.0, 10.0),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(14.0),
                            if is_selected { theme.colors.primary } else { theme.colors.text },
                        );
                        
                        ui.painter().text(
                            rect.center() + egui::vec2(0.0, 10.0),
                            egui::Align2::CENTER_CENTER,
                            desc,
                            egui::FontId::proportional(10.0),
                            theme.colors.text_weak,
                        );
                        
                        ui.add_space(10.0);
                    }
                });
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Output path
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Output:").strong());
                    
                    if let Some(path) = output_path {
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .monospace()
                                .color(theme.colors.text_weak)
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("No output selected")
                                .color(theme.colors.warning)
                        );
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            FluxButton::new("Browse")
                                .icon(regular::FOLDER_OPEN)
                        ).on_hover_text("Select output location").clicked() && !is_busy {
                            action = Some(super::PackingAction::SelectOutput);
                        }
                    });
                });
            });
        });
        
        ui.add_space(20.0);
        
        // Files list header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Files to Pack ({})", input_files.len()))
                    .size(16.0)
                    .strong()
            );
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(
                    FluxButton::new("Add More")
                        .ghost()
                        .icon(regular::PLUS)
                ).clicked() && !is_busy {
                    action = Some(super::PackingAction::AddMoreFiles);
                }
            });
        });
        
        ui.add_space(10.0);
        
        // Files grid
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut file_to_remove = None;
                
                // Calculate file sizes
                let file_infos: Vec<(PathBuf, u64)> = input_files.iter()
                    .map(|p| {
                        let size = if p.is_file() {
                            std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
                        } else {
                            calculate_dir_size(p)
                        };
                        (p.clone(), size)
                    })
                    .collect();
                
                // Draw file cards in a responsive grid
                let card_width = 350.0;
                let spacing = 10.0;
                let cards_per_row = ((ui.available_width() + spacing) / (card_width + spacing)).floor() as usize;
                
                if cards_per_row > 1 {
                    // Grid layout
                    for chunk in file_infos.chunks(cards_per_row) {
                        ui.horizontal(|ui| {
                            for (idx, (path, size)) in chunk.iter().enumerate() {
                                let global_idx = chunk.as_ptr() as usize - file_infos.as_ptr() as usize + idx;
                                
                                ui.allocate_ui(egui::vec2(card_width, 80.0), |ui| {
                                    draw_file_card(
                                        ui,
                                        theme,
                                        path,
                                        *size,
                                        global_idx,
                                        || {
                                            if !is_busy {
                                                file_to_remove = Some(global_idx);
                                            }
                                        },
                                    );
                                });
                                
                                if idx < chunk.len() - 1 {
                                    ui.add_space(spacing);
                                }
                            }
                        });
                        ui.add_space(spacing);
                    }
                } else {
                    // Single column layout
                    for (idx, (path, size)) in file_infos.iter().enumerate() {
                        draw_file_card(
                            ui,
                            theme,
                            path,
                            *size,
                            idx,
                            || {
                                if !is_busy {
                                    file_to_remove = Some(idx);
                                }
                            },
                        );
                        ui.add_space(spacing);
                    }
                }
                
                if let Some(idx) = file_to_remove {
                    action = Some(super::PackingAction::RemoveFile(idx));
                }
            });
    }
    
    action
}

fn calculate_dir_size(path: &PathBuf) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += calculate_dir_size(&entry.path());
                }
            }
        }
    }
    size
}