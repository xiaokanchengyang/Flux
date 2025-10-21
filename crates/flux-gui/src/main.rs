//! Flux GUI - A modern graphical interface for the Flux archiver

use crossbeam_channel::Sender;
use flux_core::utils::calculate_path_size;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::{debug, error, info, instrument, warn};

mod app;
mod components;
mod layout;
mod logging;
mod progress_tracker;
mod task;
mod theme;
mod views;

use app::FluxApp;
use progress_tracker::ProgressTracker;
use task::{ProgressUpdate, TaskResult, ToUi};

/// Handle pack task in background thread
#[instrument(skip(ui_sender, cancel_flag, options))]
pub fn handle_pack_task(
    inputs: Vec<PathBuf>,
    output: PathBuf,
    options: flux_core::archive::PackOptions,
    cancel_flag: Arc<AtomicBool>,
    ui_sender: &Sender<ToUi>,
) {
    if inputs.is_empty() {
        error!("No input files provided");
        let _ = ui_sender.send(ToUi::Log("Error: No input files provided".to_string()));
        let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
            "No input files".to_string(),
        )));
        return;
    }

    info!(files = inputs.len(), output = %output.display(), "Starting pack operation");
    let _ = ui_sender.send(ToUi::Log(format!(
        "Starting pack operation: {} files to {}",
        inputs.len(),
        output.display()
    )));

    // Calculate total size of all input files for progress tracking
    let mut total_size: u64 = 0;
    let mut file_sizes: Vec<(PathBuf, u64)> = Vec::new();

    for input in &inputs {
        let size = calculate_path_size(input);
        total_size += size;
        file_sizes.push((input.clone(), size));
        debug!(path = %input.display(), size_mb = size as f64 / (1024.0 * 1024.0), "Input file");
        let _ = ui_sender.send(ToUi::Log(format!(
            "Input: {} ({:.2} MB)",
            input.display(),
            size as f64 / (1024.0 * 1024.0)
        )));
    }

    info!(
        total_size_mb = total_size as f64 / (1024.0 * 1024.0),
        "Total size calculated"
    );
    let _ = ui_sender.send(ToUi::Log(format!(
        "Total size: {:.2} MB",
        total_size as f64 / (1024.0 * 1024.0)
    )));

    let mut processed_size: u64 = 0;
    let mut progress_tracker = ProgressTracker::new();

    // Send initial progress
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: total_size,
        current_file: "Preparing to pack...".to_string(),
        speed_bps: 0.0,
        eta_seconds: None,
    }));

    // Handle different compression formats
    match output.extension().and_then(|e| e.to_str()) {
        Some("zip") => {
            // For ZIP files, we'll pack each file individually
            info!("Creating ZIP archive");
            let _ = ui_sender.send(ToUi::Log("Creating ZIP archive...".to_string()));
            let mut ctx = PackContext {
                ui_sender,
                processed_size: &mut processed_size,
                total_size,
                follow_symlinks: options.follow_symlinks,
                cancel_flag: &cancel_flag,
                progress_tracker: &mut progress_tracker,
            };
            if let Err(e) = pack_multiple_zip(
                &inputs,
                &output,
                &mut ctx,
            ) {
                error!(error = %e, "Error creating ZIP");
                let _ = ui_sender.send(ToUi::Log(format!("Error creating ZIP: {}", e)));
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                return;
            }
        }
        Some(ext) => {
            // For tar-based formats, check if it's a compound extension
            let filename = output.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if filename.ends_with(".tar.gz")
                || filename.ends_with(".tar.zst")
                || filename.ends_with(".tar.xz")
                || filename.ends_with(".tar.br")
            {
                // Pack to compressed tar
                let mut ctx = PackContext {
                    ui_sender,
                    processed_size: &mut processed_size,
                    total_size,
                    follow_symlinks: options.follow_symlinks,
                    cancel_flag: &cancel_flag,
                    progress_tracker: &mut progress_tracker,
                };
                if let Err(e) = pack_multiple_tar_compressed(
                    &inputs,
                    &output,
                    options,
                    &mut ctx,
                ) {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                    return;
                }
            } else if ext == "tar" {
                // Pack to uncompressed tar
                let mut ctx = PackContext {
                    ui_sender,
                    processed_size: &mut processed_size,
                    total_size,
                    follow_symlinks: options.follow_symlinks,
                    cancel_flag: &cancel_flag,
                    progress_tracker: &mut progress_tracker,
                };
                if let Err(e) = pack_multiple_tar(
                    &inputs,
                    &output,
                    &mut ctx,
                ) {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                    return;
                }
            } else {
                // Fallback to single file packing for other formats
                if inputs.len() == 1 {
                    match flux_core::archive::pack_with_strategy(&inputs[0], &output, None, options)
                    {
                        Ok(_) => {
                            let (speed, _) = progress_tracker.update(total_size, total_size);
                            let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                                processed_bytes: total_size,
                                total_bytes: total_size,
                                current_file: "Packing complete".to_string(),
                                speed_bps: speed,
                                eta_seconds: None,
                            }));
                        }
                        Err(e) => {
                            let _ =
                                ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                            return;
                        }
                    }
                } else {
                    let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
                        "Multiple files can only be packed into tar or zip archives".to_string(),
                    )));
                    return;
                }
            }
        }
        None => {
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
                "Output file must have an extension".to_string(),
            )));
            return;
        }
    }

    // Get final file size
    if let Ok(metadata) = std::fs::metadata(&output) {
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        info!(size_mb = size_mb, "Archive created successfully");
        let _ = ui_sender.send(ToUi::Log(format!(
            "Archive created successfully: {:.2} MB",
            size_mb
        )));
    }

    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

/// Context for packing operations
struct PackContext<'a> {
    ui_sender: &'a Sender<ToUi>,
    processed_size: &'a mut u64,
    total_size: u64,
    follow_symlinks: bool,
    cancel_flag: &'a Arc<AtomicBool>,
    progress_tracker: &'a mut ProgressTracker,
}

/// Pack multiple files into a tar archive
#[instrument(skip(ctx))]
fn pack_multiple_tar(
    inputs: &[PathBuf],
    output: &PathBuf,
    ctx: &mut PackContext,
) -> Result<(), Box<dyn std::error::Error>> {
    use flux_core::archive::tar;

    // Find common base directory for relative paths
    let base_dir = find_common_base_dir(inputs);

    // Send progress updates periodically
    for input in inputs {
        // Check for cancellation
        if ctx.cancel_flag.load(Ordering::SeqCst) {
            let _ = ctx.ui_sender.send(ToUi::Finished(TaskResult::Cancelled));
            return Err("Operation cancelled".into());
        }

        let (speed, eta) = ctx.progress_tracker.update(*ctx.processed_size, ctx.total_size);
        let _ = ctx.ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: *ctx.processed_size,
            total_bytes: ctx.total_size,
            current_file: format!("Adding: {}", input.display()),
            speed_bps: speed,
            eta_seconds: eta,
        }));

        *ctx.processed_size += calculate_path_size(input);
    }

    // Pack all files
    tar::pack_multiple_files(inputs, output, base_dir.as_deref(), ctx.follow_symlinks)?;

    Ok(())
}

/// Pack multiple files into a compressed tar archive
#[instrument(skip(ctx, options))]
fn pack_multiple_tar_compressed(
    inputs: &[PathBuf],
    output: &PathBuf,
    options: flux_core::archive::PackOptions,
    ctx: &mut PackContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // First create uncompressed tar in memory or temp file
    let temp_tar = output.with_extension("tar.tmp");

    // Pack to temporary tar file
    pack_multiple_tar(
        inputs,
        &temp_tar,
        ctx,
    )?;

    // Now compress the tar file
    let (speed, eta) = ctx.progress_tracker.update(*ctx.processed_size, ctx.total_size);
    let _ = ctx.ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: *ctx.processed_size,
        total_bytes: ctx.total_size,
        current_file: "Compressing archive...".to_string(),
        speed_bps: speed,
        eta_seconds: eta,
    }));

    // Use pack_with_strategy to compress the tar file
    match flux_core::archive::pack_with_strategy(&temp_tar, output, None, options) {
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
#[instrument(skip(ctx))]
fn pack_multiple_zip(
    inputs: &[PathBuf],
    output: &PathBuf,
    ctx: &mut PackContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // For ZIP, we'll create a temporary directory and copy all files there,
    // then use flux_core to pack them
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Copy all input files to the temp directory
    for (idx, input) in inputs.iter().enumerate() {
        // Check for cancellation
        if ctx.cancel_flag.load(Ordering::SeqCst) {
            let _ = ctx.ui_sender.send(ToUi::Finished(TaskResult::Cancelled));
            return Err("Operation cancelled".into());
        }

        let (speed, eta) = ctx.progress_tracker.update(*ctx.processed_size, ctx.total_size);
        let _ = ctx.ui_sender.send(ToUi::Progress(ProgressUpdate {
            processed_bytes: *ctx.processed_size,
            total_bytes: ctx.total_size,
            current_file: format!("Preparing: {}", input.display()),
            speed_bps: speed,
            eta_seconds: eta,
        }));

        let dest_name = input
            .file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| std::ffi::OsString::from(format!("file_{}", idx)));
        let dest_path = temp_path.join(&dest_name);

        if input.is_file() {
            fs::copy(input, &dest_path)?;
        } else if input.is_dir() {
            copy_dir_recursive(input, &dest_path)?;
        }

        *ctx.processed_size += calculate_path_size(input);
    }

    // Now use flux_core to pack the temp directory
    let (speed, eta) = ctx.progress_tracker.update(*ctx.processed_size, ctx.total_size);
    let _ = ctx.ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: *ctx.processed_size,
        total_bytes: ctx.total_size,
        current_file: "Creating ZIP archive...".to_string(),
        speed_bps: speed,
        eta_seconds: eta,
    }));

    flux_core::archive::zip::pack_zip_with_options(temp_path, output, ctx.follow_symlinks)?;

    Ok(())
}

/// Recursively copy a directory
#[instrument]
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
#[instrument]
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
#[instrument(skip(ui_sender, cancel_flag))]
pub fn handle_extract_task(
    archive: PathBuf,
    output_dir: PathBuf,
    hoist: bool,
    cancel_flag: Arc<AtomicBool>,
    ui_sender: &Sender<ToUi>,
) {
    use flux_core::archive::extractor::ExtractEntryOptions;
    use std::time::Instant;

    // Send initial status
    info!(archive = %archive.display(), output_dir = %output_dir.display(), "Starting extraction");
    let _ = ui_sender.send(ToUi::Log(format!(
        "Starting extraction: {} to {}",
        archive.display(),
        output_dir.display()
    )));
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: 0,
        current_file: "Opening archive...".to_string(),
        speed_bps: 0.0,
        eta_seconds: None,
    }));

    // Create secure extractor
    let extractor = match flux_core::archive::create_secure_extractor(&archive) {
        Ok(ex) => ex,
        Err(e) => {
            error!(error = %e, "Failed to create extractor");
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
        speed_bps: 0.0,
        eta_seconds: None,
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
    let mut progress_tracker = ProgressTracker::new();

    // Send initial progress with total info
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: 0,
        total_bytes: total_size,
        current_file: format!("Extracting {} files...", total_count),
        speed_bps: 0.0,
        eta_seconds: None,
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
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(
                "Operation cancelled".to_string(),
            )));
            return;
        }

        processed_count += 1;

        // Send progress update if enough time has passed or for every file if there are few files
        if last_update.elapsed() > update_interval || total_count < 50 {
            let (speed, eta) = progress_tracker.update(processed_size, total_size);
            let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                processed_bytes: processed_size,
                total_bytes: total_size,
                current_file: format!(
                    "Extracting ({}/{}): {}",
                    processed_count,
                    total_count,
                    entry
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_else(|| entry.path.to_str().unwrap_or("..."))
                ),
                speed_bps: speed,
                eta_seconds: eta,
            }));
            last_update = Instant::now();
        }

        // Extract the entry
        if let Err(e) =
            extractor.extract_entry(&archive, entry, &output_dir, extract_options.clone())
        {
            error!(path = %entry.path.display(), error = %e, "Failed to extract file");
            let _ = ui_sender.send(ToUi::Log(format!(
                "Failed to extract {}: {}",
                entry.path.display(),
                e
            )));
            let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(format!(
                "Failed to extract {}: {}",
                entry.path.display(),
                e
            ))));
            return;
        }

        processed_size += entry.size;
    }

    // Perform directory hoisting if requested
    if hoist {
        info!("Checking for single directory to hoist...");
        let _ = ui_sender.send(ToUi::Log(
            "Checking for single directory to hoist...".to_string(),
        ));
        if let Err(e) = flux_core::archive::hoist_single_directory(&output_dir) {
            info!("Directory hoisting failed: {}", e);
            let _ = ui_sender.send(ToUi::Log(format!("Directory hoisting failed: {}", e)));
            // We don't fail the entire operation if hoisting fails
        } else {
            info!("Directory hoisting completed");
            let _ = ui_sender.send(ToUi::Log(
                "Directory hoisting completed successfully".to_string(),
            ));
        }
    }

    // Send completion
    let (speed, _) = progress_tracker.update(total_size, total_size);
    let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
        processed_bytes: total_size,
        total_bytes: total_size,
        current_file: format!("Successfully extracted {} files", total_count),
        speed_bps: speed,
        eta_seconds: None,
    }));
    info!(files = total_count, "Extraction completed");
    let _ = ui_sender.send(ToUi::Log(format!(
        "Extraction completed: {} files extracted",
        total_count
    )));
    let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
}

/// Handle sync/incremental backup task in background thread
#[instrument(skip(ui_sender, _cancel_flag, options))]
pub fn handle_sync_task(
    source_dir: PathBuf,
    target_archive: PathBuf,
    old_manifest: Option<PathBuf>,
    options: flux_core::archive::PackOptions,
    _cancel_flag: Arc<AtomicBool>,
    ui_sender: &Sender<ToUi>,
) {
    info!(
        source = %source_dir.display(),
        target = %target_archive.display(),
        incremental = old_manifest.is_some(),
        "Starting sync task"
    );

    let task_type = if old_manifest.is_some() {
        "incremental backup"
    } else {
        "full backup"
    };
    let _ = ui_sender.send(ToUi::Log(format!(
        "Starting {} from {} to {}",
        task_type,
        source_dir.display(),
        target_archive.display()
    )));

    // Check if we have an old manifest for incremental backup
    if let Some(old_manifest_path) = old_manifest {
        // Incremental backup
        match flux_core::archive::incremental::pack_incremental(
            &source_dir,
            &target_archive,
            &old_manifest_path,
            options,
        ) {
            Ok((_new_manifest_path, diff)) => {
                info!(
                    added = diff.added.len(),
                    modified = diff.modified.len(),
                    deleted = diff.deleted.len(),
                    "Incremental backup completed"
                );

                let _ = ui_sender.send(ToUi::Log(format!(
                    "Incremental backup completed: {} added, {} modified, {} deleted",
                    diff.added.len(),
                    diff.modified.len(),
                    diff.deleted.len()
                )));

                // Send final progress
                let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                    processed_bytes: 100,
                    total_bytes: 100,
                    current_file: format!("Backup complete - {} changes", diff.change_count()),
                    speed_bps: 0.0,
                    eta_seconds: None,
                }));

                let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
            }
            Err(e) => {
                error!(error = %e, "Incremental backup failed");
                let _ = ui_sender.send(ToUi::Log(format!("Incremental backup failed: {}", e)));
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            }
        }
    } else {
        // Full backup - first create the manifest
        info!("Creating initial manifest for full backup");
        let _ = ui_sender.send(ToUi::Log(
            "Creating manifest for source directory...".to_string(),
        ));

        match flux_core::manifest::Manifest::from_directory(&source_dir) {
            Ok(manifest) => {
                let file_count = manifest.file_count;
                let total_size = manifest.total_size;

                let _ = ui_sender.send(ToUi::Log(format!(
                    "Manifest created: {} files, {:.2} MB total",
                    file_count,
                    total_size as f64 / (1024.0 * 1024.0)
                )));

                // Create the full backup using regular pack
                let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                    processed_bytes: 0,
                    total_bytes: total_size,
                    current_file: "Creating full backup...".to_string(),
                    speed_bps: 0.0,
                    eta_seconds: None,
                }));

                match flux_core::archive::pack_with_strategy(
                    &source_dir,
                    &target_archive,
                    None,
                    options,
                ) {
                    Ok(_) => {
                        // Save the manifest
                        let manifest_path = target_archive.with_extension("manifest.json");
                        if let Err(e) = manifest.save(&manifest_path) {
                            warn!(error = %e, "Failed to save manifest");
                            let _ = ui_sender.send(ToUi::Log(format!(
                                "Warning: Failed to save manifest: {}",
                                e
                            )));
                        } else {
                            info!("Manifest saved to {:?}", manifest_path);
                            let _ = ui_sender.send(ToUi::Log(format!(
                                "Manifest saved to {}",
                                manifest_path.display()
                            )));
                        }

                        let _ = ui_sender.send(ToUi::Progress(ProgressUpdate {
                            processed_bytes: total_size,
                            total_bytes: total_size,
                            current_file: "Full backup complete".to_string(),
                            speed_bps: 0.0,
                            eta_seconds: None,
                        }));

                        let _ = ui_sender.send(ToUi::Finished(TaskResult::Success));
                    }
                    Err(e) => {
                        error!(error = %e, "Full backup failed");
                        let _ = ui_sender.send(ToUi::Log(format!("Full backup failed: {}", e)));
                        let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to create manifest");
                let _ = ui_sender.send(ToUi::Log(format!("Failed to create manifest: {}", e)));
                let _ = ui_sender.send(ToUi::Finished(TaskResult::Error(e.to_string())));
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing without GUI integration first (will be updated when app starts)
    crate::logging::init_tracing(None);

    info!("Starting Flux GUI application");

    // Set up eframe options with icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon = eframe::icon_data::from_png_bytes(icon_bytes).unwrap_or_else(|e| {
        tracing::warn!("Failed to load icon: {}", e);
        Default::default()
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(icon),
        centered: true,
        follow_system_theme: false, // We manage theme ourselves
        default_theme: eframe::Theme::Dark,
        persist_window: true, // Enable window state persistence
        ..Default::default()
    };

    // Run the native app
    eframe::run_native(
        "Flux - File Archiver",
        options,
        Box::new(|cc| Ok(Box::new(FluxApp::new(cc)))),
    )
}
