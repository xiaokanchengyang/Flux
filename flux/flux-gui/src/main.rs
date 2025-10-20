//! Flux GUI - A modern graphical interface for the Flux archiver

use eframe::egui;
use std::path::PathBuf;
use tracing::{error, info};

mod app;
mod state;
mod worker;

use app::FluxApp;

fn main() -> Result<(), eframe::Error> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    info!("Starting Flux GUI");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            // Icon will be added later
            // .with_icon(
            //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon.png")[..])
            //         .unwrap_or_default(),
            // ),
        ..Default::default()
    };

    eframe::run_native(
        "Flux - File Archiver",
        options,
        Box::new(|cc| {
            // Configure fonts and visuals
            configure_fonts(&cc.egui_ctx);
            configure_visuals(&cc.egui_ctx);
            
            Ok(Box::new(FluxApp::new(cc)))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    // Custom fonts will be added later
    // fonts.font_data.insert(
    //     "flux_icons".to_owned(),
    //     egui::FontData::from_static(include_bytes!("../assets/icons.ttf")),
    // );
    
    // Configure font families
    // fonts
    //     .families
    //     .entry(egui::FontFamily::Proportional)
    //     .or_default()
    //     .insert(0, "flux_icons".to_owned());
        
    ctx.set_fonts(fonts);
}

fn configure_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    
    // Customize colors for a modern look
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(35, 35, 40);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 50);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 55, 60);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(65, 65, 70);
    
    // Set accent color
    visuals.selection.bg_fill = egui::Color32::from_rgb(100, 150, 255);
    
    ctx.set_visuals(visuals);
}