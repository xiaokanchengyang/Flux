//! Zip extractor implementation

use super::extractor::{ArchiveEntry, ExtractEntryOptions, Extractor};
use crate::{Error, Result};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
// use tracing::{debug, info, warn};
use zip::ZipArchive;

/// Zip extractor
pub struct ZipExtractor;

impl Default for ZipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipExtractor {
    /// Create a new zip extractor
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for ZipExtractor {
    fn entries(&self, source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>> {
        let file = File::open(source)?;
        let mut archive = ZipArchive::new(file)?;

        let mut entries = Vec::new();

        for i in 0..archive.len() {
            match archive.by_index(i) {
                Ok(file) => {
                    let path = PathBuf::from(file.name());
                    let _comment = file.comment();

                    entries.push(Ok(ArchiveEntry {
                        path,
                        size: file.size(),
                        compressed_size: Some(file.compressed_size()),
                        mode: file.unix_mode(),
                        mtime: file.last_modified().map(|dt| {
                            // Convert from DOS datetime to Unix timestamp
                            let year = dt.year() as i64;
                            let month = dt.month() as i64;
                            let day = dt.day() as i64;
                            let hour = dt.hour() as i64;
                            let minute = dt.minute() as i64;
                            let second = dt.second() as i64;

                            // Simple conversion - may not be perfectly accurate
                            let days_since_epoch = (year - 1970) * 365 + (month - 1) * 30 + day;

                            days_since_epoch * 86400 + (hour * 3600 + minute * 60 + second)
                        }),
                        is_dir: file.is_dir(),
                        is_symlink: false, // ZIP doesn't directly support symlinks
                        link_target: None,
                        uid: None,
                        gid: None,
                    }));
                }
                Err(e) => entries.push(Err(Error::Zip(e.to_string()))),
            }
        }

        Ok(Box::new(entries.into_iter()))
    }

    fn extract_entry(
        &self,
        source: &Path,
        entry: &ArchiveEntry,
        destination: &Path,
        options: ExtractEntryOptions,
    ) -> Result<()> {
        let file = File::open(source)?;
        let mut archive = ZipArchive::new(file)?;

        // Find the entry by path
        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i)?;
            let zip_path = PathBuf::from(zip_file.name());

            if zip_path == entry.path {
                let full_path = destination.join(&entry.path);

                // Check if file exists and handle according to options
                if full_path.exists() && !options.overwrite {
                    return Err(Error::Io(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("File already exists: {:?}", full_path),
                    )));
                }

                // Create parent directory if needed
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Extract based on entry type
                if zip_file.is_dir() {
                    fs::create_dir_all(&full_path)?;
                } else {
                    let mut output_file = File::create(&full_path)?;
                    io::copy(&mut zip_file, &mut output_file)?;
                }

                // Set permissions if requested and available
                if options.preserve_permissions {
                    #[cfg(unix)]
                    {
                        if let Some(mode) = zip_file.unix_mode() {
                            use std::os::unix::fs::PermissionsExt;
                            let permissions = std::fs::Permissions::from_mode(mode);
                            fs::set_permissions(&full_path, permissions)?;
                        }
                    }
                }

                // Set timestamps if requested
                if options.preserve_timestamps {
                    if let Some(mtime) = entry.mtime {
                        #[cfg(unix)]
                        {
                            let mtime = filetime::FileTime::from_unix_time(mtime, 0);
                            filetime::set_file_mtime(&full_path, mtime)?;
                        }
                    }
                }

                return Ok(());
            }
        }

        Err(Error::NotFound(format!(
            "Entry not found in archive: {:?}",
            entry.path
        )))
    }

    fn format_name(&self) -> &'static str {
        "zip"
    }
}
