//! Theme system for Flux GUI

use egui::{Color32, FontId, Rounding, Stroke, Style, TextStyle, Visuals};
use std::collections::BTreeMap;

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

/// Color scheme for the application
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Primary color (for buttons, highlights)
    pub primary: Color32,
    /// Primary hover color
    pub primary_hover: Color32,
    /// Secondary color
    pub secondary: Color32,
    /// Background color for windows
    pub background: Color32,
    /// Background color for panels
    pub panel_bg: Color32,
    /// Text color
    pub text: Color32,
    /// Weak text color (for labels, hints)
    pub text_weak: Color32,
    /// Hyperlink color
    pub hyperlink: Color32,
    /// Success color
    pub success: Color32,
    /// Warning color
    pub warning: Color32,
    /// Error color
    pub error: Color32,
}

/// Flux application theme
#[derive(Debug, Clone)]
pub struct FluxTheme {
    pub mode: ThemeMode,
    pub colors: ColorScheme,
    pub rounding: f32,
    pub spacing: f32,
}

impl FluxTheme {
    /// Check if the theme is in dark mode
    pub fn is_dark_mode(&self) -> bool {
        matches!(self.mode, ThemeMode::Dark)
    }

    /// Create a new light theme
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            colors: ColorScheme {
                primary: Color32::from_rgb(0, 120, 212),
                primary_hover: Color32::from_rgb(0, 140, 232),
                secondary: Color32::from_rgb(100, 100, 100),
                background: Color32::from_gray(248),
                panel_bg: Color32::from_gray(240),
                text: Color32::from_gray(20),
                text_weak: Color32::from_gray(100),
                hyperlink: Color32::from_rgb(0, 120, 212),
                success: Color32::from_rgb(46, 160, 67),
                warning: Color32::from_rgb(255, 193, 7),
                error: Color32::from_rgb(220, 53, 69),
            },
            rounding: 4.0,
            spacing: 8.0,
        }
    }

    /// Create a new dark theme
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            colors: ColorScheme {
                primary: Color32::from_rgb(88, 101, 242),      // Modern purple-blue
                primary_hover: Color32::from_rgb(110, 121, 245),
                secondary: Color32::from_gray(150),
                background: Color32::from_rgb(23, 25, 35),     // Darker, more modern
                panel_bg: Color32::from_rgb(30, 33, 45),       // Slightly lighter panel
                text: Color32::from_gray(235),                 // Brighter text
                text_weak: Color32::from_gray(160),
                hyperlink: Color32::from_rgb(139, 148, 255),
                success: Color32::from_rgb(67, 181, 129),      // Modern green
                warning: Color32::from_rgb(250, 176, 5),       // Modern amber
                error: Color32::from_rgb(240, 71, 71),         // Modern red
            },
            rounding: 8.0,  // More rounded for modern look
            spacing: 10.0,  // More spacious
        }
    }

    /// Toggle between light and dark mode
    pub fn toggle(&mut self) {
        *self = match self.mode {
            ThemeMode::Light => Self::dark(),
            ThemeMode::Dark => Self::light(),
        };
    }

    /// Apply the theme to egui context
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_style(self.get_style());
    }

    /// Get egui style from theme
    pub fn get_style(&self) -> Style {
        let mut style = Style::default();

        // Visuals
        let mut visuals = if self.mode == ThemeMode::Dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };

        // Override colors
        visuals.hyperlink_color = self.colors.hyperlink;
        visuals.error_fg_color = self.colors.error;
        visuals.warn_fg_color = self.colors.warning;

        // Window styling
        visuals.window_fill = self.colors.background;
        visuals.panel_fill = self.colors.panel_bg;
        visuals.window_stroke = Stroke::new(1.0, self.colors.text_weak);

        // Rounding
        visuals.window_rounding = Rounding::same(self.rounding);
        visuals.menu_rounding = Rounding::same(self.rounding);
        // Button rounding is set per-widget in egui 0.28

        // Selection colors
        visuals.selection.bg_fill = self.colors.primary.linear_multiply(0.3);

        // Widget visuals
        visuals.widgets.noninteractive.bg_fill = self.colors.panel_bg;
        visuals.widgets.noninteractive.weak_bg_fill = self.colors.panel_bg;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.colors.text);

        visuals.widgets.inactive.bg_fill = self.colors.panel_bg.linear_multiply(0.9);
        visuals.widgets.inactive.weak_bg_fill = self.colors.panel_bg.linear_multiply(0.95);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.colors.text_weak);

        visuals.widgets.hovered.bg_fill = self.colors.primary.linear_multiply(0.1);
        visuals.widgets.hovered.weak_bg_fill = self.colors.primary.linear_multiply(0.05);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.colors.primary);

        visuals.widgets.active.bg_fill = self.colors.primary.linear_multiply(0.2);
        visuals.widgets.active.weak_bg_fill = self.colors.primary.linear_multiply(0.1);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.colors.primary);

        style.visuals = visuals;

        // Spacing
        style.spacing.item_spacing = egui::vec2(self.spacing, self.spacing);
        style.spacing.button_padding = egui::vec2(self.spacing, self.spacing / 2.0);
        style.spacing.menu_margin = egui::Margin::same(self.spacing);
        style.spacing.indent = self.spacing * 2.5;

        // Text styles with custom fonts if available
        let mut text_styles = BTreeMap::new();

        // Base font sizes
        let small_size = 12.0;
        let body_size = 14.0;
        let button_size = 14.0;
        let heading_size = 20.0;
        let monospace_size = 13.0;

        text_styles.insert(TextStyle::Small, FontId::proportional(small_size));
        text_styles.insert(TextStyle::Body, FontId::proportional(body_size));
        text_styles.insert(TextStyle::Button, FontId::proportional(button_size));
        text_styles.insert(TextStyle::Heading, FontId::proportional(heading_size));
        text_styles.insert(TextStyle::Monospace, FontId::monospace(monospace_size));

        style.text_styles = text_styles;

        style
    }

    /// Create a primary button with theme styling
    pub fn primary_button(&self, text: impl Into<egui::WidgetText>) -> egui::Button {
        egui::Button::new(text)
            .fill(self.colors.primary)
            .rounding(Rounding::same(self.rounding))
    }

    /// Create a secondary button with theme styling
    pub fn secondary_button(&self, text: impl Into<egui::WidgetText>) -> egui::Button {
        egui::Button::new(text)
            .fill(self.colors.secondary)
            .rounding(Rounding::same(self.rounding))
    }

    /// Style a button widget (modifies UI visuals temporarily)
    pub fn style_button(&self, ui: &mut egui::Ui, is_primary: bool) {
        let button_color = if is_primary {
            self.colors.primary
        } else {
            self.colors.secondary
        };

        ui.visuals_mut().widgets.inactive.bg_fill = button_color;
        ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ui.visuals_mut().widgets.hovered.bg_fill = button_color.linear_multiply(1.2);
        ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ui.visuals_mut().widgets.active.bg_fill = button_color.linear_multiply(0.8);
        ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    }
}

impl Default for FluxTheme {
    fn default() -> Self {
        // Default to dark theme
        Self::dark()
    }
}
