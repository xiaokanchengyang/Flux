//! Optimized browser view for handling extremely large archives
//! Uses lazy loading and virtualization for better performance

use super::browser_view::{format_size, get_file_icon, BrowserState};
use crate::theme::FluxTheme;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use egui_phosphor::regular;
use flux_core::archive::extractor::ArchiveEntry;
use std::collections::HashMap;
use std::path::PathBuf;

/// Optimized state for virtual scrolling with lazy loading
pub struct OptimizedBrowserState {
    /// All entries stored flat for efficient access
    pub entries: Vec<ArchiveEntry>,
    /// Parent-child relationships for tree structure
    pub tree_structure: HashMap<PathBuf, Vec<usize>>,
    /// Expanded state for directories
    pub expanded: HashMap<PathBuf, bool>,
    /// Visible entries cache (index, depth)
    pub visible_entries: Vec<(usize, usize)>,
    /// Whether visible entries need to be recalculated
    pub needs_refresh: bool,
    /// Selected items
    pub selected: HashMap<PathBuf, bool>,
    /// Search filter
    pub search_filter: String,
    /// Show hidden files
    pub show_hidden: bool,
}

impl OptimizedBrowserState {
    /// Create new optimized browser state from entries
    pub fn new(mut entries: Vec<ArchiveEntry>) -> Self {
        // Sort entries by path for better cache locality
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        
        // Build tree structure
        let mut tree_structure: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        let mut expanded = HashMap::new();
        
        // Root is always expanded
        expanded.insert(PathBuf::new(), true);
        
        for (idx, entry) in entries.iter().enumerate() {
            // Get parent path
            let parent = entry.path.parent().unwrap_or(&PathBuf::new()).to_path_buf();
            
            // Add to parent's children
            tree_structure.entry(parent).or_insert_with(Vec::new).push(idx);
            
            // If it's a directory, initialize its expansion state
            if entry.is_dir {
                expanded.insert(entry.path.clone(), false);
                // Ensure it has an entry in tree_structure even if empty
                tree_structure.entry(entry.path.clone()).or_insert_with(Vec::new);
            }
        }
        
        let mut state = Self {
            entries,
            tree_structure,
            expanded,
            visible_entries: Vec::new(),
            needs_refresh: true,
            selected: HashMap::new(),
            search_filter: String::new(),
            show_hidden: true,
        };
        
        // Initial calculation of visible entries
        state.refresh_visible_entries();
        
        state
    }
    
    /// Refresh the list of visible entries based on expansion state and filters
    pub fn refresh_visible_entries(&mut self) {
        self.visible_entries.clear();
        
        // Collect root children first to avoid borrow issues
        let root_children: Vec<usize> = self.tree_structure
            .get(&PathBuf::new())
            .map(|children| children.clone())
            .unwrap_or_default();
        
        // Start from root
        for child_idx in root_children {
            self.add_visible_entry(child_idx, 0);
        }
        
        self.needs_refresh = false;
    }
    
    /// Recursively add visible entries
    fn add_visible_entry(&mut self, entry_idx: usize, depth: usize) {
        let entry = &self.entries[entry_idx];
        
        // Apply filters
        if !self.search_filter.is_empty() {
            let name = entry.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !name.to_lowercase().contains(&self.search_filter.to_lowercase()) {
                return;
            }
        }
        
        if !self.show_hidden {
            if let Some(name) = entry.path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    return;
                }
            }
        }
        
        // Add to visible entries
        self.visible_entries.push((entry_idx, depth));
        
        // If it's a directory and expanded, add children
        if entry.is_dir && self.expanded.get(&entry.path).copied().unwrap_or(false) {
            // Clone children to avoid borrow issues
            let children: Vec<usize> = self.tree_structure
                .get(&entry.path)
                .map(|c| c.clone())
                .unwrap_or_default();
            
            for child_idx in children {
                self.add_visible_entry(child_idx, depth + 1);
            }
        }
    }
    
    /// Toggle expansion state of a directory
    pub fn toggle_expanded(&mut self, path: &PathBuf) {
        if let Some(expanded) = self.expanded.get_mut(path) {
            *expanded = !*expanded;
            self.needs_refresh = true;
        }
    }
    
    /// Toggle selection of an item
    pub fn toggle_selection(&mut self, path: &PathBuf) {
        let current = self.selected.get(path).copied().unwrap_or(false);
        self.selected.insert(path.clone(), !current);
    }
    
    /// Update search filter
    pub fn set_search_filter(&mut self, filter: String) {
        if self.search_filter != filter {
            self.search_filter = filter;
            self.needs_refresh = true;
        }
    }
    
    /// Update show hidden state
    pub fn set_show_hidden(&mut self, show: bool) {
        if self.show_hidden != show {
            self.show_hidden = show;
            self.needs_refresh = true;
        }
    }
}

/// Draw the optimized table view
pub fn draw_optimized_table_view(
    ui: &mut Ui,
    state: &mut BrowserState,
    opt_state: &mut OptimizedBrowserState,
    theme: &FluxTheme,
) {
    // Refresh visible entries if needed
    if opt_state.needs_refresh {
        opt_state.refresh_visible_entries();
    }
    
    // Search and filter controls
    ui.horizontal(|ui| {
        ui.label(regular::MAGNIFYING_GLASS);
        let search_response = ui.text_edit_singleline(&mut opt_state.search_filter);
        if search_response.changed() {
            opt_state.set_search_filter(opt_state.search_filter.clone());
        }
        
        ui.separator();
        
        let mut show_hidden = opt_state.show_hidden;
        if ui.checkbox(&mut show_hidden, "Show hidden files").changed() {
            opt_state.set_show_hidden(show_hidden);
        }
        
        ui.separator();
        
        ui.label(format!("Showing {} of {} items", 
            opt_state.visible_entries.len(),
            opt_state.entries.len()
        ));
    });
    
    ui.separator();
    
    // Build the table
    let available_height = ui.available_height();
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(30.0)) // Expand/Checkbox
        .column(Column::auto().at_least(20.0)) // Icon
        .column(Column::remainder().at_least(200.0)) // Name
        .column(Column::auto().at_least(80.0)) // Size
        .column(Column::auto().at_least(100.0)) // Type
        .column(Column::auto().at_least(150.0)) // Modified
        .min_scrolled_height(0.0)
        .max_scroll_height(available_height);
    
    // Header
    table
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("");
            });
            header.col(|ui| {
                ui.label("");
            });
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
        })
        .body(|body| {
            // Virtual scrolling - only render visible rows
            let row_height = 20.0;
            
            // Clone data we need to avoid borrow issues
            let visible_entries = opt_state.visible_entries.clone();
            let entries_len = opt_state.entries.len();
            
            body.rows(row_height, visible_entries.len(), |mut row| {
                let row_index = row.index();
                if let Some(&(entry_idx, depth)) = visible_entries.get(row_index) {
                    if entry_idx < entries_len {
                        // Extract the data we need before creating closures
                        let entry = &opt_state.entries[entry_idx];
                        let path = entry.path.clone();
                        let is_dir = entry.is_dir;
                        let size = entry.size;
                        let mtime = entry.mtime;
                        let is_selected = opt_state.selected.get(&path).copied().unwrap_or(false);
                        let is_highlighted = state.highlighted.as_ref() == Some(&path);
                        let is_expanded = opt_state.expanded.get(&path).copied().unwrap_or(false);
                        
                        // Expand/Checkbox column
                        row.col(|ui| {
                            ui.add_space(depth as f32 * 16.0);
                            
                            if is_dir {
                                // Expand/collapse button
                                let icon = if is_expanded { "▼" } else { "▶" };
                                if ui.small_button(icon).clicked() {
                                    // We'll handle state changes after the table
                                }
                            } else {
                                // Checkbox for files
                                let mut checkbox_state = is_selected;
                                ui.checkbox(&mut checkbox_state, "");
                            }
                        });
                        
                        // Icon column
                        row.col(|ui| {
                            let icon = if is_dir {
                                regular::FOLDER
                            } else {
                                get_file_icon(&path)
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
                            if !is_dir {
                                ui.label(format_size(size));
                            }
                        });
                        
                        // Type column
                        row.col(|ui| {
                            if is_dir {
                                ui.label("Directory");
                            } else {
                                ui.label(get_file_type(&path));
                            }
                        });
                        
                        // Modified column
                        row.col(|ui| {
                            if let Some(mtime) = mtime {
                                let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(mtime, 0)
                                    .unwrap_or_default();
                                ui.label(datetime.format("%Y-%m-%d %H:%M").to_string());
                            }
                        });
                    }
                }
            });
        });
}

/// Get file type string
fn get_file_type(path: &PathBuf) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        "txt" | "md" | "log" => "Text",
        "jpg" | "jpeg" | "png" | "gif" => "Image",
        "mp3" | "wav" | "flac" => "Audio",
        "mp4" | "avi" | "mkv" => "Video",
        "zip" | "tar" | "gz" | "7z" => "Archive",
        "pdf" => "PDF",
        "doc" | "docx" => "Document",
        "xls" | "xlsx" => "Spreadsheet",
        "rs" | "py" | "js" | "cpp" => "Source Code",
        _ => "File",
    }
}