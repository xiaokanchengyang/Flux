//! Tar extractor implementation

use super::extractor::{ArchiveEntry, ExtractEntryOptions, Extractor};
use crate::strategy::Algorithm;
use crate::{Error, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use tar::Archive;
use tracing::warn;
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

/// Tar extractor
pub struct TarExtractor {
    compression: Option<Algorithm>,
}

impl Default for TarExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TarExtractor {
    /// Create a new tar extractor
    pub fn new() -> Self {
        Self { compression: None }
    }

    /// Create a tar extractor with compression
    pub fn with_compression(compression: Algorithm) -> Self {
        Self {
            compression: Some(compression),
        }
    }

    /// Create appropriate reader based on compression
    fn create_reader<'a>(&self, file: File) -> Result<Box<dyn Read + 'a>> {
        match self.compression {
            None => Ok(Box::new(file)),
            Some(Algorithm::Gzip) => Ok(Box::new(GzDecoder::new(file))),
            Some(Algorithm::Zstd) => Ok(Box::new(ZstdDecoder::new(file)?)),
            Some(Algorithm::Xz) => Ok(Box::new(XzDecoder::new(file))),
            Some(Algorithm::Brotli) => Ok(Box::new(brotli::Decompressor::new(file, 4096))),
            Some(Algorithm::Store) => Ok(Box::new(file)),
        }
    }
}

impl Extractor for TarExtractor {
    fn entries(&self, source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>> {
        let file = File::open(source)?;
        let reader = self.create_reader(file)?;
        let mut archive = Archive::new(reader);

        // Collect all entries into a vector since we can't return the archive itself
        let mut entries = Vec::new();

        for entry in archive.entries()? {
            match entry {
                Ok(entry) => {
                    let path = entry.path()?.to_path_buf();
                    let header = entry.header();

                    entries.push(Ok(ArchiveEntry {
                        path,
                        size: header.size()?,
                        compressed_size: None, // Tar doesn't store compressed size per entry
                        mode: Some(header.mode()?),
                        mtime: Some(header.mtime()? as i64),
                        is_dir: header.entry_type().is_dir(),
                        is_symlink: header.entry_type().is_symlink(),
                        link_target: header.link_name()?.map(|p| p.to_path_buf()),
                        uid: header.uid().ok().map(|u| u as u32),
                        gid: header.gid().ok().map(|g| g as u32),
                    }));
                }
                Err(e) => entries.push(Err(Error::Io(e))),
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
        let reader = self.create_reader(file)?;
        let mut archive = Archive::new(reader);

        // Find and extract the specific entry
        for archive_entry in archive.entries()? {
            let mut archive_entry = archive_entry?;
            let entry_path = archive_entry.path()?.to_path_buf();

            if entry_path == entry.path {
                let full_path = destination.join(&entry_path);

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
                let header = archive_entry.header();
                match header.entry_type() {
                    tar::EntryType::Directory => {
                        fs::create_dir_all(&full_path)?;
                    }
                    tar::EntryType::Regular | tar::EntryType::Continuous => {
                        archive_entry.unpack(&full_path)?;
                    }
                    tar::EntryType::Symlink => {
                        if let Some(link_target) = &entry.link_target {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs;
                                if full_path.exists() {
                                    std::fs::remove_file(&full_path)?;
                                }
                                fs::symlink(link_target, &full_path)?;
                            }
                            #[cfg(not(unix))]
                            {
                                warn!("Symlink extraction not supported on this platform");
                            }
                        }
                    }
                    _ => {
                        warn!("Unsupported entry type: {:?}", header.entry_type());
                    }
                }

                // Set permissions if requested
                if options.preserve_permissions {
                    if let Some(mode) = entry.mode {
                        #[cfg(unix)]
                        {
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
                            //use std::os::unix::fs::MetadataExt;
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
        match self.compression {
            None => "tar",
            Some(Algorithm::Gzip) => "tar.gz",
            Some(Algorithm::Zstd) => "tar.zst",
            Some(Algorithm::Xz) => "tar.xz",
            Some(Algorithm::Brotli) => "tar.br",
            Some(Algorithm::Store) => "tar",
        }
    }
}

/// Create an extractor for the given tar file based on its extension
pub fn create_tar_extractor(path: &Path) -> Result<Box<dyn Extractor>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let compression = if stem.ends_with(".tar") {
        match ext {
            "gz" => Some(Algorithm::Gzip),
            "zst" => Some(Algorithm::Zstd),
            "xz" => Some(Algorithm::Xz),
            "br" => Some(Algorithm::Brotli),
            _ => None,
        }
    } else {
        None
    };

    Ok(Box::new(TarExtractor { compression }))
}
