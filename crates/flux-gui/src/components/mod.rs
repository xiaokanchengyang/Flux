//! Custom UI components for Flux GUI

use crate::theme::FluxTheme;
use egui::{vec2, Color32, Context, Id, Rect, Response, Sense, Ui, Widget};
use egui_phosphor::regular;

/// A modern button with Flux styling
pub struct FluxButton {
    text: String,
    icon: Option<&'static str>,
    variant: ButtonVariant,
    min_size: egui::Vec2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

impl FluxButton {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            variant: ButtonVariant::Secondary,
            min_size: vec2(0.0, 36.0),
        }
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn min_size(mut self, size: egui::Vec2) -> Self {
        self.min_size = size;
        self
    }
}

impl Widget for FluxButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = ui
            .ctx()
            .data(|d| d.get_temp::<FluxTheme>(Id::NULL).unwrap_or_default());

        let padding = vec2(16.0, 8.0);
        let icon_spacing = 8.0;

        // Calculate size
        let text_size = ui.fonts(|f| {
            f.layout_no_wrap(
                self.text.clone(),
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            )
            .rect
            .size()
        });

        let icon_size = if self.icon.is_some() { 16.0 } else { 0.0 };
        let content_width = if self.icon.is_some() {
            icon_size + icon_spacing + text_size.x
        } else {
            text_size.x
        };

        let desired_size = vec2(
            (content_width + padding.x * 2.0).max(self.min_size.x),
            (text_size.y + padding.y * 2.0).max(self.min_size.y),
        );

        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // Get colors based on variant and state
            let (bg_color, text_color) = match self.variant {
                ButtonVariant::Primary => {
                    let bg = if response.hovered() {
                        theme.colors.primary_hover
                    } else if response.is_pointer_button_down_on() {
                        theme.colors.primary.gamma_multiply(0.8)
                    } else {
                        theme.colors.primary
                    };
                    (bg, Color32::WHITE)
                }
                ButtonVariant::Secondary => {
                    let bg = if response.hovered() {
                        theme.colors.panel_bg.gamma_multiply(1.2)
                    } else if response.is_pointer_button_down_on() {
                        theme.colors.panel_bg.gamma_multiply(0.8)
                    } else {
                        theme.colors.panel_bg
                    };
                    (bg, theme.colors.text)
                }
                ButtonVariant::Danger => {
                    let bg = if response.hovered() {
                        theme.colors.error.gamma_multiply(1.2)
                    } else if response.is_pointer_button_down_on() {
                        theme.colors.error.gamma_multiply(0.8)
                    } else {
                        theme.colors.error
                    };
                    (bg, Color32::WHITE)
                }
                ButtonVariant::Ghost => {
                    let bg = if response.hovered() {
                        theme.colors.panel_bg.gamma_multiply(0.5)
                    } else if response.is_pointer_button_down_on() {
                        theme.colors.panel_bg.gamma_multiply(0.3)
                    } else {
                        Color32::TRANSPARENT
                    };
                    (bg, theme.colors.text)
                }
            };

            // Draw background
            ui.painter()
                .rect(rect, theme.rounding, bg_color, visuals.bg_stroke);

            // Draw content
            let mut cursor = rect.min + vec2(padding.x, rect.height() / 2.0);

            // Draw icon if present
            if let Some(icon) = self.icon {
                ui.painter().text(
                    cursor + vec2(icon_size / 2.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(icon_size),
                    text_color,
                );
                cursor.x += icon_size + icon_spacing;
            }

            // Draw text
            ui.painter().text(
                cursor
                    + vec2(
                        content_width / 2.0
                            - if self.icon.is_some() {
                                (icon_size + icon_spacing) / 2.0
                            } else {
                                0.0
                            },
                        0.0,
                    ),
                egui::Align2::CENTER_CENTER,
                &self.text,
                egui::FontId::proportional(14.0),
                text_color,
            );
        }

        response
    }
}

/// A large drop zone for file selection
pub struct DropZone {
    id: Id,
    text: String,
    subtext: String,
    accepts_multiple: bool,
}

impl DropZone {
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            text: "Drop files here".to_string(),
            subtext: "or click to browse".to_string(),
            accepts_multiple: true,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn subtext(mut self, subtext: impl Into<String>) -> Self {
        self.subtext = subtext.into();
        self
    }

    pub fn single_file(mut self) -> Self {
        self.accepts_multiple = false;
        self
    }
}

impl Widget for DropZone {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = ui
            .ctx()
            .data(|d| d.get_temp::<FluxTheme>(Id::NULL).unwrap_or_default());

        let desired_size = vec2(ui.available_width(), 160.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let hover_animation =
                ui.ctx()
                    .animate_bool_with_time(self.id.with("hover"), response.hovered(), 0.2);

            // Background
            let bg_color = theme
                .colors
                .panel_bg
                .lerp_to_gamma(theme.colors.primary.gamma_multiply(0.1), hover_animation);
            ui.painter()
                .rect_filled(rect, theme.rounding * 2.0, bg_color);

            // Dashed border
            let border_color = theme
                .colors
                .text_weak
                .gamma_multiply(0.5)
                .lerp_to_gamma(theme.colors.primary, hover_animation);

            // Draw dashed border manually
            let dash_len = 8.0;
            let gap_len = 4.0;
            let border_width = 2.0 + hover_animation;
            let corners = [
                rect.min,
                rect.min + vec2(rect.width(), 0.0),
                rect.max,
                rect.min + vec2(0.0, rect.height()),
            ];

            for i in 0..4 {
                let start = corners[i];
                let end = corners[(i + 1) % 4];
                let dir = (end - start).normalized();
                let len = (end - start).length();

                let mut pos = 0.0;
                while pos < len {
                    let dash_start = start + dir * pos;
                    let dash_end = start + dir * (pos + dash_len).min(len);

                    ui.painter()
                        .line_segment([dash_start, dash_end], (border_width, border_color));

                    pos += dash_len + gap_len;
                }
            }

            // Icon and text
            let center = rect.center();
            let icon_offset = 5.0 * hover_animation * (ui.ctx().frame_nr() as f32 * 0.1).sin();

            ui.painter().text(
                center - vec2(0.0, 30.0 + icon_offset),
                egui::Align2::CENTER_CENTER,
                regular::DOWNLOAD_SIMPLE,
                egui::FontId::proportional(48.0 + 4.0 * hover_animation),
                theme
                    .colors
                    .primary
                    .gamma_multiply(0.7 + 0.3 * hover_animation),
            );

            ui.painter().text(
                center + vec2(0.0, 20.0),
                egui::Align2::CENTER_CENTER,
                &self.text,
                egui::FontId::proportional(18.0),
                theme.colors.text,
            );

            ui.painter().text(
                center + vec2(0.0, 45.0),
                egui::Align2::CENTER_CENTER,
                &self.subtext,
                egui::FontId::proportional(14.0),
                theme.colors.text_weak,
            );
        }

        response
    }
}

/// Progress indicator with modern styling
pub struct FluxProgress {
    progress: f32,
    text: Option<String>,
    show_percentage: bool,
}

impl FluxProgress {
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            text: None,
            show_percentage: true,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }
}

impl Widget for FluxProgress {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = ui
            .ctx()
            .data(|d| d.get_temp::<FluxTheme>(Id::NULL).unwrap_or_default());

        let height = 24.0;
        let desired_size = vec2(ui.available_width(), height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            // Background
            ui.painter().rect_filled(
                rect,
                height / 2.0,
                theme.colors.panel_bg.gamma_multiply(0.5),
            );

            // Progress fill
            if self.progress > 0.0 {
                let progress_rect =
                    Rect::from_min_size(rect.min, vec2(rect.width() * self.progress, height));

                // Gradient effect
                let gradient_id = ui.make_persistent_id("progress_gradient");
                let animation = ui
                    .ctx()
                    .animate_value_with_time(gradient_id, self.progress, 0.3);

                ui.painter().rect_filled(
                    progress_rect,
                    height / 2.0,
                    theme.colors.primary.gamma_multiply(0.8 + 0.2 * animation),
                );

                // Shimmer effect
                let shimmer_offset = (ui.ctx().frame_nr() as f32 * 0.02) % 2.0;
                if self.progress < 1.0 && self.progress > 0.0 {
                    let shimmer_x = progress_rect.max.x - 40.0 + shimmer_offset * 40.0;
                    let shimmer_rect = Rect::from_min_max(
                        egui::pos2(shimmer_x.max(progress_rect.min.x), progress_rect.min.y),
                        egui::pos2(
                            (shimmer_x + 20.0).min(progress_rect.max.x),
                            progress_rect.max.y,
                        ),
                    );

                    ui.painter().rect_filled(
                        shimmer_rect,
                        height / 2.0,
                        Color32::from_white_alpha(30),
                    );
                }
            }

            // Text overlay
            let text = if let Some(custom_text) = self.text {
                custom_text
            } else if self.show_percentage {
                format!("{:.0}%", self.progress * 100.0)
            } else {
                String::new()
            };

            if !text.is_empty() {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(12.0),
                    theme.colors.text,
                );
            }
        }

        response
    }
}

/// Store theme in context for components to access
pub fn set_theme_in_context(ctx: &Context, theme: &FluxTheme) {
    ctx.data_mut(|d| d.insert_temp(Id::NULL, theme.clone()));
}
