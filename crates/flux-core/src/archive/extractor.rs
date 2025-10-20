//! Extractor trait for archive operations

use crate::Result;
use std::path::{Path, PathBuf};

/// Entry in an archive
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Path within the archive (relative to archive root)
    pub path: PathBuf,
    /// Original size in bytes
    pub size: u64,
    /// Compressed size in bytes (if available)
    pub compressed_size: Option<u64>,
    /// Unix permissions (if available)
    pub mode: Option<u32>,
    /// Modification time (Unix timestamp)
    pub mtime: Option<i64>,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
    /// Link target (for symlinks)
    pub link_target: Option<PathBuf>,
    /// User ID (if available)
    pub uid: Option<u32>,
    /// Group ID (if available)
    pub gid: Option<u32>,
}

/// Options for extracting entries
#[derive(Debug, Clone, Default)]
pub struct ExtractEntryOptions {
    /// Whether to overwrite existing files
    pub overwrite: bool,
    /// Whether to preserve permissions
    pub preserve_permissions: bool,
    /// Whether to preserve timestamps
    pub preserve_timestamps: bool,
    /// Whether to follow symlinks when extracting
    pub follow_symlinks: bool,
}

/// Trait for archive extractors
pub trait Extractor: Send + Sync {
    /// Get an iterator over all entries in the archive without extracting
    fn entries(&self, source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>>;

    /// Extract a single entry to the specified destination
    fn extract_entry(
        &self,
        source: &Path,
        entry: &ArchiveEntry,
        destination: &Path,
        options: ExtractEntryOptions,
    ) -> Result<()>;

    /// Get the format name for this extractor
    fn format_name(&self) -> &'static str;
}

/// Conflict resolution action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    /// Overwrite the existing file
    Overwrite,
    /// Skip this file
    Skip,
    /// Overwrite all future conflicts
    OverwriteAll,
    /// Skip all future conflicts
    SkipAll,
    /// Abort the entire extraction
    Abort,
    /// Rename the file (append number)
    Rename,
}

/// Trait for handling extraction conflicts
pub trait ConflictHandler {
    /// Called when a file already exists at the destination
    fn handle_conflict(&mut self, entry: &ArchiveEntry, existing_path: &Path) -> ConflictAction;
}

/// Default conflict handler that always skips
pub struct SkipConflictHandler;

impl ConflictHandler for SkipConflictHandler {
    fn handle_conflict(&mut self, _entry: &ArchiveEntry, _existing_path: &Path) -> ConflictAction {
        ConflictAction::Skip
    }
}

/// Conflict handler that always overwrites
pub struct OverwriteConflictHandler;

impl ConflictHandler for OverwriteConflictHandler {
    fn handle_conflict(&mut self, _entry: &ArchiveEntry, _existing_path: &Path) -> ConflictAction {
        ConflictAction::Overwrite
    }
}
