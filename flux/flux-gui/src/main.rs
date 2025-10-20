//! Flux GUI - A modern graphical interface for the Flux archiver

use crossbeam_channel::Sender;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

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
    cancel_flag: Arc<AtomicBool>,
    ui_sender: &Sender<ToUi>,
) {
    
    if inputs.is_empty() {
        let _ = ui_sender.send(ToUi::Log("Error: No input files provided".to_string()));
        let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("No input files".to_string())));
        return;
    }
    
    let _ = ui_sender.send(ToUi::Log(format!("Starting pack operation: {} files to {}", inputs.len(), output.display())));

    // Calculate total size of all input files for progress tracking
    let mut total_size: u64 = 0;
    let mut file_sizes: Vec<(PathBuf, u64)> = Vec::new();
    
    for input in &inputs {
        let size = calculate_path_size(input);
        total_size += size;
        file_sizes.push((input.clone(), size));
        let _ = ui_sender.send(ToUi::Log(format!("Input: {} ({:.2} MB)", input.display(), size as f64 / (1024.0 * 1024.0))));
    }
    
    let _ = ui_sender.send(ToUi::Log(format!("Total size: {:.2} MB", total_size as f64 / (1024.0 * 1024.0))));
    
    let mut processed_size: u64 = 0;
    
    // Send initial progress
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: total_size,
        current_file: "Preparing to pack...".to_string(),
    }));
    
    // Handle different compression formats
    match output.extension().and_then(|e| e.to_str()) {
        Some("zip") => {
            // For ZIP files, we'll pack each file individually
            let _ = ui_sender.send(ToUi::Log("Creating ZIP archive...".to_string()));
            if let Err(e) = pack_multiple_zip(&inputs, &output, ui_sender, &mut processed_size, total_size, options.follow_symlinks, &cancel_flag) {
                let _ = ui_sender.send(ToUi::Log(format!("Error creating ZIP: {}", e)));
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                return;
            }
        }
        Some(ext) => {
            // For tar-based formats, check if it's a compound extension
            let filename = output.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            if filename.ends_with(".tar.gz") || filename.ends_with(".tar.zst") || 
               filename.ends_with(".tar.xz") || filename.ends_with(".tar.br") {
                // Pack to compressed tar
                if let Err(e) = pack_multiple_tar_compressed(&inputs, &output, ui_sender, &mut processed_size, total_size, options, &cancel_flag) {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                    return;
                }
            } else if ext == "tar" {
                // Pack to uncompressed tar
                if let Err(e) = pack_multiple_tar(&inputs, &output, ui_sender, &mut processed_size, total_size, options.follow_symlinks, &cancel_flag) {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                    return;
                }
            } else {
                // Fallback to single file packing for other formats
                if inputs.len() == 1 {
                    match flux_lib::archive::pack_with_strategy(&inputs[0], &output, None, options) {
                        Ok(_) => {
                            let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                                processed_bytes: total_size,
                                total_bytes: total_size,
                                current_file: "Packing complete".to_string(),
                            }));
                        }
                        Err(e) => {
                            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                            return;
                        }
                    }
                } else {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("Multiple files can only be packed into tar or zip archives".to_string())));
                    return;
                }
            }
        }
        None => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("Output file must have an extension".to_string())));
            return;
        }
    }
    
    // Get final file size
    if let Ok(metadata) = std::fs::metadata(&output) {
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        let _ = ui_sender.send(ToUi::Log(format!("Archive created successfully: {:.2} MB", size_mb)));
    }
    
    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

/// Calculate the total size of a path (file or directory)
fn calculate_path_size(path: &PathBuf) -> u64 {
    if path.is_file() {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else if path.is_dir() {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                size += calculate_path_size(&entry.path());
            }
        }
        size
    } else {
        0
    }
}

/// Pack multiple files into a tar archive
fn pack_multiple_tar(
    inputs: &[PathBuf],
    output: &PathBuf,
    ui_sender: &Sender<ToUi>,
    processed_size: &mut u64,
    total_size: u64,
    follow_symlinks: bool,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use flux_lib::archive::tar;
    
    // Find common base directory for relative paths
    let base_dir = find_common_base_dir(inputs);
    
    // Send progress updates periodically
    for input in inputs {
        // Check for cancellation
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Cancelled));
            return Err("Operation cancelled".into());
        }
        
        let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: *processed_size,
            total_bytes: total_size,
            current_file: format!("Adding: {}", input.display()),
        }));
        
        *processed_size += calculate_path_size(input);
    }
    
    // Pack all files
    tar::pack_multiple_files(inputs, output, base_dir.as_deref(), follow_symlinks)?;
    
    Ok(())
}

/// Pack multiple files into a compressed tar archive
fn pack_multiple_tar_compressed(
    inputs: &[PathBuf],
    output: &PathBuf,
    ui_sender: &Sender<ToUi>,
    processed_size: &mut u64,
    total_size: u64,
    options: flux_lib::archive::PackOptions,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    
    // First create uncompressed tar in memory or temp file
    let temp_tar = output.with_extension("tar.tmp");
    
    // Pack to temporary tar file
    pack_multiple_tar(inputs, &temp_tar, ui_sender, processed_size, total_size, options.follow_symlinks, cancel_flag)?;
    
    // Now compress the tar file
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: *processed_size,
        total_bytes: total_size,
        current_file: "Compressing archive...".to_string(),
    }));
    
    // Use pack_with_strategy to compress the tar file
    match flux_lib::archive::pack_with_strategy(&temp_tar, output, None, options) {
        Ok(_) => {
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_tar);
            Ok(())
        }
        Err(e) => {
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_tar);
            Err(e.into())
        }
    }
}

/// Pack multiple files into a ZIP archive
fn pack_multiple_zip(
    inputs: &[PathBuf],
    output: &PathBuf,
    ui_sender: &Sender<ToUi>,
    processed_size: &mut u64,
    total_size: u64,
    follow_symlinks: bool,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // For ZIP, we'll create a temporary directory and copy all files there,
    // then use flux_lib to pack them
    use std::fs;
    use tempfile::TempDir;
    
    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    
    // Copy all input files to the temp directory
    for (idx, input) in inputs.iter().enumerate() {
        // Check for cancellation
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Cancelled));
            return Err("Operation cancelled".into());
        }
        
        let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: *processed_size,
            total_bytes: total_size,
            current_file: format!("Preparing: {}", input.display()),
        }));
        
        let dest_name = input.file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| std::ffi::OsString::from(format!("file_{}", idx)));
        let dest_path = temp_path.join(&dest_name);
        
        if input.is_file() {
            fs::copy(input, &dest_path)?;
        } else if input.is_dir() {
            copy_dir_recursive(input, &dest_path)?;
        }
        
        *processed_size += calculate_path_size(input);
    }
    
    // Now use flux_lib to pack the temp directory
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: *processed_size,
        total_bytes: total_size,
        current_file: "Creating ZIP archive...".to_string(),
    }));
    
    flux_lib::archive::zip::pack_zip_with_options(temp_path, output, follow_symlinks)?;
    
    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// Find the common base directory for a set of paths
fn find_common_base_dir(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    
    // If all paths have the same parent, use that as base
    let first_parent = paths[0].parent();
    if let Some(parent) = first_parent {
        if paths.iter().all(|p| p.parent() == first_parent) {
            return Some(parent.to_path_buf());
        }
    }
    
    None
}

/// Handle extract task in background thread
pub fn handle_extract_task(
    archive: PathBuf,
    output_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    ui_sender: &Sender<ToUi>,
) {
    use flux_lib::archive::extractor::ExtractEntryOptions;
    use std::time::Instant;
    
    // Send initial status
    let _ = ui_sender.send(ToUi::Log(format!("Starting extraction: {} to {}", archive.display(), output_dir.display())));
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: 0,
        current_file: "Opening archive...".to_string(),
    }));
    
    // Create extractor
    let extractor = match flux_lib::archive::create_extractor(&archive) {
        Ok(ex) => ex,
        Err(e) => {
            let _ = ui_sender.send(ToUi::Log(format!("Failed to create extractor: {}", e)));
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            return;
        }
    };
    
    // Get entries to calculate total size
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: 0,
        current_file: "Reading archive contents...".to_string(),
    }));
    
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
    
    // Calculate total size and count
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let total_count = entries.len();
    let mut processed_size: u64 = 0;
    let mut processed_count = 0;
    
    // Send initial progress with total info
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: total_size,
        current_file: format!("Extracting {} files...", total_count),
    }));
    
    // Extract options
    let extract_options = ExtractEntryOptions {
        overwrite: true,
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
    };
    
    // Track time for periodic updates
    let mut last_update = Instant::now();
    let update_interval = std::time::Duration::from_millis(100); // Update every 100ms
    
    // Extract each entry
    for entry in &entries {
        // Check for cancellation
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error("Operation cancelled".to_string())));
            return;
        }
        
        processed_count += 1;
        
        // Send progress update if enough time has passed or for every file if there are few files
        if last_update.elapsed() > update_interval || total_count < 50 {
            let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                processed_bytes: processed_size,
                total_bytes: total_size,
                current_file: format!(
                    "Extracting ({}/{}): {}",
                    processed_count,
                    total_count,
                    entry.path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_else(|| entry.path.to_str().unwrap_or("..."))
                ),
            }));
            last_update = Instant::now();
        }
        
        // Extract the entry
        if let Err(e) = extractor.extract_entry(&archive, entry, &output_dir, extract_options.clone()) {
            let _ = ui_sender.send(ToUi::Log(format!("Failed to extract {}: {}", entry.path.display(), e)));
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
        current_file: format!("Successfully extracted {} files", total_count),
    }));
    let _ = ui_sender.send(ToUi::Log(format!("Extraction completed: {} files extracted", total_count)));
    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    
    log::info!("Starting Flux GUI application");
    
    // Set up eframe options with icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon = eframe::icon_data::from_png_bytes(icon_bytes)
        .unwrap_or_else(|e| {
            log::warn!("Failed to load icon: {}", e);
            Default::default()
        });
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(icon),
        ..Default::default()
    };

    // Run the native app
    eframe::run_native(
        "Flux - File Archiver",
        options,
        Box::new(|cc| Ok(Box::new(FluxApp::new(cc)))),
    )
}