//! Modern layout system for Flux GUI with sidebar navigation

use crate::app::AppView;
use crate::theme::FluxTheme;
use egui::{vec2, Color32, Context, Id, Rect, Response, Sense, Ui};
use egui_phosphor::regular;

/// Sidebar navigation item
#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub view: AppView,
    pub tooltip: &'static str,
}

impl NavItem {
    /// Default navigation items
    pub fn default_items() -> Vec<Self> {
        vec![
            NavItem {
                id: "welcome",
                icon: regular::HOUSE,
                label: "Home",
                view: AppView::Welcome,
                tooltip: "Start screen",
            },
            NavItem {
                id: "pack",
                icon: regular::PACKAGE,
                label: "Pack",
                view: AppView::Packing,
                tooltip: "Create archives",
            },
            NavItem {
                id: "extract",
                icon: regular::FOLDER_OPEN,
                label: "Extract",
                view: AppView::Extracting,
                tooltip: "Extract archives",
            },
            NavItem {
                id: "browse",
                icon: regular::BINOCULARS,
                label: "Browse",
                view: AppView::Browsing,
                tooltip: "Browse archive contents",
            },
            NavItem {
                id: "sync",
                icon: regular::ARROW_SQUARE_OUT,
                label: "Sync",
                view: AppView::Syncing,
                tooltip: "Incremental backup",
            },
        ]
    }
}

/// Sidebar navigation component
pub struct Sidebar {
    pub width: f32,
    pub collapsed: bool,
    pub animation_state: f32,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            width: 180.0,
            collapsed: false,
            animation_state: 1.0,
        }
    }
}

impl Sidebar {
    /// Toggle collapsed state
    pub fn toggle_collapse(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Get current width based on collapsed state and animation
    pub fn current_width(&self) -> f32 {
        let target_width = if self.collapsed { 60.0 } else { self.width };
        target_width * self.animation_state + 60.0 * (1.0 - self.animation_state)
    }

    /// Draw the sidebar
    pub fn show(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        current_view: &mut AppView,
        theme: &FluxTheme,
        items: &[NavItem],
    ) {
        // Animate width transition
        let animation_id = ui.make_persistent_id("sidebar_animation");
        self.animation_state = ctx.animate_bool_with_time(animation_id, !self.collapsed, 0.2);

        let sidebar_width = self.current_width();

        // Sidebar background
        let available_rect = ui.available_rect_before_wrap();
        let sidebar_rect = Rect::from_min_size(
            available_rect.min,
            vec2(sidebar_width, available_rect.height()),
        );

        ui.painter()
            .rect_filled(sidebar_rect, 0.0, theme.colors.panel_bg.gamma_multiply(0.8));

        // Draw sidebar content
        ui.allocate_ui_with_layout(
            vec2(sidebar_width, available_rect.height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);

                // Logo/Header area
                ui.horizontal(|ui| {
                    ui.add_space(if self.collapsed { 15.0 } else { 20.0 });

                    if !self.collapsed {
                        ui.heading("Flux");
                    } else {
                        ui.label(egui::RichText::new("F").size(20.0).strong());
                    }

                    // Collapse button
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        let collapse_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(if self.collapsed { "▶" } else { "◀" })
                                    .size(12.0),
                            )
                            .frame(false)
                            .min_size(vec2(20.0, 20.0)),
                        );

                        if collapse_btn.clicked() {
                            self.toggle_collapse();
                        }
                    });
                });

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // Navigation items
                for item in items {
                    let is_selected = current_view == &item.view;

                    ui.horizontal(|ui| {
                        let item_response = self.draw_nav_item(ui, item, is_selected, theme);

                        if item_response.clicked() {
                            *current_view = item.view;
                        }
                    });

                    ui.add_space(5.0);
                }

                // Spacer
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(20.0);

                    // Settings button at bottom
                    ui.horizontal(|ui| {
                        let settings_response = self.draw_nav_item(
                            ui,
                            &NavItem {
                                id: "settings",
                                icon: regular::GEAR,
                                label: "Settings",
                                view: AppView::Welcome, // Will implement Settings view later
                                tooltip: "Application settings",
                            },
                            false,
                            theme,
                        );

                        if settings_response.clicked() {
                            // TODO: Open settings dialog
                        }
                    });
                });
            },
        );
    }

    /// Draw a single navigation item
    fn draw_nav_item(
        &self,
        ui: &mut Ui,
        item: &NavItem,
        is_selected: bool,
        theme: &FluxTheme,
    ) -> Response {
        let available_width = ui.available_width();
        let item_height = 40.0;
        let (rect, response) =
            ui.allocate_exact_size(vec2(available_width, item_height), Sense::click());

        // Hover animation
        let hover_animation =
            ui.ctx()
                .animate_bool_with_time(response.id, response.hovered() || is_selected, 0.15);

        // Background
        if is_selected {
            ui.painter().rect_filled(
                rect,
                theme.rounding,
                theme.colors.primary.gamma_multiply(0.2),
            );

            // Selection indicator
            let indicator_rect = Rect::from_min_size(rect.min, vec2(4.0, rect.height()));
            ui.painter()
                .rect_filled(indicator_rect, 2.0, theme.colors.primary);
        } else if hover_animation > 0.0 {
            ui.painter().rect_filled(
                rect,
                theme.rounding,
                theme
                    .colors
                    .panel_bg
                    .lerp_to_gamma(theme.colors.primary.gamma_multiply(0.1), hover_animation),
            );
        }

        // Content
        let icon_size = 20.0;
        let padding = 20.0;
        let icon_pos = rect.min + vec2(padding, (item_height - icon_size) / 2.0);

        // Icon color
        let icon_color = if is_selected {
            theme.colors.primary
        } else {
            theme
                .colors
                .text
                .lerp_to_gamma(theme.colors.primary, hover_animation * 0.5)
        };

        // Draw icon
        ui.painter().text(
            icon_pos + vec2(icon_size / 2.0, icon_size / 2.0),
            egui::Align2::CENTER_CENTER,
            item.icon,
            egui::FontId::proportional(icon_size),
            icon_color,
        );

        // Draw label if not collapsed
        if !self.collapsed {
            let label_pos = icon_pos + vec2(icon_size + 15.0, icon_size / 2.0);
            ui.painter().text(
                label_pos,
                egui::Align2::LEFT_CENTER,
                item.label,
                egui::FontId::proportional(14.0),
                if is_selected {
                    theme.colors.text
                } else {
                    theme.colors.text_weak
                },
            );
        }

        // Tooltip when collapsed
        if self.collapsed {
            response.on_hover_text(item.label)
        } else {
            response.on_hover_text(item.tooltip)
        }
    }
}

/// Card component for modern UI
pub struct Card;

impl Card {
    /// Draw a card with content
    pub fn show<R>(
        ui: &mut Ui,
        theme: &FluxTheme,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        egui::Frame::none()
            .fill(theme.colors.panel_bg)
            .rounding(theme.rounding * 2.0)
            .inner_margin(egui::Margin::same(16.0))
            .shadow(egui::epaint::Shadow {
                offset: vec2(0.0, 2.0),
                blur: 8.0,
                spread: 0.0,
                color: Color32::from_black_alpha(20),
            })
            .show(ui, add_contents)
    }

    /// Draw a card with hover effect
    pub fn show_interactive<R>(
        ui: &mut Ui,
        theme: &FluxTheme,
        id: impl Into<Id>,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> egui::InnerResponse<R> {
        let id = id.into();
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(rect, id, Sense::hover());

        let hover_animation = ui.ctx().animate_bool_with_time(id, response.hovered(), 0.2);

        egui::Frame::none()
            .fill(
                theme
                    .colors
                    .panel_bg
                    .lerp_to_gamma(theme.colors.primary.gamma_multiply(0.05), hover_animation),
            )
            .rounding(theme.rounding * 2.0)
            .inner_margin(egui::Margin::same(16.0))
            .shadow(egui::epaint::Shadow {
                offset: vec2(0.0, 2.0 + hover_animation * 2.0),
                blur: 8.0 + hover_animation * 4.0,
                spread: 0.0,
                color: Color32::from_black_alpha((20.0 + hover_animation * 10.0) as u8),
            })
            .show(ui, |ui| add_contents(ui, hover_animation))
    }
}

/// Modern file/folder card for packing view
pub fn draw_file_card(
    ui: &mut Ui,
    theme: &FluxTheme,
    path: &std::path::Path,
    size: u64,
    index: usize,
    on_remove: impl FnOnce(),
) {
    let card_id = ui.make_persistent_id(("file_card", index));

    Card::show_interactive(ui, theme, card_id, |ui, hover| {
        ui.horizontal(|ui| {
            // File icon
            let icon = if path.is_dir() {
                regular::FOLDER
            } else {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("zip") | Some("tar") | Some("gz") => regular::ARCHIVE,
                    Some("txt") | Some("md") => regular::FILE_TEXT,
                    Some("jpg") | Some("png") | Some("gif") => regular::IMAGE,
                    _ => regular::FILE,
                }
            };

            ui.label(
                egui::RichText::new(icon)
                    .size(32.0)
                    .color(theme.colors.primary.gamma_multiply(0.8 + hover * 0.2)),
            );

            ui.add_space(12.0);

            // File info
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown"),
                    )
                    .strong(),
                );

                let size_text = format_file_size(size);
                ui.label(
                    egui::RichText::new(size_text)
                        .size(12.0)
                        .color(theme.colors.text_weak),
                );
            });

            // Remove button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let remove_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(regular::X)
                            .size(16.0)
                            .color(theme.colors.error.gamma_multiply(0.8 + hover * 0.2)),
                    )
                    .frame(false)
                    .min_size(vec2(24.0, 24.0)),
                );

                if remove_btn.clicked() {
                    on_remove();
                }

                remove_btn.on_hover_text("Remove from list");
            });
        });
    });
}

/// Format file size for display
fn format_file_size(bytes: u64) -> String {
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
