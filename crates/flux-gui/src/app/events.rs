//! Event handling for the Flux GUI application

use super::{AppView, FluxApp};
use crate::task::TaskCommand;
use crate::views::BrowserState;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::{debug, info, warn};

impl FluxApp {
    /// Analyze dropped files and switch view accordingly
    pub(super) fn analyze_dropped_files(&mut self, files: Vec<PathBuf>) {
        if files.is_empty() {
            return;
        }

        // Check if it's a single archive file
        if files.len() == 1 {
            let file = &files[0];
            let file_name = file.file_name().map(|n| n.to_string_lossy().to_string());

            // Check for common archive extensions
            if let Some(ext) = file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(
                    ext_str.as_str(),
                    "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "br"
                ) {
                    // Switch to browser view to explore the archive
                    if let Err(e) = self.open_archive_browser(file.clone()) {
                        // Fall back to extraction view if browser fails
                        warn!("Failed to open archive browser: {}", e);
                        self.view = AppView::Extracting;
                        self.input_files = files;
                        info!(file = ?file_name, "Ready to extract archive");
                        self.toasts.info(format!(
                            "Ready to extract: {}",
                            file_name.as_deref().unwrap_or("archive")
                        ));
                    }
                    return;
                }
            }

            // Also check for compound extensions like .tar.gz
            if let Some(name) = &file_name {
                let name_lower = name.to_lowercase();
                if name_lower.ends_with(".tar.gz")
                    || name_lower.ends_with(".tar.zst")
                    || name_lower.ends_with(".tar.xz")
                    || name_lower.ends_with(".tar.br")
                {
                    // Switch to browser view to explore the archive
                    if let Err(e) = self.open_archive_browser(file.clone()) {
                        // Fall back to extraction view if browser fails
                        warn!("Failed to open archive browser: {}", e);
                        self.view = AppView::Extracting;
                        self.input_files = files;
                        info!(file = name, "Ready to extract compressed tar archive");
                        self.toasts.info(format!("Ready to extract: {}", name));
                    }
                    return;
                }
            }
        }

        // Multiple files, single non-archive file, or directories - switch to packing view
        self.view = AppView::Packing;
        let count = files.len();
        self.input_files = files;
        info!(files = count, "Ready to pack files");
        self.toasts.info(format!(
            "Ready to pack {} file{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    /// Cancel the current task
    pub(super) fn cancel_task(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
            info!("Cancelling current task");
            self.toasts.info("Cancelling task...");
        }
    }

    /// Reset to welcome view
    pub(super) fn reset_to_welcome(&mut self) {
        debug!("Resetting to welcome view");
        self.view = AppView::Welcome;
        self.input_files.clear();
        self.output_path = None;
        self.current_progress = 0.0;
        self.status_text = "Ready".to_string();
        self.cancel_flag = None;
    }

    /// Start the task based on current view and inputs
    pub(super) fn start_task(&mut self) {
        match self.view {
            AppView::Packing => {
                if let Some(output) = &self.output_path {
                    // Validate output path
                    if let Some(parent) = output.parent() {
                        if !parent.exists() {
                            warn!("Output directory does not exist: {:?}", parent);
                            self.toasts.error("Output directory does not exist");
                            return;
                        }
                    }

                    // Determine the algorithm based on the selected compression format
                    let algorithm = match self.compression_format.as_str() {
                        "tar.gz" => Some("gz".to_string()),
                        "tar.zst" => Some("zst".to_string()),
                        "tar.xz" => Some("xz".to_string()),
                        "zip" => Some("zip".to_string()),
                        _ => None,
                    };

                    let options = flux_core::archive::PackOptions {
                        smart: false, // Disable smart mode since user explicitly selected format
                        algorithm,
                        level: None,
                        threads: None,
                        force_compress: false,
                        follow_symlinks: false,
                    };

                    // Create cancel flag
                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());

                    let command = TaskCommand::Pack {
                        inputs: self.input_files.clone(),
                        output: output.clone(),
                        options,
                        cancel_flag,
                    };

                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting pack operation...".to_string();
                        info!("Starting pack operation");
                        self.toasts.info("Starting to create archive...");
                    } else {
                        warn!("Failed to send pack command to background thread");
                        self.toasts
                            .error("Failed to start task: background thread not responding");
                    }
                } else {
                    warn!("No output path selected");
                    self.toasts.error("Please select an output path first");
                }
            }
            AppView::Extracting => {
                if let (Some(archive), Some(output_dir)) =
                    (self.input_files.first(), &self.output_path)
                {
                    // Validate archive exists
                    if !archive.exists() {
                        warn!("Archive file not found: {:?}", archive);
                        self.toasts.error("Archive file not found");
                        return;
                    }

                    // Validate output directory exists
                    if !output_dir.exists() {
                        warn!("Output directory does not exist: {:?}", output_dir);
                        self.toasts.error("Output directory does not exist");
                        return;
                    }

                    // Create cancel flag
                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());

                    let command = TaskCommand::Extract {
                        archive: archive.clone(),
                        output_dir: output_dir.clone(),
                        hoist: self.extract_hoist,
                        cancel_flag,
                    };

                    if self.task_sender.send(command).is_ok() {
                        self.is_busy = true;
                        self.current_progress = 0.0;
                        self.status_text = "Starting extraction...".to_string();
                        info!("Starting extraction operation");
                        self.toasts.info("Starting extraction...");
                    } else {
                        warn!("Failed to send extract command to background thread");
                        self.toasts
                            .error("Failed to start task: background thread not responding");
                    }
                } else {
                    warn!("Missing archive or output directory");
                    self.toasts
                        .error("Please select an archive and output directory first");
                }
            }
            AppView::Welcome => {}
            AppView::Syncing => {
                // Should use start_sync_task instead
                warn!("start_task called in Syncing view, use start_sync_task instead");
            }
            AppView::Browsing => {
                // Browser view doesn't use start_task
                warn!("start_task called in Browsing view");
            }
        }
    }

    /// Start the sync/incremental backup task
    pub(super) fn start_sync_task(&mut self) {
        if let (Some(source_dir), Some(target_archive)) =
            (&self.sync_source_dir, &self.sync_target_archive)
        {
            // Validate source directory exists
            if !source_dir.exists() {
                warn!("Source directory does not exist: {:?}", source_dir);
                self.toasts.error("Source directory does not exist");
                return;
            }

            // Validate output directory exists
            if let Some(parent) = target_archive.parent() {
                if !parent.exists() {
                    warn!("Target directory does not exist: {:?}", parent);
                    self.toasts.error("Target directory does not exist");
                    return;
                }
            }

            // Determine compression algorithm from file extension
            let filename = target_archive
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let algorithm = if filename.ends_with(".tar.gz") {
                Some("gz".to_string())
            } else if filename.ends_with(".tar.zst") {
                Some("zst".to_string())
            } else if filename.ends_with(".tar.xz") {
                Some("xz".to_string())
            } else {
                None
            };

            let options = flux_core::archive::PackOptions {
                smart: false,
                algorithm,
                level: Some(6), // Default compression level
                threads: None,
                force_compress: false,
                follow_symlinks: false,
            };

            // Create cancel flag
            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.cancel_flag = Some(cancel_flag.clone());

            let command = TaskCommand::Sync {
                source_dir: source_dir.clone(),
                target_archive: target_archive.clone(),
                old_manifest: self.sync_manifest_path.clone(),
                options,
                cancel_flag,
            };

            if self.task_sender.send(command).is_ok() {
                self.is_busy = true;
                self.current_progress = 0.0;

                let task_type = if self.sync_manifest_path.is_some() {
                    "incremental backup"
                } else {
                    "full backup"
                };

                self.status_text = format!("Starting {}...", task_type);
                info!("Starting sync operation: {}", task_type);
                self.toasts.info(format!("Starting {}...", task_type));
            } else {
                warn!("Failed to send sync command to background thread");
                self.toasts
                    .error("Failed to start task: background thread not responding");
            }
        } else {
            warn!("Missing source directory or target archive");
            self.toasts
                .error("Please select source directory and target archive first");
        }
    }

    /// Open the archive browser for a given archive file
    pub(super) fn open_archive_browser(&mut self, archive_path: PathBuf) -> Result<(), String> {
        use flux_core::archive;

        // Create an extractor for the archive
        let extractor = archive::create_extractor(&archive_path)
            .map_err(|e| format!("Failed to open archive: {}", e))?;

        // Get all entries from the archive
        let entries_iter = extractor
            .entries(&archive_path)
            .map_err(|e| format!("Failed to read archive entries: {}", e))?;

        // Collect entries into a vector
        let mut entries = Vec::new();
        for entry_result in entries_iter {
            match entry_result {
                Ok(entry) => entries.push(entry),
                Err(e) => warn!("Failed to read entry: {}", e),
            }
        }

        // Create browser state
        let browser_state = BrowserState::new(archive_path.clone(), entries);

        // Switch to browser view
        self.view = AppView::Browsing;
        self.browser_state = Some(browser_state);

        info!("Opened archive browser for: {:?}", archive_path);
        self.toasts.info(format!(
            "Browsing: {}",
            archive_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("archive")
        ));

        Ok(())
    }

    /// Extract selected entries from an archive
    pub(super) fn extract_selected_entries(
        &mut self,
        entries: Vec<flux_core::archive::extractor::ArchiveEntry>,
        _archive_path: PathBuf,
        _output_dir: PathBuf,
    ) {
        self.toasts
            .info(format!("Extracting {} selected items...", entries.len()));

        // Store the paths of selected entries
        let entry_count = entries.len();
        let entry_names: Vec<String> = entries
            .iter()
            .map(|e| e.path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string())
            .collect();

        // For now, show a detailed message about what would be extracted
        let message = if entry_count <= 3 {
            format!("Would extract: {}", entry_names.join(", "))
        } else {
            format!("Would extract {} items including: {}, ...", 
                entry_count, 
                entry_names.iter().take(3).cloned().collect::<Vec<_>>().join(", "))
        };
        
        self.toasts.info(message);
        self.toasts.warning("Partial extraction feature is coming soon!");
        
        // TODO: Implement partial extraction in flux-core
        // This requires extending the extractor API to support extracting specific entries
    }
}
