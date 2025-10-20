//! Flux GUI - A modern graphical interface for the Flux archiver

use crossbeam_channel::Sender;
use std::path::PathBuf;

mod task;
mod views;
mod app;

use task::{ToUi, ProgressUpdate, TaskResult};
use app::FluxApp;

/// Handle pack task in background thread
pub fn handle_pack_task(
    inputs: Vec<PathBuf>,
    output: PathBuf,
    options: flux_lib::archive::PackOptions,
    ui_sender: &Sender<ToUi>,
) {
    // For simplicity, if multiple inputs, pack the first one
    // In a real implementation, you might want to create a temporary directory
    // and copy all files there first
    if let Some(input) = inputs.first() {
        // Use flux_lib::archive::pack_with_strategy function
        match flux_lib::archive::pack_with_strategy(input, &output, None, options) {
            Ok(_) => {
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
            }
            Err(e) => {
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            }
        }
    } else {
        let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("No input files".to_string())));
    }
}

/// Handle extract task in background thread
pub fn handle_extract_task(
    archive: PathBuf,
    output_dir: PathBuf,
    ui_sender: &Sender<ToUi>,
) {
    use flux_lib::archive::extractor::ExtractEntryOptions;
    
    // Create extractor
    let extractor = match flux_lib::archive::create_extractor(&archive) {
        Ok(ex) => ex,
        Err(e) => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            return;
        }
    };
    
    // Get entries to calculate total size
    let entries: Vec<_> = match extractor.entries(&archive) {
        Ok(entries) => {
            // Collect entries first to calculate total size
            entries.filter_map(|e| e.ok()).collect()
        }
        Err(e) => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            return;
        }
    };
    
    // Calculate total size
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let mut processed_size: u64 = 0;
    
    // Extract options
    let extract_options = ExtractEntryOptions {
        overwrite: true,
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
    };
    
    // Extract each entry
    for entry in &entries {
        // Send progress update
        let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: processed_size,
            total_bytes: total_size,
            current_file: entry.path.display().to_string(),
        }));
        
        // Extract the entry
        if let Err(e) = extractor.extract_entry(&archive, entry, &output_dir, extract_options.clone()) {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
                format!("Failed to extract {}: {}", entry.path.display(), e)
            )));
            return;
        }
        
        processed_size += entry.size;
    }
    
    // Send completion
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: total_size,
        total_bytes: total_size,
        current_file: "Extraction complete".to_string(),
    }));
    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

fn main() -> Result<(), eframe::Error> {
    // Set up eframe options with default values
    let options = eframe::NativeOptions::default();

    // Run the native app
    eframe::run_native(
        "Flux - File Archiver",
        options,
        Box::new(|cc| Ok(Box::new(FluxApp::new(cc)))),
    )
}