//! Secure wrapper for archive extractors with security checks

use super::extractor::{Extractor, ArchiveEntry, ExtractEntryOptions};
use crate::security::{SecurityOptions, sanitize_path, validate_symlink, check_compression_ratio, check_extraction_size, check_disk_space};
use crate::{Result, Error};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Secure wrapper for any extractor that adds security checks
pub struct SecureExtractor {
    inner: Box<dyn Extractor>,
    security_options: SecurityOptions,
}

impl SecureExtractor {
    /// Create a new secure extractor with default security options
    pub fn new(inner: Box<dyn Extractor>) -> Self {
        Self {
            inner,
            security_options: SecurityOptions::default(),
        }
    }
    
    /// Create a new secure extractor with custom security options
    pub fn with_options(inner: Box<dyn Extractor>, security_options: SecurityOptions) -> Self {
        Self {
            inner,
            security_options,
        }
    }
}

impl Extractor for SecureExtractor {
    fn entries(&self, source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>> {
        // Get entries from inner extractor
        let entries = self.inner.entries(source)?;
        let security_options = self.security_options.clone();
        
        // Wrap the iterator to add security checks
        let secure_entries = entries.map(move |entry_result| {
            match entry_result {
                Ok(entry) => {
                    // Validate the entry path
                    if let Err(e) = validate_entry_path(&entry.path) {
                        warn!(path = ?entry.path, error = %e, "Invalid entry path");
                        return Err(e);
                    }
                    
                    // Check compression ratio if both sizes are available
                    if let Some(compressed_size) = entry.compressed_size {
                        if compressed_size > 0 {
                            if let Err(e) = check_compression_ratio(
                                compressed_size,
                                entry.size,
                                security_options.max_compression_ratio,
                            ) {
                                warn!(
                                    path = ?entry.path,
                                    compressed_size,
                                    uncompressed_size = entry.size,
                                    "Suspicious compression ratio"
                                );
                                return Err(e);
                            }
                        }
                    }
                    
                    Ok(entry)
                }
                Err(e) => Err(e),
            }
        });
        
        Ok(Box::new(secure_entries))
    }
    
    fn extract_entry(
        &self,
        source: &Path,
        entry: &ArchiveEntry,
        destination: &Path,
        options: ExtractEntryOptions,
    ) -> Result<()> {
        // Sanitize the destination path
        let safe_path = sanitize_path(destination, &entry.path)?;
        
        debug!(
            entry_path = ?entry.path,
            safe_path = ?safe_path,
            "Extracting entry with sanitized path"
        );
        
        // If it's a symlink, validate the target
        if entry.is_symlink {
            if let Some(ref target) = entry.link_target {
                validate_symlink(
                    destination,
                    &safe_path,
                    target,
                    self.security_options.allow_external_symlinks,
                )?;
            }
        }
        
        // Create a modified entry with the sanitized path
        let safe_entry = ArchiveEntry {
            path: safe_path.strip_prefix(destination)
                .unwrap_or(&safe_path)
                .to_path_buf(),
            ..entry.clone()
        };
        
        // Extract using the inner extractor
        self.inner.extract_entry(source, &safe_entry, destination, options)
    }
    
    fn format_name(&self) -> &'static str {
        self.inner.format_name()
    }
}

/// Validate an entry path to ensure it doesn't contain dangerous components
fn validate_entry_path(path: &Path) -> Result<()> {
    use std::path::Component;
    
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {
                // These are safe
            }
            Component::ParentDir => {
                return Err(Error::SecurityError(format!(
                    "Entry path contains parent directory component: {:?}",
                    path
                )));
            }
            Component::RootDir => {
                return Err(Error::SecurityError(format!(
                    "Entry path is absolute: {:?}",
                    path
                )));
            }
            Component::Prefix(_) => {
                return Err(Error::SecurityError(format!(
                    "Entry path contains Windows prefix: {:?}",
                    path
                )));
            }
        }
    }
    
    Ok(())
}

/// Extract an archive with security checks and size limits
pub fn extract_archive_secure(
    source: &Path,
    destination: &Path,
    extractor: Box<dyn Extractor>,
    security_options: SecurityOptions,
) -> Result<()> {
    info!(
        source = ?source,
        destination = ?destination,
        max_size = security_options.max_extraction_size,
        "Starting secure extraction"
    );
    
    // Check disk space if enabled
    if security_options.check_disk_space {
        // First pass: calculate total size
        let mut total_size = 0u64;
        for entry in extractor.entries(source)? {
            let entry = entry?;
            total_size = total_size.saturating_add(entry.size);
        }
        
        debug!(total_size, "Calculated total extraction size");
        
        // Check if we have enough disk space
        check_disk_space(destination, total_size)?;
    }
    
    // Create secure extractor
    let secure_extractor = SecureExtractor::with_options(
        extractor,
        security_options.clone(),
    );
    
    // Track total extracted size
    let total_extracted = Arc::new(AtomicU64::new(0));
    
    // Extract entries
    let mut extracted_count = 0;
    let mut error_count = 0;
    
    for entry in secure_extractor.entries(source)? {
        match entry {
            Ok(entry) => {
                // Check if extraction would exceed size limit
                let current_total = total_extracted.load(Ordering::Relaxed);
                if let Err(e) = check_extraction_size(
                    current_total,
                    entry.size,
                    security_options.max_extraction_size,
                ) {
                    return Err(e);
                }
                
                // Extract the entry
                let options = ExtractEntryOptions {
                    overwrite: true,
                    preserve_permissions: true,
                    preserve_timestamps: true,
                    follow_symlinks: false,
                };
                
                match secure_extractor.extract_entry(source, &entry, destination, options) {
                    Ok(()) => {
                        extracted_count += 1;
                        total_extracted.fetch_add(entry.size, Ordering::Relaxed);
                        
                        if extracted_count % 100 == 0 {
                            debug!(extracted_count, "Extraction progress");
                        }
                    }
                    Err(e) => {
                        warn!(path = ?entry.path, error = %e, "Failed to extract entry");
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to read entry");
                error_count += 1;
            }
        }
    }
    
    info!(
        extracted_count,
        error_count,
        total_bytes = total_extracted.load(Ordering::Relaxed),
        "Extraction completed"
    );
    
    if error_count > 0 {
        Err(Error::PartialFailure { count: error_count })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::extractor::ArchiveEntry;
    use std::path::PathBuf;
    
    // Mock extractor for testing
    struct MockExtractor {
        entries: Vec<ArchiveEntry>,
    }
    
    impl Extractor for MockExtractor {
        fn entries(&self, _source: &Path) -> Result<Box<dyn Iterator<Item = Result<ArchiveEntry>>>> {
            let entries = self.entries.clone();
            Ok(Box::new(entries.into_iter().map(Ok)))
        }
        
        fn extract_entry(
            &self,
            _source: &Path,
            _entry: &ArchiveEntry,
            _destination: &Path,
            _options: ExtractEntryOptions,
        ) -> Result<()> {
            Ok(())
        }
        
        fn format_name(&self) -> &'static str {
            "mock"
        }
    }
    
    #[test]
    fn test_secure_extractor_blocks_path_traversal() {
        let mock = MockExtractor {
            entries: vec![
                ArchiveEntry {
                    path: PathBuf::from("../etc/passwd"),
                    size: 1000,
                    compressed_size: None,
                    mode: None,
                    mtime: None,
                    is_dir: false,
                    is_symlink: false,
                    link_target: None,
                    uid: None,
                    gid: None,
                },
            ],
        };
        
        let secure = SecureExtractor::new(Box::new(mock));
        let entries: Vec<_> = secure.entries(Path::new("test.zip"))
            .unwrap()
            .collect();
        
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_err());
    }
    
    #[test]
    fn test_secure_extractor_allows_normal_paths() {
        let mock = MockExtractor {
            entries: vec![
                ArchiveEntry {
                    path: PathBuf::from("normal/file.txt"),
                    size: 1000,
                    compressed_size: Some(500),
                    mode: None,
                    mtime: None,
                    is_dir: false,
                    is_symlink: false,
                    link_target: None,
                    uid: None,
                    gid: None,
                },
            ],
        };
        
        let secure = SecureExtractor::new(Box::new(mock));
        let entries: Vec<_> = secure.entries(Path::new("test.zip"))
            .unwrap()
            .collect();
        
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_ok());
    }
    
    #[test]
    fn test_compression_ratio_check() {
        let mock = MockExtractor {
            entries: vec![
                ArchiveEntry {
                    path: PathBuf::from("bomb.txt"),
                    size: 1_000_000_000, // 1 GB uncompressed
                    compressed_size: Some(1000), // 1 KB compressed = 1000:1 ratio
                    mode: None,
                    mtime: None,
                    is_dir: false,
                    is_symlink: false,
                    link_target: None,
                    uid: None,
                    gid: None,
                },
            ],
        };
        
        let secure = SecureExtractor::new(Box::new(mock));
        let entries: Vec<_> = secure.entries(Path::new("test.zip"))
            .unwrap()
            .collect();
        
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_err()); // Should fail due to high compression ratio
    }
}