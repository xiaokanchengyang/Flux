//! Archive modification functionality for GUI

use super::FluxApp;
use flux_core::archive::modifier::{create_modifier, ModifyOptions};
use std::path::PathBuf;
use std::thread;
use tracing::{error, info};

impl FluxApp {
    /// Add files to an archive
    pub fn add_files_to_archive(&mut self, archive_path: PathBuf, files: Vec<PathBuf>) {
        info!("Adding {} files to archive: {:?}", files.len(), archive_path);
        
        let files_count = files.len();
        
        // Show progress
        self.status_text = format!("Adding {} files to archive...", files_count);
        self.toasts.info(format!("Adding {} files to archive", files_count));
        
        // Run modification in background thread
        thread::spawn(move || {
            let modifier = match create_modifier(&archive_path) {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to create modifier: {}", e);
                    return;
                }
            };
            
            let options = ModifyOptions {
                preserve_permissions: true,
                preserve_timestamps: true,
                follow_symlinks: false,
                compression_level: 6,
            };
            
            match modifier.add_files(&archive_path, &files, &options) {
                Ok(()) => {
                    info!("Successfully added {} files", files_count);
                }
                Err(e) => {
                    error!("Failed to add files: {}", e);
                }
            }
        });
    }
    
    /// Remove files from an archive
    pub fn remove_files_from_archive(&mut self, archive_path: PathBuf, patterns: Vec<String>) {
        info!("Removing files matching {} patterns from archive: {:?}", patterns.len(), archive_path);
        
        let patterns_count = patterns.len();
        
        // Show progress
        self.status_text = format!("Removing {} files from archive...", patterns_count);
        self.toasts.info(format!("Removing {} files from archive", patterns_count));
        
        // Run modification in background thread
        thread::spawn(move || {
            let modifier = match create_modifier(&archive_path) {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to create modifier: {}", e);
                    return;
                }
            };
            
            let options = ModifyOptions::default();
            
            match modifier.remove_files(&archive_path, &patterns, &options) {
                Ok(()) => {
                    info!("Successfully removed {} files", patterns_count);
                }
                Err(e) => {
                    error!("Failed to remove files: {}", e);
                }
            }
        });
        
        // Clear selection after removal
        if let Some(browser_state) = &mut self.browser_state {
            browser_state.selected.clear();
        }
    }
}