//! Archive browser view for exploring and extracting archive contents

use crate::components::{set_theme_in_context, FluxButton};
use crate::layout::Card;
use crate::theme::FluxTheme;
use egui::{vec2, Context, Ui, Widget};
use egui_phosphor::regular;
use flux_core::archive::extractor::ArchiveEntry;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Tree node for file hierarchy
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub entry: Option<ArchiveEntry>,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(name: String, path: PathBuf, entry: Option<ArchiveEntry>) -> Self {
        Self {
            name,
            path,
            entry,
            children: Vec::new(),
            is_expanded: false,
        }
    }

    /// Build tree structure from flat list of entries
    pub fn build_tree(entries: Vec<ArchiveEntry>) -> TreeNode {
        let mut root = TreeNode::new("Archive Root".to_string(), PathBuf::new(), None);

        for entry in entries {
            let components: Vec<_> = entry.path.components().collect();
            let mut current = &mut root;

            for (i, component) in components.iter().enumerate() {
                let name = component.as_os_str().to_string_lossy().to_string();
                let path = components[..=i].iter().collect::<PathBuf>();

                // Find or create child node
                let child_idx = current.children.iter().position(|c| c.name == name);

                if let Some(idx) = child_idx {
                    current = &mut current.children[idx];
                } else {
                    let is_last = i == components.len() - 1;
                    let node_entry = if is_last { Some(entry.clone()) } else { None };

                    current.children.push(TreeNode::new(name, path, node_entry));
                    current = current.children.last_mut().unwrap();
                }
            }
        }

        // Auto-expand root
        root.is_expanded = true;

        root
    }

    /// Check if this node or any descendant is selected
    pub fn has_selected_descendant(&self, selected: &HashSet<PathBuf>) -> bool {
        if selected.contains(&self.path) {
            return true;
        }

        for child in &self.children {
            if child.has_selected_descendant(selected) {
                return true;
            }
        }

        false
    }

    /// Get all entry paths under this node
    pub fn get_all_entry_paths(&self, paths: &mut Vec<PathBuf>) {
        if self.entry.is_some() {
            paths.push(self.path.clone());
        }

        for child in &self.children {
            child.get_all_entry_paths(paths);
        }
    }
}

/// Archive browser state
pub struct BrowserState {
    /// The archive file being browsed
    pub archive_path: PathBuf,
    /// Tree structure of archive contents
    pub tree: TreeNode,
    /// Selected items (paths)
    pub selected: HashSet<PathBuf>,
    /// Currently highlighted item
    pub highlighted: Option<PathBuf>,
    /// Search filter
    pub search_filter: String,
    /// Show hidden files
    pub show_hidden: bool,
    /// Info panel width
    pub info_panel_width: f32,
    /// Total archive size
    pub total_size: u64,
    /// Number of files
    pub file_count: usize,
    /// Number of directories
    pub dir_count: usize,
    /// Use table view instead of tree view
    pub use_table_view: bool,
}

impl BrowserState {
    /// Create a new browser state from entries
    pub fn new(archive_path: PathBuf, entries: Vec<ArchiveEntry>) -> Self {
        let mut total_size = 0u64;
        let mut file_count = 0;
        let mut dir_count = 0;

        for entry in &entries {
            if entry.is_dir {
                dir_count += 1;
            } else {
                file_count += 1;
                total_size += entry.size;
            }
        }

        let tree = TreeNode::build_tree(entries);

        Self {
            archive_path,
            tree,
            selected: HashSet::new(),
            highlighted: None,
            search_filter: String::new(),
            show_hidden: true,
            info_panel_width: 300.0,
            total_size,
            file_count,
            dir_count,
            use_table_view: false,
        }
    }

    /// Toggle selection of an item
    #[allow(dead_code)]
    pub fn toggle_selection(&mut self, path: PathBuf) {
        if self.selected.contains(&path) {
            self.selected.remove(&path);
        } else {
            self.selected.insert(path);
        }
    }

    /// Select all items under a node
    #[allow(dead_code)]
    pub fn select_node_recursive(&mut self, node: &TreeNode) {
        let mut paths = Vec::new();
        node.get_all_entry_paths(&mut paths);

        for path in paths {
            self.selected.insert(path);
        }
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Get selected entries
    pub fn get_selected_entries(&self) -> Vec<ArchiveEntry> {
        let mut entries = Vec::new();
        self.collect_selected_entries(&self.tree, &mut entries);
        entries
    }

    fn collect_selected_entries(&self, node: &TreeNode, entries: &mut Vec<ArchiveEntry>) {
        if let Some(entry) = &node.entry {
            if self.selected.contains(&node.path) {
                entries.push(entry.clone());
            }
        }

        for child in &node.children {
            self.collect_selected_entries(child, entries);
        }
    }
}

/// Actions that can be triggered from the browser view
#[derive(Debug, Clone)]
pub enum BrowserAction {
    /// Extract selected items to a directory
    #[allow(dead_code)]
    ExtractSelected(PathBuf),
    /// Extract all items to a directory
    #[allow(dead_code)]
    ExtractAll(PathBuf),
    /// Close the browser and return to main view
    Close,
    /// Open file dialog to choose extraction destination
    ChooseDestination,
}

/// Draw the archive browser view
pub fn draw_browser_view(
    ctx: &Context,
    ui: &mut Ui,
    state: &mut BrowserState,
    theme: &FluxTheme,
) -> Option<BrowserAction> {
    set_theme_in_context(ctx, theme);

    let mut action = None;

    // Header
    ui.horizontal(|ui| {
        ui.heading("Archive Browser");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = Some(BrowserAction::Close);
            }
        });
    });

    ui.separator();

    // Archive info bar
    Card::show(ui, theme, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(regular::ARCHIVE)
                    .size(20.0)
                    .color(theme.colors.primary),
            );
            ui.label(
                egui::RichText::new(
                    state
                        .archive_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown Archive"),
                )
                .strong(),
            );

            ui.separator();

            ui.label(format!("{} files", state.file_count));
            ui.label(format!("{} folders", state.dir_count));
            ui.label(format!("Total: {}", format_size(state.total_size)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Action buttons
                let extract_all_btn = FluxButton::new("Extract All")
                    .icon(regular::DOWNLOAD_SIMPLE)
                    .primary();

                if extract_all_btn.ui(ui).clicked() {
                    action = Some(BrowserAction::ChooseDestination);
                }

                let selected_count = state.selected.len();
                if selected_count > 0 {
                    let extract_selected_btn =
                        FluxButton::new(format!("Extract {} Selected", selected_count))
                            .icon(regular::DOWNLOAD);

                    if extract_selected_btn.ui(ui).clicked() {
                        action = Some(BrowserAction::ChooseDestination);
                    }
                }
            });
        });
    });

    ui.add_space(8.0);

    // Search and filters
    ui.horizontal(|ui| {
        ui.label(regular::MAGNIFYING_GLASS);
        ui.text_edit_singleline(&mut state.search_filter);

        ui.separator();

        ui.checkbox(&mut state.show_hidden, "Show hidden files");

        ui.separator();

        // View mode toggle
        if ui
            .selectable_label(!state.use_table_view, "🌳 Tree")
            .clicked()
        {
            state.use_table_view = false;
        }
        if ui
            .selectable_label(state.use_table_view, "📊 Table")
            .clicked()
        {
            state.use_table_view = true;
        }

        if !state.selected.is_empty() {
            ui.separator();
            if ui.button("Clear Selection").clicked() {
                state.clear_selection();
            }
        }
    });

    ui.separator();

    // Main content area with tree and info panel
    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let tree_width = available_width - state.info_panel_width - 8.0;

        // File tree or table view
        ui.allocate_ui(vec2(tree_width, ui.available_height()), |ui| {
            if state.use_table_view {
                // Use table view for better performance with large archives
                super::browser_table_view::draw_table_view(ui, state, theme);
            } else {
                // Traditional tree view
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Temporarily extract fields to avoid mutable borrow issues
                    let selected = &state.selected;
                    let highlighted = &state.highlighted;
                    let search_filter = &state.search_filter;
                    let show_hidden = state.show_hidden;

                    let (new_highlighted, selection_changes) = draw_tree_node(
                        ui,
                        &mut state.tree,
                        selected,
                        highlighted,
                        search_filter,
                        show_hidden,
                        theme,
                        0,
                    );

                    // Apply changes after drawing
                    if let Some(path) = new_highlighted {
                        state.highlighted = Some(path);
                    }

                    for (path, selected) in selection_changes {
                        if selected {
                            state.selected.insert(path);
                        } else {
                            state.selected.remove(&path);
                        }
                    }
                });
            }
        });

        ui.separator();

        // Info panel
        ui.allocate_ui(vec2(state.info_panel_width, ui.available_height()), |ui| {
            draw_info_panel(ui, state, theme);
        });
    });

    action
}

/// Draw a tree node and its children
fn draw_tree_node(
    ui: &mut Ui,
    node: &mut TreeNode,
    selected: &HashSet<PathBuf>,
    highlighted: &Option<PathBuf>,
    search_filter: &str,
    show_hidden: bool,
    theme: &FluxTheme,
    depth: usize,
) -> (Option<PathBuf>, Vec<(PathBuf, bool)>) {
    let mut new_highlighted = None;
    let mut selection_changes = Vec::new();

    // Skip if filtered
    if !search_filter.is_empty()
        && !node
            .name
            .to_lowercase()
            .contains(&search_filter.to_lowercase())
        && !node
            .children
            .iter()
            .any(|c| contains_filter(c, search_filter))
    {
        return (new_highlighted, selection_changes);
    }

    // Skip hidden files if needed
    if !show_hidden && node.name.starts_with('.') {
        return (new_highlighted, selection_changes);
    }

    let indent = depth as f32 * 20.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        let has_children = !node.children.is_empty();
        let is_selected = selected.contains(&node.path);
        let is_highlighted = highlighted.as_ref() == Some(&node.path);

        // Expand/collapse button for directories
        if has_children {
            let arrow = if node.is_expanded { "▼" } else { "▶" };
            if ui.small_button(arrow).clicked() {
                node.is_expanded = !node.is_expanded;
            }
        } else {
            ui.add_space(20.0); // Spacing for alignment
        }

        // Selection checkbox
        let mut checkbox_selected =
            is_selected || (has_children && node.has_selected_descendant(selected));
        if ui.checkbox(&mut checkbox_selected, "").clicked() {
            if has_children {
                // Select/deselect all children
                let mut paths = Vec::new();
                node.get_all_entry_paths(&mut paths);
                for path in paths {
                    selection_changes.push((path, checkbox_selected));
                }
            } else {
                selection_changes.push((node.path.clone(), !is_selected));
            }
        }

        // Icon
        let icon = if let Some(entry) = &node.entry {
            if entry.is_dir {
                regular::FOLDER
            } else {
                get_file_icon(&node.path)
            }
        } else {
            regular::FOLDER
        };

        ui.label(
            egui::RichText::new(icon)
                .size(16.0)
                .color(theme.colors.primary),
        );

        // Name
        let name_response = ui.selectable_label(
            is_highlighted,
            egui::RichText::new(&node.name).color(if is_selected {
                theme.colors.primary
            } else {
                theme.colors.text
            }),
        );

        if name_response.clicked() {
            new_highlighted = Some(node.path.clone());
        }

        if name_response.double_clicked() && has_children {
            node.is_expanded = !node.is_expanded;
        }

        // Size for files
        if let Some(entry) = &node.entry {
            if !entry.is_dir {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format_size(entry.size))
                            .size(12.0)
                            .color(theme.colors.text_weak),
                    );
                });
            }
        }
    });

    // Draw children if expanded
    if node.is_expanded {
        for child in &mut node.children {
            let (child_highlighted, child_changes) = draw_tree_node(
                ui,
                child,
                selected,
                highlighted,
                search_filter,
                show_hidden,
                theme,
                depth + 1,
            );
            if child_highlighted.is_some() {
                new_highlighted = child_highlighted;
            }
            selection_changes.extend(child_changes);
        }
    }

    (new_highlighted, selection_changes)
}

/// Draw the info panel showing details about selected item
fn draw_info_panel(ui: &mut Ui, state: &BrowserState, theme: &FluxTheme) {
    ui.heading("Details");
    ui.separator();

    if let Some(highlighted_path) = &state.highlighted {
        if let Some(entry) = find_entry_by_path(&state.tree, highlighted_path) {
            Card::show(ui, theme, |ui| {
                ui.vertical(|ui| {
                    // File name
                    ui.label(
                        egui::RichText::new(
                            highlighted_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown"),
                        )
                        .strong()
                        .size(16.0),
                    );

                    ui.add_space(8.0);

                    // Details grid
                    egui::Grid::new("file_details")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Type:");
                            if entry.is_dir {
                                ui.label("Directory");
                            } else {
                                ui.label(get_file_type(highlighted_path));
                            }
                            ui.end_row();

                            ui.label("Size:");
                            ui.label(format_size(entry.size));
                            ui.end_row();

                            if let Some(compressed_size) = entry.compressed_size {
                                ui.label("Compressed:");
                                ui.label(format!(
                                    "{} ({:.1}%)",
                                    format_size(compressed_size),
                                    (compressed_size as f64 / entry.size as f64) * 100.0
                                ));
                                ui.end_row();
                            }

                            if let Some(mtime) = entry.mtime {
                                ui.label("Modified:");
                                ui.label(format_timestamp(mtime));
                                ui.end_row();
                            }

                            if let Some(mode) = entry.mode {
                                ui.label("Permissions:");
                                ui.label(format_permissions(mode));
                                ui.end_row();
                            }

                            ui.label("Path:");
                            ui.label(highlighted_path.to_string_lossy().to_string());
                            ui.end_row();
                        });
                });
            });
        }
    } else if state.selected.is_empty() {
        ui.label(
            egui::RichText::new("Select an item to view details")
                .color(theme.colors.text_weak)
                .italics(),
        );
    } else {
        // Multiple selection summary
        Card::show(ui, theme, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} items selected", state.selected.len()))
                        .strong(),
                );

                ui.add_space(8.0);

                let selected_entries = state.get_selected_entries();
                let total_size: u64 = selected_entries
                    .iter()
                    .filter(|e| !e.is_dir)
                    .map(|e| e.size)
                    .sum();

                let file_count = selected_entries.iter().filter(|e| !e.is_dir).count();
                let dir_count = selected_entries.iter().filter(|e| e.is_dir).count();

                egui::Grid::new("selection_summary")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Files:");
                        ui.label(file_count.to_string());
                        ui.end_row();

                        ui.label("Directories:");
                        ui.label(dir_count.to_string());
                        ui.end_row();

                        ui.label("Total size:");
                        ui.label(format_size(total_size));
                        ui.end_row();
                    });
            });
        });
    }
}

/// Check if a node or its children contain the filter string
fn contains_filter(node: &TreeNode, filter: &str) -> bool {
    let filter_lower = filter.to_lowercase();

    if node.name.to_lowercase().contains(&filter_lower) {
        return true;
    }

    for child in &node.children {
        if contains_filter(child, &filter_lower) {
            return true;
        }
    }

    false
}

/// Find an entry by path in the tree
fn find_entry_by_path<'a>(node: &'a TreeNode, path: &Path) -> Option<&'a ArchiveEntry> {
    if node.path == path {
        return node.entry.as_ref();
    }

    for child in &node.children {
        if let Some(entry) = find_entry_by_path(child, path) {
            return Some(entry);
        }
    }

    None
}

/// Get icon for file type
pub fn get_file_icon(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("zip") | Some("tar") | Some("gz") | Some("7z") => regular::ARCHIVE,
        Some("txt") | Some("md") | Some("log") => regular::FILE_TEXT,
        Some("jpg") | Some("png") | Some("gif") | Some("svg") => regular::IMAGE,
        Some("mp3") | Some("wav") | Some("flac") => regular::FILE_AUDIO,
        Some("mp4") | Some("avi") | Some("mkv") => regular::FILE_VIDEO,
        Some("pdf") => regular::FILE_PDF,
        Some("doc") | Some("docx") | Some("odt") => regular::FILE_DOC,
        Some("xls") | Some("xlsx") | Some("ods") => regular::FILE_XLS,
        Some("ppt") | Some("pptx") | Some("odp") => regular::FILE_PPT,
        Some("rs") | Some("py") | Some("js") | Some("cpp") => regular::FILE_CODE,
        _ => regular::FILE,
    }
}

/// Get human-readable file type
fn get_file_type(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("zip") => "ZIP Archive".to_string(),
        Some("tar") => "TAR Archive".to_string(),
        Some("gz") => "Gzip Compressed".to_string(),
        Some("7z") => "7-Zip Archive".to_string(),
        Some("txt") => "Text Document".to_string(),
        Some("md") => "Markdown Document".to_string(),
        Some("pdf") => "PDF Document".to_string(),
        Some("jpg") | Some("jpeg") => "JPEG Image".to_string(),
        Some("png") => "PNG Image".to_string(),
        Some("gif") => "GIF Image".to_string(),
        Some("mp3") => "MP3 Audio".to_string(),
        Some("mp4") => "MP4 Video".to_string(),
        Some("rs") => "Rust Source".to_string(),
        Some("py") => "Python Script".to_string(),
        Some("js") => "JavaScript".to_string(),
        Some(ext) => ext.to_uppercase(),
        None => "File".to_string(),
    }
}

/// Format file size for display
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format Unix timestamp
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};

    if let Some(dt) = Local.timestamp_opt(timestamp, 0).single() {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Format Unix permissions
fn format_permissions(mode: u32) -> String {
    let mut perms = String::with_capacity(10);

    // File type
    perms.push(match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        _ => '-',
    });

    // User permissions
    perms.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    perms.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    perms.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // Group permissions
    perms.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    perms.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    perms.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // Other permissions
    perms.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    perms.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    perms.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    perms
}
