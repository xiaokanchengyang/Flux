//! Archive modification functionality for adding and removing files

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use tracing::{info, warn};

/// Options for modifying archives
#[derive(Debug, Clone)]
pub struct ModifyOptions {
    /// Whether to preserve file permissions
    pub preserve_permissions: bool,
    /// Whether to preserve timestamps
    pub preserve_timestamps: bool,
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
    /// Compression level (0-9, where 0 is no compression)
    pub compression_level: u32,
}

impl Default for ModifyOptions {
    fn default() -> Self {
        Self {
            preserve_permissions: true,
            preserve_timestamps: true,
            follow_symlinks: false,
            compression_level: 6,
        }
    }
}

/// Actions that can be performed on an archive
#[derive(Debug, Clone)]
pub enum ModifyAction {
    /// Add files to the archive
    Add(Vec<PathBuf>),
    /// Remove files matching patterns from the archive
    Remove(Vec<String>),
    /// Update existing files in the archive
    Update(Vec<PathBuf>),
}

/// Trait for archive modification
pub trait ArchiveModifier {
    /// Add files to an existing archive
    fn add_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()>;
    
    /// Remove files from an archive by pattern
    fn remove_files(&self, archive: &Path, patterns: &[String], options: &ModifyOptions) -> Result<()>;
    
    /// Update existing files in an archive
    fn update_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()>;
    
    /// Apply multiple modification actions
    fn apply_modifications(&self, archive: &Path, actions: &[ModifyAction], options: &ModifyOptions) -> Result<()> {
        for action in actions {
            match action {
                ModifyAction::Add(files) => self.add_files(archive, files, options)?,
                ModifyAction::Remove(patterns) => self.remove_files(archive, patterns, options)?,
                ModifyAction::Update(files) => self.update_files(archive, files, options)?,
            }
        }
        Ok(())
    }
}

/// Modify a tar archive
pub mod tar_modifier {
    use super::*;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use tar::{Builder, Header};
    use flate2::read::GzDecoder;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    
    pub struct TarModifier {
        compression: Option<CompressionType>,
    }
    
    #[derive(Debug, Clone, Copy)]
    enum CompressionType {
        Gzip,
        Zstd,
        Xz,
        Brotli,
    }
    
    impl TarModifier {
        pub fn new(archive_path: &Path) -> Result<Self> {
            // Detect compression type from extension
            let compression = match archive_path.extension().and_then(|e| e.to_str()) {
                Some("gz") | Some("tgz") => Some(CompressionType::Gzip),
                Some("zst") | Some("tzst") => Some(CompressionType::Zstd),
                Some("xz") | Some("txz") => Some(CompressionType::Xz),
                Some("br") => Some(CompressionType::Brotli),
                _ => None,
            };
            
            Ok(Self { compression })
        }
        
        fn open_archive_reader(&self, path: &Path) -> Result<Box<dyn Read>> {
            let file = File::open(path)?;
            
            match self.compression {
                Some(CompressionType::Gzip) => Ok(Box::new(GzDecoder::new(file))),
                Some(CompressionType::Zstd) => {
                    Ok(Box::new(zstd::stream::read::Decoder::new(file)?))
                },
                Some(CompressionType::Xz) => {
                    Ok(Box::new(xz2::read::XzDecoder::new(file)))
                },
                Some(CompressionType::Brotli) => {
                    Ok(Box::new(brotli::Decompressor::new(file, 4096)))
                },
                None => Ok(Box::new(file)),
            }
        }
        
        fn create_archive_writer(&self, file: File, level: u32) -> Result<Box<dyn Write>> {
            match self.compression {
                Some(CompressionType::Gzip) => {
                    Ok(Box::new(GzEncoder::new(file, Compression::new(level))))
                },
                Some(CompressionType::Zstd) => {
                    Ok(Box::new(zstd::stream::write::Encoder::new(file, level as i32)?))
                },
                Some(CompressionType::Xz) => {
                    Ok(Box::new(xz2::write::XzEncoder::new(file, level)))
                },
                Some(CompressionType::Brotli) => {
                    Ok(Box::new(brotli::CompressorWriter::new(file, 4096, level, 22)))
                },
                None => Ok(Box::new(file)),
            }
        }
    }
    
    impl ArchiveModifier for TarModifier {
        fn add_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()> {
            info!("Adding {} files to archive: {:?}", files.len(), archive);
            
            // Create a temporary file for the new archive
            let temp_file = tempfile::NamedTempFile::new_in(archive.parent().unwrap_or(Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            
            // Open the original archive for reading
            let reader = self.open_archive_reader(archive)?;
            let mut archive_reader = tar::Archive::new(reader);
            
            // Create new archive writer
            let writer = self.create_archive_writer(temp_file.reopen()?, options.compression_level)?;
            let mut builder = Builder::new(writer);
            
            // Copy existing entries
            let mut existing_paths = HashSet::new();
            for entry in archive_reader.entries()? {
                let mut entry = entry?;
                let path = entry.path()?.to_path_buf();
                existing_paths.insert(path.clone());
                
                let header = entry.header().clone();
                builder.append(&header, &mut entry)?;
            }
            
            // Add new files
            for file_path in files {
                if !file_path.exists() {
                    warn!("File does not exist, skipping: {:?}", file_path);
                    continue;
                }
                
                // Get relative path for the archive
                let archive_path = if file_path.is_absolute() {
                    PathBuf::from(file_path.file_name()
                        .ok_or_else(|| Error::InvalidPath("Cannot get file name".to_string()))?)
                } else {
                    file_path.clone()
                };
                
                // Skip if already exists
                if existing_paths.contains(&archive_path) {
                    warn!("File already exists in archive, skipping: {:?}", archive_path);
                    continue;
                }
                
                if file_path.is_file() {
                    let mut file = File::open(file_path)?;
                    let metadata = file.metadata()?;
                    let mut header = Header::new_gnu();
                    
                    header.set_path(&archive_path)?;
                    header.set_size(metadata.len());
                    
                    if options.preserve_permissions {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            header.set_mode(metadata.permissions().mode());
                        }
                    }
                    
                    if options.preserve_timestamps {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                                header.set_mtime(duration.as_secs());
                            }
                        }
                    }
                    
                    header.set_cksum();
                    builder.append(&header, &mut file)?;
                    info!("Added file: {:?}", archive_path);
                }
            }
            
            // Finish writing
            builder.finish()?;
            drop(builder);
            
            // Replace original with temporary
            fs::rename(temp_path, archive)?;
            
            Ok(())
        }
        
        fn remove_files(&self, archive: &Path, patterns: &[String], options: &ModifyOptions) -> Result<()> {
            info!("Removing files matching patterns from archive: {:?}", archive);
            
            // Create a temporary file for the new archive
            let temp_file = tempfile::NamedTempFile::new_in(archive.parent().unwrap_or(Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            
            // Open the original archive for reading
            let reader = self.open_archive_reader(archive)?;
            let mut archive_reader = tar::Archive::new(reader);
            
            // Create new archive writer
            let writer = self.create_archive_writer(temp_file.reopen()?, options.compression_level)?;
            let mut builder = Builder::new(writer);
            
            // Copy entries that don't match removal patterns
            let mut removed_count = 0;
            for entry in archive_reader.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;
                let path_str = path.to_string_lossy();
                
                // Check if path matches any removal pattern
                let should_remove = patterns.iter().any(|pattern| {
                    // Simple glob-like matching
                    if pattern.contains('*') {
                        let pattern_parts: Vec<&str> = pattern.split('*').collect();
                        if pattern_parts.len() == 2 {
                            path_str.starts_with(pattern_parts[0]) && path_str.ends_with(pattern_parts[1])
                        } else {
                            path_str.contains(pattern)
                        }
                    } else {
                        &path_str == pattern
                    }
                });
                
                if should_remove {
                    info!("Removing: {}", path_str);
                    removed_count += 1;
                } else {
                    let header = entry.header().clone();
                    builder.append(&header, &mut entry)?;
                }
            }
            
            // Finish writing
            builder.finish()?;
            drop(builder);
            
            if removed_count == 0 {
                warn!("No files matched the removal patterns");
                fs::remove_file(temp_path)?;
                return Ok(());
            }
            
            // Replace original with temporary
            fs::rename(temp_path, archive)?;
            info!("Removed {} files", removed_count);
            
            Ok(())
        }
        
        fn update_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()> {
            info!("Updating {} files in archive: {:?}", files.len(), archive);
            
            // Build a map of files to update
            let mut update_map = std::collections::HashMap::new();
            for file_path in files {
                if !file_path.exists() {
                    warn!("File does not exist, skipping: {:?}", file_path);
                    continue;
                }
                
                let archive_path = if file_path.is_absolute() {
                    PathBuf::from(file_path.file_name()
                        .ok_or_else(|| Error::InvalidPath("Cannot get file name".to_string()))?)
                } else {
                    file_path.clone()
                };
                
                update_map.insert(archive_path, file_path.clone());
            }
            
            // Create a temporary file for the new archive
            let temp_file = tempfile::NamedTempFile::new_in(archive.parent().unwrap_or(Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            
            // Open the original archive for reading
            let reader = self.open_archive_reader(archive)?;
            let mut archive_reader = tar::Archive::new(reader);
            
            // Create new archive writer
            let writer = self.create_archive_writer(temp_file.reopen()?, options.compression_level)?;
            let mut builder = Builder::new(writer);
            
            // Process entries
            let mut updated_count = 0;
            for entry in archive_reader.entries()? {
                let mut entry = entry?;
                let path = entry.path()?.to_path_buf();
                
                if let Some(update_path) = update_map.get(&path) {
                    // Update this entry
                    let mut file = File::open(update_path)?;
                    let metadata = file.metadata()?;
                    let mut header = Header::new_gnu();
                    
                    header.set_path(&path)?;
                    header.set_size(metadata.len());
                    
                    if options.preserve_permissions {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            header.set_mode(metadata.permissions().mode());
                        }
                    }
                    
                    if options.preserve_timestamps {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                                header.set_mtime(duration.as_secs());
                            }
                        }
                    }
                    
                    header.set_cksum();
                    builder.append(&header, &mut file)?;
                    info!("Updated: {:?}", path);
                    updated_count += 1;
                } else {
                    // Keep original entry
                    let header = entry.header().clone();
                    builder.append(&header, &mut entry)?;
                }
            }
            
            // Finish writing
            builder.finish()?;
            drop(builder);
            
            if updated_count == 0 {
                warn!("No files were updated");
                fs::remove_file(temp_path)?;
                return Ok(());
            }
            
            // Replace original with temporary
            fs::rename(temp_path, archive)?;
            info!("Updated {} files", updated_count);
            
            Ok(())
        }
    }
}

/// Modify a zip archive
pub mod zip_modifier {
    use super::*;
    use std::fs::{self, File};
    use zip::{ZipArchive, ZipWriter, write::FileOptions};
    
    pub struct ZipModifier;
    
    impl ZipModifier {
        pub fn new() -> Self {
            Self
        }
    }
    
    impl ArchiveModifier for ZipModifier {
        fn add_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()> {
            info!("Adding {} files to ZIP archive: {:?}", files.len(), archive);
            
            // Create a temporary file
            let temp_file = tempfile::NamedTempFile::new_in(archive.parent().unwrap_or(Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            
            // Open original archive
            let file = File::open(archive)?;
            let mut zip_reader = ZipArchive::new(file)?;
            
            // Create new archive
            let temp = File::create(&temp_path)?;
            let mut zip_writer = ZipWriter::new(temp);
            
            // Copy existing files
            let mut existing_names = HashSet::new();
            for i in 0..zip_reader.len() {
                let mut entry = zip_reader.by_index(i)?;
                let name = entry.name().to_string();
                existing_names.insert(name.clone());
                
                let options: FileOptions<()> = FileOptions::default()
                    .compression_method(entry.compression());
                
                zip_writer.start_file(&name, options)?;
                std::io::copy(&mut entry, &mut zip_writer)?;
            }
            
            // Add new files
            for file_path in files {
                if !file_path.exists() {
                    warn!("File does not exist, skipping: {:?}", file_path);
                    continue;
                }
                
                let archive_name = if file_path.is_absolute() {
                    file_path.file_name()
                        .ok_or_else(|| Error::InvalidPath("Cannot get file name".to_string()))?
                        .to_string_lossy()
                        .to_string()
                } else {
                    file_path.to_string_lossy().to_string()
                };
                
                if existing_names.contains(&archive_name) {
                    warn!("File already exists in archive, skipping: {}", archive_name);
                    continue;
                }
                
                if file_path.is_file() {
                    let mut file = File::open(file_path)?;
                    
                    let mut file_options: FileOptions<()> = FileOptions::default();
                    if options.compression_level > 0 {
                        file_options = file_options.compression_method(zip::CompressionMethod::Deflated);
                    }
                    
                    zip_writer.start_file(&archive_name, file_options)?;
                    std::io::copy(&mut file, &mut zip_writer)?;
                    info!("Added file: {}", archive_name);
                }
            }
            
            zip_writer.finish()?;
            
            // Replace original
            fs::rename(temp_path, archive)?;
            
            Ok(())
        }
        
        fn remove_files(&self, archive: &Path, patterns: &[String], options: &ModifyOptions) -> Result<()> {
            info!("Removing files matching patterns from ZIP archive: {:?}", archive);
            
            // Create a temporary file
            let temp_file = tempfile::NamedTempFile::new_in(archive.parent().unwrap_or(Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            
            // Open original archive
            let file = File::open(archive)?;
            let mut zip_reader = ZipArchive::new(file)?;
            
            // Create new archive
            let temp = File::create(&temp_path)?;
            let mut zip_writer = ZipWriter::new(temp);
            
            // Copy non-matching files
            let mut removed_count = 0;
            for i in 0..zip_reader.len() {
                let mut entry = zip_reader.by_index(i)?;
                let name = entry.name();
                
                // Check if name matches any removal pattern
                let should_remove = patterns.iter().any(|pattern| {
                    if pattern.contains('*') {
                        let pattern_parts: Vec<&str> = pattern.split('*').collect();
                        if pattern_parts.len() == 2 {
                            name.starts_with(pattern_parts[0]) && name.ends_with(pattern_parts[1])
                        } else {
                            name.contains(pattern)
                        }
                    } else {
                        name == pattern
                    }
                });
                
                if should_remove {
                    info!("Removing: {}", name);
                    removed_count += 1;
                } else {
                    let options: FileOptions<()> = FileOptions::default()
                        .compression_method(entry.compression());
                    
                    zip_writer.start_file(name, options)?;
                    std::io::copy(&mut entry, &mut zip_writer)?;
                }
            }
            
            zip_writer.finish()?;
            
            if removed_count == 0 {
                warn!("No files matched the removal patterns");
                fs::remove_file(temp_path)?;
                return Ok(());
            }
            
            // Replace original
            fs::rename(temp_path, archive)?;
            info!("Removed {} files", removed_count);
            
            Ok(())
        }
        
        fn update_files(&self, archive: &Path, files: &[PathBuf], options: &ModifyOptions) -> Result<()> {
            // For ZIP, updating is similar to adding with overwrite
            // First remove the files, then add them
            let patterns: Vec<String> = files.iter()
                .filter_map(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .collect();
            
            if !patterns.is_empty() {
                self.remove_files(archive, &patterns, options)?;
            }
            
            self.add_files(archive, files, options)
        }
    }
}

/// Create an archive modifier based on the archive format
pub fn create_modifier(archive_path: &Path) -> Result<Box<dyn ArchiveModifier>> {
    let ext = archive_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    match ext {
        "zip" => Ok(Box::new(zip_modifier::ZipModifier::new())),
        "tar" | "gz" | "tgz" | "zst" | "tzst" | "xz" | "txz" | "br" => {
            Ok(Box::new(tar_modifier::TarModifier::new(archive_path)?))
        },
        _ => Err(Error::UnsupportedFormat(format!("Cannot modify {} archives", ext))),
    }
}