//! Table-based browser view for better performance with large archives
//! This module provides a virtual scrolling table view using egui_extras::Table

use super::browser_view::{format_size, get_file_icon, BrowserState};
use crate::theme::FluxTheme;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use egui_phosphor::regular;
use flux_core::archive::extractor::ArchiveEntry;
use std::path::{Path, PathBuf};

/// Draw the table-based browser view with virtual scrolling
pub fn draw_table_view(ui: &mut Ui, state: &mut BrowserState, theme: &FluxTheme) {
    // Flatten the tree into a list for table display
    let mut flat_entries = Vec::new();
    flatten_tree(
        &state.tree,
        &mut flat_entries,
        &state.search_filter,
        state.show_hidden,
        0,
    );

    // Calculate available height for the table
    let available_height = ui.available_height();

    // Build the table with virtual scrolling
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(20.0)) // Checkbox
        .column(Column::auto().at_least(20.0)) // Icon
        .column(Column::remainder().at_least(200.0)) // Name
        .column(Column::auto().at_least(80.0)) // Size
        .column(Column::auto().at_least(100.0)) // Type
        .column(Column::auto().at_least(150.0)) // Modified
        .column(Column::auto().at_least(100.0)) // Compressed
        .min_scrolled_height(0.0)
        .max_scroll_height(available_height);

    // Header
    table
        .header(20.0, |mut header| {
            header.col(|ui| {
                // Select all checkbox
                let all_selected = !flat_entries.is_empty()
                    && flat_entries.iter().all(|(_, entry, _)| {
                        entry
                            .as_ref()
                            .is_some_and(|e| !e.is_dir && state.selected.contains(&e.path))
                    });
                let mut checkbox_state = all_selected;
                if ui.checkbox(&mut checkbox_state, "").changed() {
                    for (path, entry, _) in &flat_entries {
                        if let Some(e) = entry {
                            if !e.is_dir {
                                if checkbox_state {
                                    state.selected.insert(path.clone());
                                } else {
                                    state.selected.remove(path);
                                }
                            }
                        }
                    }
                }
            });
            header.col(|_| {}); // Icon column
            header.col(|ui| {
                ui.strong("Name");
            });
            header.col(|ui| {
                ui.strong("Size");
            });
            header.col(|ui| {
                ui.strong("Type");
            });
            header.col(|ui| {
                ui.strong("Modified");
            });
            header.col(|ui| {
                ui.strong("Compressed");
            });
        })
        .body(|body| {
            // Virtual scrolling body
            body.rows(20.0, flat_entries.len(), |mut row| {
                let row_index = row.index();
                if let Some((path, entry_opt, indent)) = flat_entries.get(row_index) {
                    let is_selected = state.selected.contains(path);
                    let is_highlighted = state.highlighted.as_ref() == Some(path);

                    // Checkbox column
                    row.col(|ui| {
                        ui.add_space(*indent as f32 * 10.0);
                        if let Some(entry) = entry_opt {
                            if !entry.is_dir {
                                let mut checkbox_state = is_selected;
                                if ui.checkbox(&mut checkbox_state, "").changed() {
                                    if checkbox_state {
                                        state.selected.insert(path.clone());
                                    } else {
                                        state.selected.remove(path);
                                    }
                                }
                            }
                        }
                    });

                    // Icon column
                    row.col(|ui| {
                        let icon = if let Some(entry) = entry_opt {
                            if entry.is_dir {
                                regular::FOLDER
                            } else {
                                get_file_icon(path)
                            }
                        } else {
                            regular::FOLDER
                        };
                        ui.label(
                            egui::RichText::new(icon)
                                .size(16.0)
                                .color(theme.colors.primary),
                        );
                    });

                    // Name column
                    row.col(|ui| {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_else(|| path.to_str().unwrap_or("Unknown"));

                        let response = ui.selectable_label(
                            is_highlighted,
                            egui::RichText::new(name).color(if is_selected {
                                theme.colors.primary
                            } else {
                                theme.colors.text
                            }),
                        );

                        if response.clicked() {
                            state.highlighted = Some(path.clone());
                        }
                    });

                    // Size column
                    row.col(|ui| {
                        if let Some(entry) = entry_opt {
                            if !entry.is_dir {
                                ui.label(format_size(entry.size));
                            }
                        }
                    });

                    // Type column
                    row.col(|ui| {
                        if let Some(entry) = entry_opt {
                            if entry.is_dir {
                                ui.label("Directory");
                            } else {
                                ui.label(get_file_type(path));
                            }
                        }
                    });

                    // Modified column
                    row.col(|ui| {
                        if let Some(entry) = entry_opt {
                            if let Some(mtime) = entry.mtime {
                                let datetime =
                                    chrono::DateTime::<chrono::Utc>::from_timestamp(mtime, 0)
                                        .unwrap_or_default();
                                ui.label(datetime.format("%Y-%m-%d %H:%M").to_string());
                            }
                        }
                    });

                    // Compressed column
                    row.col(|ui| {
                        if let Some(entry) = entry_opt {
                            if let Some(compressed) = entry.compressed_size {
                                ui.label(format!(
                                    "{} ({:.1}%)",
                                    format_size(compressed),
                                    (compressed as f64 / entry.size as f64) * 100.0
                                ));
                            }
                        }
                    });
                }
            });
        });
}

/// Flatten the tree structure into a list of entries for table display
fn flatten_tree(
    node: &super::browser_view::TreeNode,
    flat_list: &mut Vec<(PathBuf, Option<ArchiveEntry>, usize)>,
    search_filter: &str,
    show_hidden: bool,
    depth: usize,
) {
    // Skip root node
    if depth > 0 {
        // Apply filters
        if !search_filter.is_empty()
            && !node
                .name
                .to_lowercase()
                .contains(&search_filter.to_lowercase())
        {
            return;
        }

        if !show_hidden && node.name.starts_with('.') {
            return;
        }

        flat_list.push((node.path.clone(), node.entry.clone(), depth - 1));
    }

    // Recursively add children if it's a directory
    for child in &node.children {
        flatten_tree(child, flat_list, search_filter, show_hidden, depth + 1);
    }
}

/// Get file type string from path
fn get_file_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" | "md" | "log" => "Text",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" => "Image",
        "mp3" | "wav" | "flac" | "ogg" => "Audio",
        "mp4" | "avi" | "mkv" | "mov" => "Video",
        "zip" | "tar" | "gz" | "7z" | "rar" => "Archive",
        "pdf" => "PDF",
        "doc" | "docx" => "Document",
        "xls" | "xlsx" => "Spreadsheet",
        "exe" | "msi" => "Executable",
        "rs" | "py" | "js" | "cpp" | "java" => "Source Code",
        _ => "File",
    }
}
