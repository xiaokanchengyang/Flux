//! Smart compression strategy module

use crate::{Error, Result};
use std::path::Path;
use rayon::current_num_threads;
use tracing::{debug, info};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// Store files without compression
    Store,
    /// Gzip compression
    Gzip,
    /// Zstandard compression
    Zstd,
    /// XZ compression
    Xz,
    /// Brotli compression
    Brotli,
}

impl Algorithm {
    /// Parse algorithm from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "store" | "none" => Some(Algorithm::Store),
            "gzip" | "gz" => Some(Algorithm::Gzip),
            "zstd" | "zst" => Some(Algorithm::Zstd),
            "xz" => Some(Algorithm::Xz),
            "brotli" | "br" => Some(Algorithm::Brotli),
            _ => None,
        }
    }
}

/// Compression strategy configuration
#[derive(Debug, Clone)]
pub struct CompressionStrategy {
    /// Compression algorithm to use
    pub algorithm: Algorithm,
    /// Compression level (1-9 for most algorithms)
    pub level: u32,
    /// Number of threads to use
    pub threads: usize,
    /// Force compression on already compressed files
    pub force_compress: bool,
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Zstd,
            level: 3,
            threads: current_num_threads(),
            force_compress: false,
        }
    }
}

/// Known compressed file extensions
const COMPRESSED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "avif", "heic", "heif",  // Images
    "mp4", "avi", "mkv", "mov", "webm", "flv",                     // Videos
    "mp3", "aac", "flac", "ogg", "opus", "m4a", "wma",            // Audio
    "zip", "rar", "7z", "gz", "bz2", "xz", "zst", "lz4",          // Archives
    "dmg", "iso", "img",                                            // Disk images
    "pdf", "epub", "mobi",                                          // Documents
    "apk", "ipa", "deb", "rpm", "msi", "exe",                      // Packages
];

/// Text file extensions that compress well
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "log", "json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf",
    "md", "rst", "tex", "org", "adoc",                              // Markup
    "html", "htm", "css", "js", "ts", "jsx", "tsx",                // Web
    "py", "rs", "go", "c", "cpp", "h", "hpp", "java", "kt", "swift", // Code
    "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",              // Scripts
    "sql", "csv", "tsv",                                            // Data
];

/// Large file threshold in bytes (100MB)
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Small file threshold in bytes (1KB)
const SMALL_FILE_THRESHOLD: u64 = 1024;

impl CompressionStrategy {
    /// Create a smart compression strategy based on file characteristics
    pub fn smart<P: AsRef<Path>>(path: P, level: Option<u32>, threads: Option<usize>) -> Self {
        let path = path.as_ref();
        let mut strategy = Self::default();
        
        // Override with user preferences if provided
        if let Some(level) = level {
            strategy.level = level;
        }
        if let Some(threads) = threads {
            strategy.threads = threads;
        }

        // Get file extension
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // Rule 1: Text files prefer zstd or brotli with high thread count
        if TEXT_EXTENSIONS.contains(&extension.as_str()) {
            info!("Detected text file ({}) - using zstd with high thread count", extension);
            strategy.algorithm = Algorithm::Zstd;
            strategy.threads = current_num_threads().max(4);
            if strategy.level == 3 {
                strategy.level = 6; // Higher compression for text
            }
            return strategy;
        }

        // Rule 2: Skip compression for already compressed files
        if COMPRESSED_EXTENSIONS.contains(&extension.as_str()) && !strategy.force_compress {
            info!("Detected compressed file ({}) - using store mode", extension);
            strategy.algorithm = Algorithm::Store;
            return strategy;
        }

        // Rule 3: Large file handling
        if let Ok(metadata) = path.metadata() {
            let size = metadata.len();
            
            if size > LARGE_FILE_THRESHOLD {
                info!("Detected large file ({} bytes) - using memory-efficient settings", size);
                strategy.algorithm = Algorithm::Xz;
                strategy.threads = 1; // XZ is memory intensive, use single thread
                if strategy.level == 3 {
                    strategy.level = 2; // Lower level for memory efficiency
                }
                return strategy;
            }
            
            // Rule 4: Small files should be batched
            if size < SMALL_FILE_THRESHOLD {
                debug!("Detected small file ({} bytes) - will be batched in tar", size);
                // Keep default zstd for small files, they'll be batched in tar
            }
        }

        info!("Using default strategy: {:?}", strategy);
        strategy
    }

    /// Create strategy for a directory (considering multiple files)
    pub fn smart_for_directory<P: AsRef<Path>>(
        path: P,
        level: Option<u32>,
        threads: Option<usize>,
    ) -> Result<Self> {
        let path = path.as_ref();
        let mut strategy = Self::default();
        
        // Override with user preferences if provided
        if let Some(level) = level {
            strategy.level = level;
        }
        if let Some(threads) = threads {
            strategy.threads = threads;
        }

        // Analyze directory contents
        let mut total_size = 0u64;
        let mut file_count = 0u32;
        let mut text_files = 0u32;
        let mut compressed_files = 0u32;

        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                file_count += 1;
                
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }

                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if TEXT_EXTENSIONS.contains(&ext_lower.as_str()) {
                        text_files += 1;
                    } else if COMPRESSED_EXTENSIONS.contains(&ext_lower.as_str()) {
                        compressed_files += 1;
                    }
                }
            }
        }

        if file_count == 0 {
            return Err(Error::InvalidPath("Directory is empty".to_string()));
        }

        let avg_file_size = total_size / file_count as u64;
        let text_ratio = text_files as f32 / file_count as f32;
        let compressed_ratio = compressed_files as f32 / file_count as f32;

        info!(
            "Directory analysis: {} files, {:.2}MB total, {:.2}KB avg, {:.1}% text, {:.1}% compressed",
            file_count,
            total_size as f64 / (1024.0 * 1024.0),
            avg_file_size as f64 / 1024.0,
            text_ratio * 100.0,
            compressed_ratio * 100.0
        );

        // Choose strategy based on analysis
        if compressed_ratio > 0.8 && !strategy.force_compress {
            // Mostly compressed files, use store
            info!("Directory contains mostly compressed files - using store mode");
            strategy.algorithm = Algorithm::Store;
        } else if text_ratio > 0.5 {
            // Mostly text files, use zstd with high compression
            info!("Directory contains mostly text files - using zstd with high compression");
            strategy.algorithm = Algorithm::Zstd;
            strategy.threads = current_num_threads().max(4);
            if strategy.level == 3 {
                strategy.level = 6;
            }
        } else if avg_file_size < SMALL_FILE_THRESHOLD {
            // Many small files, use fast compression
            info!("Directory contains many small files - using fast compression");
            strategy.algorithm = Algorithm::Zstd;
            strategy.threads = current_num_threads();
            if strategy.level == 3 {
                strategy.level = 1; // Fast compression for many small files
            }
        } else if total_size > LARGE_FILE_THRESHOLD * 10 {
            // Very large total size, use memory-efficient compression
            info!("Directory has very large total size - using memory-efficient compression");
            strategy.algorithm = Algorithm::Xz;
            strategy.threads = 2; // Limited threads for memory efficiency
            if strategy.level == 3 {
                strategy.level = 2;
            }
        }

        Ok(strategy)
    }

    /// Adjust strategy for parallel processing
    pub fn adjust_for_parallel(&mut self) {
        match self.algorithm {
            Algorithm::Zstd => {
                // Zstd benefits from parallelism
                self.threads = current_num_threads().max(4);
            }
            Algorithm::Xz => {
                // XZ is memory intensive, limit parallelism
                self.threads = self.threads.min(2);
            }
            Algorithm::Brotli => {
                // Brotli has moderate parallelism benefits
                self.threads = self.threads.min(current_num_threads() / 2).max(1);
            }
            Algorithm::Gzip => {
                // Gzip has limited parallelism benefits
                self.threads = self.threads.min(4);
            }
            Algorithm::Store => {
                // No compression, parallelism doesn't matter
                self.threads = 1;
            }
        }
        
        debug!("Adjusted threads for {:?}: {}", self.algorithm, self.threads);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_algorithm_from_str() {
        assert_eq!(Algorithm::from_str("zstd"), Some(Algorithm::Zstd));
        assert_eq!(Algorithm::from_str("GZIP"), Some(Algorithm::Gzip));
        assert_eq!(Algorithm::from_str("store"), Some(Algorithm::Store));
        assert_eq!(Algorithm::from_str("invalid"), None);
    }

    #[test]
    fn test_smart_strategy_for_text_file() {
        let temp_dir = TempDir::new().unwrap();
        let text_file = temp_dir.path().join("test.log");
        fs::write(&text_file, "log content").unwrap();

        let strategy = CompressionStrategy::smart(&text_file, None, None);
        assert_eq!(strategy.algorithm, Algorithm::Zstd);
        assert!(strategy.level > 3);
    }

    #[test]
    fn test_smart_strategy_for_compressed_file() {
        let temp_dir = TempDir::new().unwrap();
        let compressed_file = temp_dir.path().join("image.jpg");
        fs::write(&compressed_file, "fake jpeg").unwrap();

        let strategy = CompressionStrategy::smart(&compressed_file, None, None);
        assert_eq!(strategy.algorithm, Algorithm::Store);
    }

    #[test]
    fn test_smart_strategy_for_directory() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create mixed content
        fs::write(temp_dir.path().join("file1.txt"), "text content").unwrap();
        fs::write(temp_dir.path().join("file2.log"), "log content").unwrap();
        fs::write(temp_dir.path().join("image.jpg"), "fake jpeg").unwrap();
        
        let strategy = CompressionStrategy::smart_for_directory(temp_dir.path(), None, None).unwrap();
        // Should recognize mixed content and use appropriate strategy
        assert!(matches!(strategy.algorithm, Algorithm::Zstd | Algorithm::Store));
    }
}