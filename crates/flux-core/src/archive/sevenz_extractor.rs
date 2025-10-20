//! 7z extractor implementation

use super::extractor::{ArchiveEntry, ExtractEntryOptions, Extractor};
use crate::{Error, Result};
use sevenz_rust::{Password, SevenZReader};
use std::fs::{self, File};
use std::io;
use std::path::Path;
// use tracing::{debug, info, warn};

/// 7z extractor
pub struct SevenZExtractor;

impl Default for SevenZExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SevenZExtractor {
    /// Create a new 7z extractor
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for SevenZExtractor {
    fn entries(&self, _source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>> {
        // Note: 7z doesn't provide a way to list entries without extracting
        // For now, we'll return an error indicating this operation is not supported
        Err(Error::UnsupportedOperation(
            "Listing entries for 7z archives is not currently supported".to_string(),
        ))
    }

    fn extract_entry(
        &self,
        _source: &Path,
        _entry: &ArchiveEntry,
        _destination: &Path,
        _options: ExtractEntryOptions,
    ) -> Result<()> {
        // Note: 7z doesn't support extracting individual entries
        Err(Error::UnsupportedOperation(
            "Extracting individual entries from 7z archives is not currently supported".to_string(),
        ))
    }

    fn format_name(&self) -> &'static str {
        "7z"
    }
}

/// Extract entire 7z archive (fallback for non-interactive mode)
pub fn extract_7z_fallback<P: AsRef<Path>, Q: AsRef<Path>>(
    archive: P,
    output_dir: Q,
) -> Result<()> {
    let archive = archive.as_ref();
    let output_dir = output_dir.as_ref();

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let mut reader = SevenZReader::open(archive, Password::empty())
        .map_err(|e| Error::Archive(format!("Failed to open 7z archive: {}", e)))?;

    // Extract all files
    reader
        .for_each_entries(|entry, reader| {
            let path = output_dir.join(&entry.name);

            if entry.is_directory {
                fs::create_dir_all(&path)?;
            } else {
                // Create parent directory if needed
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut output_file = File::create(&path)?;
                io::copy(reader, &mut output_file)?;
            }

            Ok(true) // Continue extraction
        })
        .map_err(|e| Error::Archive(format!("Failed to extract 7z archive: {}", e)))?;

    Ok(())
}
