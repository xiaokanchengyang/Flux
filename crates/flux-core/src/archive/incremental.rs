//! Incremental backup support

use crate::archive::{tar, PackOptions};
use crate::manifest::{Manifest, ManifestDiff};
use crate::Result;
use std::path::{Path, PathBuf};
use tracing::info;

/// Pack files incrementally based on manifest
pub fn pack_incremental<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
    input_dir: P,
    output: Q,
    old_manifest_path: R,
    options: PackOptions,
) -> Result<(PathBuf, ManifestDiff)> {
    let input_dir = input_dir.as_ref();
    let output = output.as_ref();
    let old_manifest_path = old_manifest_path.as_ref();

    info!("Starting incremental backup from {:?}", input_dir);

    // Load old manifest
    let old_manifest = Manifest::load(old_manifest_path)?;

    // Create new manifest
    let new_manifest = Manifest::from_directory(input_dir)?;

    // Calculate differences
    let diff = old_manifest.diff(&new_manifest);

    info!(
        "Incremental backup: {} added, {} modified, {} deleted",
        diff.added.len(),
        diff.modified.len(),
        diff.deleted.len()
    );

    if !diff.has_changes() {
        info!("No changes detected, skipping backup");
        return Ok((PathBuf::new(), diff));
    }

    // Create list of files to pack
    let mut files_to_pack = Vec::new();

    // Add new and modified files
    for path in &diff.added {
        files_to_pack.push(input_dir.join(path));
    }
    for path in &diff.modified {
        files_to_pack.push(input_dir.join(path));
    }

    // Also include manifest of deleted files for restoration purposes
    if !diff.deleted.is_empty() {
        // Create a deleted files list
        let deleted_list_path = output.with_extension("deleted.txt");
        let deleted_content = diff
            .deleted
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&deleted_list_path, deleted_content)?;
        info!("Created deleted files list at {:?}", deleted_list_path);
    }

    // Pack the changed files
    info!("Packing {} changed files", files_to_pack.len());

    // For incremental backup, we'll create a tar archive with the changed files
    // The tar will preserve the directory structure
    tar::pack_multiple_files(
        &files_to_pack,
        output,
        Some(input_dir),
        options.follow_symlinks,
    )?;

    // Save new manifest
    let new_manifest_path = output.with_extension("manifest.json");
    new_manifest.save(&new_manifest_path)?;

    info!("Incremental backup completed: {:?}", output);
    info!("New manifest saved: {:?}", new_manifest_path);

    Ok((new_manifest_path, diff))
}
