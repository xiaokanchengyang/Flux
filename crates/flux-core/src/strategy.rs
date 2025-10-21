//! Smart compression strategy module
//!
//! This module provides intelligent compression strategies based on file characteristics.
//!
//! # Small File Batching
//!
//! One of the key optimizations is the automatic batching of small files. When processing
//! directories with many small files (< 1KB), the strategy system recognizes that these
//! files benefit from being archived together before compression. This allows the compression
//! algorithm to:
//!
//! - Share dictionary data across files
//! - Find common patterns between files
//! - Reduce per-file metadata overhead
//! - Achieve significantly better compression ratios
//!
//! The batching happens automatically when using smart strategies - no manual configuration
//! is required. Simply enable smart mode and the system will detect and optimize for small files.

use crate::config::Config;
use crate::{Error, Result};
use glob::Pattern;
use rayon::current_num_threads;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
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

impl std::str::FromStr for Algorithm {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "store" | "none" => Ok(Algorithm::Store),
            "gzip" | "gz" => Ok(Algorithm::Gzip),
            "zstd" | "zst" => Ok(Algorithm::Zstd),
            "xz" => Ok(Algorithm::Xz),
            "brotli" | "br" => Ok(Algorithm::Brotli),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algorithm::Store => write!(f, "store"),
            Algorithm::Gzip => write!(f, "gzip"),
            Algorithm::Zstd => write!(f, "zstd"),
            Algorithm::Xz => write!(f, "xz"),
            Algorithm::Brotli => write!(f, "brotli"),
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
    /// Enable long mode for zstd (for very large files)
    pub long_mode: bool,
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Zstd,
            level: 3,
            threads: current_num_threads(),
            force_compress: false,
            long_mode: false,
        }
    }
}

/// Known compressed file extensions
const COMPRESSED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "avif", "heic", "heif", // Images
    "mp4", "avi", "mkv", "mov", "webm", "flv", // Videos
    "mp3", "aac", "flac", "ogg", "opus", "m4a", "wma", // Audio
    "zip", "rar", "7z", "gz", "bz2", "xz", "zst", "lz4", // Archives
    "dmg", "iso", "img", // Disk images
    "pdf", "epub", "mobi", // Documents
    "apk", "ipa", "deb", "rpm", "msi", "exe", // Packages
];

/// Text file extensions that compress well
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "log", "json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf", "md", "rst", "tex",
    "org", "adoc", // Markup
    "html", "htm", "css", "js", "ts", "jsx", "tsx", // Web
    "py", "rs", "go", "c", "cpp", "h", "hpp", "java", "kt", "swift", // Code
    "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", // Scripts
    "sql", "csv", "tsv", // Data
];

/// Large file threshold in bytes (100MB)
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Small file threshold in bytes (1KB)
const SMALL_FILE_THRESHOLD: u64 = 1024;

/// High entropy threshold (above this, file is likely compressed)
const HIGH_ENTROPY_THRESHOLD: f64 = 7.5;

/// Sample size for entropy calculation (16KB)
const ENTROPY_SAMPLE_SIZE: usize = 16 * 1024;

/// Calculate Shannon entropy for a byte sample
fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    // Count byte frequencies using a single pass
    let mut freq = [0u64; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    // Calculate entropy with pre-computed length
    let len = data.len() as f64;
    let inv_len = 1.0 / len;
    let mut entropy = 0.0;

    // Use iterator for better performance
    freq.iter()
        .filter(|&&count| count > 0)
        .for_each(|&count| {
            let p = count as f64 * inv_len;
            entropy -= p * p.log2();
        });

    entropy
}

/// Check if a file has high entropy (likely compressed)
fn is_high_entropy_file(path: &Path) -> Result<bool> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Read sample from the beginning of the file
    let mut buffer = vec![0u8; ENTROPY_SAMPLE_SIZE];
    let bytes_read = reader.read(&mut buffer)?;

    if bytes_read == 0 {
        return Ok(false);
    }

    buffer.truncate(bytes_read);
    let entropy = calculate_entropy(&buffer);

    debug!(
        "File {:?} entropy: {:.2} (threshold: {:.2})",
        path.file_name().unwrap_or_default(),
        entropy,
        HIGH_ENTROPY_THRESHOLD
    );

    Ok(entropy > HIGH_ENTROPY_THRESHOLD)
}

/// Apply custom rules from configuration to determine compression strategy
fn apply_custom_rules(path: &Path, config: &Config) -> Option<CompressionStrategy> {
    let file_name = path.file_name()?.to_str()?;
    let file_size = path.metadata().ok()?.len();

    // Sort rules by priority (descending)
    let mut rules = config.rules.clone();
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    for rule in rules {
        // Check if any pattern matches
        let pattern_matches = rule.patterns.iter().any(|pattern| {
            Pattern::new(pattern)
                .ok()
                .map(|p| p.matches(file_name))
                .unwrap_or(false)
        });

        if !pattern_matches {
            continue;
        }

        // Check size constraints
        if let Some(min_size) = rule.min_size {
            if file_size < min_size {
                continue;
            }
        }

        if let Some(max_size) = rule.max_size {
            if file_size > max_size {
                continue;
            }
        }

        // Rule matches, create strategy
        info!("Applying custom rule '{}' to file {:?}", rule.name, path);

        let algorithm = rule.algorithm.parse().ok()?;
        return Some(CompressionStrategy {
            algorithm,
            level: rule.level.unwrap_or(3),
            threads: rule.threads.unwrap_or_else(current_num_threads),
            force_compress: false,
            long_mode: false,
        });
    }

    None
}

impl CompressionStrategy {
    /// Create a smart compression strategy based on file characteristics
    pub fn smart<P: AsRef<Path>>(path: P, level: Option<u32>, threads: Option<usize>) -> Self {
        let path = path.as_ref();
        let mut strategy = Self::default();

        // Store user preferences for later application
        let user_level = level;
        let user_threads = threads;

        // Get file size early for size-based rules
        let file_size = path.metadata().map(|m| m.len()).unwrap_or(0);

        // Rule 0: Check custom rules from configuration (skip in tests)
        if std::env::var("FLUX_NO_CONFIG").is_err() {
            if let Ok(config) = Config::load() {
                if let Some(custom_strategy) = apply_custom_rules(path, &config) {
                    info!("Using custom rule-based strategy: {:?}", custom_strategy);
                    // Apply user overrides to custom strategy
                    let mut custom = custom_strategy;
                    if let Some(level) = user_level {
                        custom.level = level;
                    }
                    if let Some(threads) = user_threads {
                        custom.threads = threads;
                    }
                    return custom;
                }

                // Check size-based rules
                for size_rule in &config.strategy.size_rules {
                    if file_size >= size_rule.threshold {
                        info!(
                            "File size {} bytes exceeds threshold {} bytes, using {} algorithm with level {}",
                            file_size, size_rule.threshold, size_rule.algorithm, size_rule.level
                        );

                        if let Ok(algorithm) = size_rule.algorithm.parse::<Algorithm>() {
                            strategy.algorithm = algorithm;
                            strategy.level = user_level.unwrap_or(size_rule.level);

                            // Apply automatic thread adjustment for the selected algorithm
                            strategy.threads = user_threads.unwrap_or_else(|| {
                                match algorithm {
                                    Algorithm::Xz => 1, // XZ should always use single thread
                                    Algorithm::Zstd => (current_num_threads() / 2).max(2),
                                    Algorithm::Brotli => (current_num_threads() / 3).max(1),
                                    _ => current_num_threads(),
                                }
                            });

                            return strategy;
                        }
                    }
                }
            }
        }

        // Get file extension
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // Rule 1: Text files prefer zstd or brotli with high thread count
        if TEXT_EXTENSIONS.contains(&extension.as_str()) {
            info!(
                "Detected text file ({}) - using zstd with high thread count",
                extension
            );
            strategy.algorithm = Algorithm::Zstd;
            strategy.threads = user_threads.unwrap_or_else(|| current_num_threads().max(4));
            strategy.level = user_level.unwrap_or(6); // Higher compression for text
            return strategy;
        }

        // Rule 2: Skip compression for already compressed files
        if COMPRESSED_EXTENSIONS.contains(&extension.as_str()) && !strategy.force_compress {
            info!(
                "Detected compressed file ({}) - using store mode",
                extension
            );
            strategy.algorithm = Algorithm::Store;
            return strategy;
        }

        // Rule 2b: Check entropy for files without known extensions
        // This catches compressed files that might not have standard extensions
        // Skip entropy check for files with known extensions (text or compressed)
        if !TEXT_EXTENSIONS.contains(&extension.as_str())
            && !COMPRESSED_EXTENSIONS.contains(&extension.as_str())
        {
            if let Ok(metadata) = path.metadata() {
                // Check entropy for files 1KB or larger (to avoid false positives on tiny files)
                if metadata.len() >= 1024 {
                    if let Ok(is_compressed) = is_high_entropy_file(path) {
                        if is_compressed && !strategy.force_compress {
                            info!(
                                "Detected high-entropy file (likely compressed) - using store mode"
                            );
                            strategy.algorithm = Algorithm::Store;
                            strategy.level = user_level.unwrap_or(strategy.level);
                            strategy.threads = user_threads.unwrap_or(strategy.threads);
                            return strategy;
                        }
                    }
                }
            }
        }

        // Rule 3: Large file handling
        if let Ok(metadata) = path.metadata() {
            let size = metadata.len();

            // Get threshold from config or use default
            let large_file_threshold = if std::env::var("FLUX_NO_CONFIG").is_err() {
                Config::load_or_default()
                    .strategy
                    .large_file_threshold
                    .unwrap_or(LARGE_FILE_THRESHOLD)
            } else {
                LARGE_FILE_THRESHOLD
            };

            if size > large_file_threshold {
                info!(
                    "Detected large file ({} bytes) - using memory-efficient settings",
                    size
                );

                // Check if we should use zstd long mode for very large files
                let config = if std::env::var("FLUX_NO_CONFIG").is_err() {
                    Config::load_or_default()
                } else {
                    Config::default()
                };

                if size > large_file_threshold * 10 && config.strategy.enable_long_mode {
                    // For files > 1GB, use zstd with long mode
                    info!("Using zstd with long mode for very large file");
                    strategy.algorithm = Algorithm::Zstd;
                    strategy.long_mode = true;
                    strategy.threads = user_threads.unwrap_or(2);
                    strategy.level = user_level.unwrap_or(3);
                } else {
                    // For files 100MB-1GB, use XZ for better compression
                    strategy.algorithm = Algorithm::Xz;
                    strategy.threads = user_threads.unwrap_or(1); // XZ is memory intensive
                    strategy.level = user_level.unwrap_or(2); // Lower level for memory efficiency
                }
                return strategy;
            }

            // Rule 4: Medium-sized files (1MB - 100MB) - balanced approach
            if size > 1024 * 1024 && size < LARGE_FILE_THRESHOLD {
                info!(
                    "Detected medium file ({:.2} MB) - using balanced settings",
                    size as f64 / (1024.0 * 1024.0)
                );
                strategy.algorithm = Algorithm::Zstd;
                // Use moderate thread count for medium files
                strategy.threads =
                    user_threads.unwrap_or_else(|| (current_num_threads() / 2).max(2));
                strategy.level = user_level.unwrap_or(strategy.level);
            }

            // Rule 5: Small files batching strategy
            // Small files (< 1KB) benefit from being batched together in a tar archive
            // before compression. This allows the compression algorithm to find patterns
            // across multiple files, resulting in better compression ratios.
            if size < SMALL_FILE_THRESHOLD {
                debug!(
                    "Detected small file ({} bytes) - will be batched in tar for optimal compression",
                    size
                );
                // Keep default zstd for small files
                // When multiple small files are packed together, they are automatically
                // batched into a tar archive first, then the entire archive is compressed.
                // This approach typically achieves 20-50% better compression ratios compared
                // to compressing each file individually.
            }
        }

        // Apply any remaining user preferences
        if let Some(level) = user_level {
            strategy.level = level;
        }
        if let Some(threads) = user_threads {
            strategy.threads = threads;
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
        if compressed_ratio > 0.7 && !strategy.force_compress {
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

    /// Adjust strategy for parallel processing with file size consideration
    pub fn adjust_for_parallel(&mut self, file_size: u64) {
        match self.algorithm {
            Algorithm::Zstd => {
                // Zstd benefits from parallelism, scale with file size
                if file_size < 10 * 1024 * 1024 {
                    // < 10MB
                    self.threads = 1;
                } else if file_size < 100 * 1024 * 1024 {
                    // < 100MB
                    self.threads = (current_num_threads() / 2).max(2);
                } else {
                    self.threads = current_num_threads().max(4);
                }

                // Apply long mode thread adjustment
                if self.long_mode {
                    self.threads = self.threads.min(4); // Long mode uses more memory
                }
            }
            Algorithm::Xz => {
                // XZ is memory intensive, always limit parallelism
                self.threads = 1; // Force single thread for stability
                info!("XZ compression forced to single thread for stability");
            }
            Algorithm::Brotli => {
                // Brotli has moderate parallelism benefits
                if file_size < 50 * 1024 * 1024 {
                    // < 50MB
                    self.threads = 1;
                } else {
                    self.threads = (current_num_threads() / 2).clamp(1, 4);
                }
            }
            Algorithm::Gzip => {
                // Gzip has limited parallelism benefits
                self.threads = self.threads.min(2);
            }
            Algorithm::Store => {
                // No compression, parallelism doesn't matter
                self.threads = 1;
            }
        }

        debug!(
            "Adjusted threads for {:?} ({}MB file): {}",
            self.algorithm,
            file_size / (1024 * 1024),
            self.threads
        );
    }

    /// Legacy method without file size (for backwards compatibility)
    pub fn adjust_for_parallel_legacy(&mut self) {
        self.adjust_for_parallel(100 * 1024 * 1024); // Assume 100MB file
    }
}

/// Determine compression strategy for a specific file entry
pub fn determine_compression_for_entry<P: AsRef<Path>>(
    path: P,
    size: u64,
    config: &Config,
) -> CompressionStrategy {
    let path = path.as_ref();

    // Check size-based rules first (highest priority)
    for size_rule in &config.strategy.size_rules {
        if size >= size_rule.threshold {
            info!(
                "File {:?} size {} bytes exceeds threshold {} bytes, using {} algorithm with level {}",
                path.file_name().unwrap_or_default(),
                size,
                size_rule.threshold,
                size_rule.algorithm,
                size_rule.level
            );

            if let Ok(algorithm) = size_rule.algorithm.parse::<Algorithm>() {
                let mut strategy = CompressionStrategy {
                    algorithm,
                    level: size_rule.level,
                    threads: 1, // Will be adjusted later based on algorithm
                    force_compress: false,
                    long_mode: false,
                };

                // Adjust threads based on algorithm and size
                match algorithm {
                    Algorithm::Xz => strategy.threads = 1, // XZ should always use single thread
                    Algorithm::Zstd => {
                        if size < 10 * 1024 * 1024 {
                            strategy.threads = 1;
                        } else if size < 100 * 1024 * 1024 {
                            strategy.threads = 2;
                        } else {
                            strategy.threads = (rayon::current_num_threads() / 2).max(2);
                        }
                    }
                    Algorithm::Brotli => {
                        if size < 50 * 1024 * 1024 {
                            strategy.threads = 1;
                        } else {
                            strategy.threads = (rayon::current_num_threads() / 3).max(1);
                        }
                    }
                    _ => strategy.threads = rayon::current_num_threads(),
                }

                return strategy;
            }
        }
    }

    // If no size rule matches, use smart strategy
    CompressionStrategy::smart(path, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, SizeRule};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_algorithm_from_str() {
        assert_eq!("zstd".parse::<Algorithm>(), Ok(Algorithm::Zstd));
        assert_eq!("GZIP".parse::<Algorithm>(), Ok(Algorithm::Gzip));
        assert_eq!("store".parse::<Algorithm>(), Ok(Algorithm::Store));
        assert!("invalid".parse::<Algorithm>().is_err());
    }

    #[test]
    fn test_smart_strategy_for_text_file() {
        let temp_dir = TempDir::new().unwrap();
        let text_file = temp_dir.path().join("test.log");
        // Create a larger text file to avoid entropy false positives
        let content =
            "This is a log file with enough content to be properly analyzed.\n".repeat(20);
        fs::write(&text_file, content).unwrap();

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

        let strategy =
            CompressionStrategy::smart_for_directory(temp_dir.path(), None, None).unwrap();
        // Should recognize mixed content and use appropriate strategy
        assert!(matches!(
            strategy.algorithm,
            Algorithm::Zstd | Algorithm::Store
        ));
    }

    #[test]
    fn test_determine_compression_for_entry_with_size_rules() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("large_file.bin");
        fs::write(&test_file, "test content").unwrap();

        // Create config with size rules
        let mut config = Config::default();
        config.strategy.size_rules = vec![SizeRule {
            threshold: 100 * 1024 * 1024, // 100 MiB
            algorithm: "xz".to_string(),
            level: 7,
        }];

        // Test file below threshold
        let strategy = determine_compression_for_entry(&test_file, 50 * 1024 * 1024, &config);
        // Should use smart strategy (not xz) since it's below threshold
        assert_ne!(strategy.algorithm, Algorithm::Xz);

        // Test file above threshold
        let strategy = determine_compression_for_entry(&test_file, 150 * 1024 * 1024, &config);
        assert_eq!(strategy.algorithm, Algorithm::Xz);
        assert_eq!(strategy.level, 7);
        assert_eq!(strategy.threads, 1); // XZ should always use single thread
    }
}
