//! Extract command implementation with interactive support

use anyhow::Result;
use dialoguer::Select;
use flux_lib::archive::extractor::{ConflictAction, ConflictHandler, ExtractEntryOptions};
use flux_lib::archive::{create_extractor, ExtractOptions};
use flux_lib::Error as FluxError;
use indicatif::{ProgressBar, ProgressStyle};
// use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Interactive conflict handler for CLI
pub struct InteractiveConflictHandler {
    global_action: Option<ConflictAction>,
}

impl InteractiveConflictHandler {
    pub fn new() -> Self {
        Self {
            global_action: None,
        }
    }
}

impl ConflictHandler for InteractiveConflictHandler {
    fn handle_conflict(
        &mut self,
        entry: &flux_lib::archive::extractor::ArchiveEntry,
        existing_path: &Path,
    ) -> ConflictAction {
        // If we have a global action set, use it
        if let Some(action) = self.global_action {
            return action;
        }

        // Build the prompt
        let prompt = format!(
            "File already exists: {}\nSize: {} bytes",
            existing_path.display(),
            entry.size
        );

        let options = vec![
            "Overwrite",
            "Skip",
            "Rename",
            "Overwrite All",
            "Skip All",
            "Abort",
        ];

        let selection = Select::new()
            .with_prompt(&prompt)
            .items(&options)
            .default(1) // Default to Skip
            .interact()
            .unwrap_or(1);

        let action = match selection {
            0 => ConflictAction::Overwrite,
            1 => ConflictAction::Skip,
            2 => ConflictAction::Rename,
            3 => {
                self.global_action = Some(ConflictAction::OverwriteAll);
                ConflictAction::Overwrite
            }
            4 => {
                self.global_action = Some(ConflictAction::SkipAll);
                ConflictAction::Skip
            }
            5 => ConflictAction::Abort,
            _ => ConflictAction::Skip,
        };

        action
    }
}

/// Extract with interactive conflict handling
pub fn extract_interactive(
    archive: &Path,
    output_dir: &Path,
    strip_components: Option<usize>,
    show_progress: bool,
) -> Result<()> {
    // Check if it's a 7z archive (which doesn't support interactive extraction)
    let ext = archive
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    if ext == "7z" {
        warn!("Interactive extraction is not supported for 7z archives. Using standard extraction.");
        let options = ExtractOptions {
            overwrite: false,
            skip: true,
            rename: false,
            strip_components,
        };
        return extract_with_options(archive, output_dir, options, show_progress);
    }

    // Create the extractor
    let extractor = create_extractor(archive)?;

    // Get all entries first to show progress
    let entries: Vec<_> = extractor
        .entries(archive)?
        .collect::<Result<Vec<_>, _>>()?;

    let total_entries = entries.len();
    info!("Found {} entries in archive", total_entries);

    // Create progress bar if requested
    let progress = if show_progress {
        let pb = ProgressBar::new(total_entries as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    // Create conflict handler
    let mut conflict_handler = InteractiveConflictHandler::new();

    // Statistics
    let mut extracted = 0;
    let mut skipped = 0;
    let mut failed = 0;

    // Process each entry
    for (idx, entry) in entries.iter().enumerate() {
        if let Some(ref pb) = progress {
            pb.set_position(idx as u64);
            pb.set_message(format!("{}", entry.path.display()));
        }

        // Calculate destination path
        let mut dest_path = entry.path.clone();
        
        // Strip components if requested
        if let Some(strip) = strip_components {
            let components: Vec<_> = dest_path.components().collect();
            if components.len() > strip {
                dest_path = components[strip..].iter().collect();
            } else {
                debug!("Skipping entry with insufficient path components: {:?}", entry.path);
                skipped += 1;
                continue;
            }
        }

        let full_dest = output_dir.join(&dest_path);

        // Check if destination exists
        let action = if full_dest.exists() {
            conflict_handler.handle_conflict(entry, &full_dest)
        } else {
            ConflictAction::Overwrite // No conflict, proceed
        };

        match action {
            ConflictAction::Abort => {
                warn!("Extraction aborted by user");
                if failed > 0 {
                    return Err(FluxError::PartialFailure { count: failed as u32 }.into());
                }
                return Ok(());
            }
            ConflictAction::Skip | ConflictAction::SkipAll => {
                debug!("Skipping: {:?}", dest_path);
                skipped += 1;
                continue;
            }
            ConflictAction::Rename => {
                // Find a non-conflicting name
                let mut counter = 1;
                let mut renamed_path = full_dest.clone();
                while renamed_path.exists() {
                    let file_stem = full_dest
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("file");
                    let extension = full_dest
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|e| format!(".{}", e))
                        .unwrap_or_default();
                    let new_name = format!("{}_{}{}", file_stem, counter, extension);
                    renamed_path = full_dest.with_file_name(new_name);
                    counter += 1;
                }

                // Extract to renamed path
                match extractor.extract_entry(
                    archive,
                    entry,
                    &renamed_path.parent().unwrap_or(output_dir),
                    ExtractEntryOptions {
                        overwrite: true,
                        preserve_permissions: true,
                        preserve_timestamps: true,
                        follow_symlinks: false,
                    },
                ) {
                    Ok(_) => {
                        info!("Extracted (renamed): {:?} -> {:?}", entry.path, renamed_path);
                        extracted += 1;
                    }
                    Err(e) => {
                        warn!("Failed to extract {:?}: {}", entry.path, e);
                        failed += 1;
                    }
                }
            }
            ConflictAction::Overwrite | ConflictAction::OverwriteAll => {
                // Extract with overwrite
                match extractor.extract_entry(
                    archive,
                    entry,
                    output_dir,
                    ExtractEntryOptions {
                        overwrite: true,
                        preserve_permissions: true,
                        preserve_timestamps: true,
                        follow_symlinks: false,
                    },
                ) {
                    Ok(_) => {
                        debug!("Extracted: {:?}", dest_path);
                        extracted += 1;
                    }
                    Err(e) => {
                        warn!("Failed to extract {:?}: {}", entry.path, e);
                        failed += 1;
                    }
                }
            }
        }
    }

    if let Some(pb) = progress {
        pb.finish_with_message("Extraction complete");
    }

    info!(
        "Extraction summary: {} extracted, {} skipped, {} failed",
        extracted, skipped, failed
    );

    if failed > 0 {
        Err(FluxError::PartialFailure { count: failed as u32 }.into())
    } else {
        Ok(())
    }
}

/// Extract with non-interactive options
pub fn extract_with_options(
    archive: &Path,
    output_dir: &Path,
    options: ExtractOptions,
    _show_progress: bool,
) -> Result<()> {
    // For backward compatibility, use the old extraction method
    flux_lib::archive::extract_with_options(archive, output_dir, options)?;
    Ok(())
}