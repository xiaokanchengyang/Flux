//! Archive operations module

pub mod extractor;
pub mod incremental;
pub mod modifier;
pub mod secure_extractor;
pub mod sevenz;
pub mod sevenz_extractor;
pub mod tar;
pub mod tar_extractor;
pub mod zip;
pub mod zip_extractor;

use crate::strategy::{Algorithm, CompressionStrategy};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// Archive entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// Path within the archive
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
}

/// Pack files into an archive
pub fn pack<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    format: Option<&str>,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    // For now, we only support tar format
    let format = format.unwrap_or("tar");

    match format {
        "tar" => tar::pack_tar(input, output),
        "zip" => zip::pack_zip(input, output),
        "7z" => sevenz::pack_7z(input, output),
        _ => Err(Error::UnsupportedFormat(format.to_string())),
    }
}

/// Extract files from an archive
pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(archive: P, output_dir: Q) -> Result<()> {
    let archive = archive.as_ref();
    let output_dir = output_dir.as_ref();

    // Detect format by extension
    let ext = archive
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Check for double extensions
    let stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let double_ext = if stem.ends_with(".tar") {
        format!("tar.{}", ext)
    } else {
        ext.to_string()
    };

    match double_ext.as_str() {
        "tar" => tar::extract_tar(archive, output_dir),
        "tar.gz" | "tgz" => tar::extract_tar_compressed(archive, output_dir, Algorithm::Gzip),
        "tar.zst" | "tzst" => tar::extract_tar_compressed(archive, output_dir, Algorithm::Zstd),
        "tar.xz" | "txz" => tar::extract_tar_compressed(archive, output_dir, Algorithm::Xz),
        "tar.br" => tar::extract_tar_compressed(archive, output_dir, Algorithm::Brotli),
        _ => match ext {
            "tar" => tar::extract_tar(archive, output_dir),
            "gz" if stem.ends_with(".tar") => {
                tar::extract_tar_compressed(archive, output_dir, Algorithm::Gzip)
            }
            "zst" if stem.ends_with(".tar") => {
                tar::extract_tar_compressed(archive, output_dir, Algorithm::Zstd)
            }
            "xz" if stem.ends_with(".tar") => {
                tar::extract_tar_compressed(archive, output_dir, Algorithm::Xz)
            }
            "br" if stem.ends_with(".tar") => {
                tar::extract_tar_compressed(archive, output_dir, Algorithm::Brotli)
            }
            "zip" => zip::extract_zip(archive, output_dir),
            "7z" => sevenz::extract_7z(archive, output_dir),
            _ => Err(Error::UnsupportedFormat(ext.to_string())),
        },
    }
}

/// Inspect archive contents without extracting
pub fn inspect<P: AsRef<Path>>(archive: P) -> Result<Vec<ArchiveEntry>> {
    let archive = archive.as_ref();

    // Detect format by extension
    let ext = archive
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Check for double extensions
    let stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let double_ext = if stem.ends_with(".tar") {
        format!("tar.{}", ext)
    } else {
        ext.to_string()
    };

    match double_ext.as_str() {
        "tar" => tar::inspect_tar(archive),
        "tar.gz" | "tgz" => tar::inspect_tar_compressed(archive, Algorithm::Gzip),
        "tar.zst" | "tzst" => tar::inspect_tar_compressed(archive, Algorithm::Zstd),
        "tar.xz" | "txz" => tar::inspect_tar_compressed(archive, Algorithm::Xz),
        "tar.br" => tar::inspect_tar_compressed(archive, Algorithm::Brotli),
        _ => match ext {
            "tar" => tar::inspect_tar(archive),
            "gz" if stem.ends_with(".tar") => tar::inspect_tar_compressed(archive, Algorithm::Gzip),
            "zst" if stem.ends_with(".tar") => {
                tar::inspect_tar_compressed(archive, Algorithm::Zstd)
            }
            "xz" if stem.ends_with(".tar") => tar::inspect_tar_compressed(archive, Algorithm::Xz),
            "br" if stem.ends_with(".tar") => {
                tar::inspect_tar_compressed(archive, Algorithm::Brotli)
            }
            "zip" => zip::inspect_zip(archive),
            "7z" => sevenz::inspect_7z(archive),
            _ => Err(Error::UnsupportedFormat(ext.to_string())),
        },
    }
}

/// Create an extractor for the given archive file
pub fn create_extractor(path: &Path) -> Result<Box<dyn extractor::Extractor>> {
    create_extractor_inner(path, false)
}

/// Create a secure extractor for the given archive file with security checks
pub fn create_secure_extractor(path: &Path) -> Result<Box<dyn extractor::Extractor>> {
    create_extractor_inner(path, true)
}

/// Internal function to create extractor with optional security wrapper
fn create_extractor_inner(path: &Path, secure: bool) -> Result<Box<dyn extractor::Extractor>> {
    // Detect format by extension
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    // Check for double extensions
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let double_ext = if stem.ends_with(".tar") {
        format!("tar.{}", ext)
    } else {
        ext.to_string()
    };

    let base_extractor: Box<dyn extractor::Extractor> = match double_ext.as_str() {
        "tar" => Box::new(tar_extractor::TarExtractor::new()),
        "tar.gz" | "tgz" => Box::new(tar_extractor::TarExtractor::with_compression(
            Algorithm::Gzip,
        )),
        "tar.zst" | "tzst" => Box::new(tar_extractor::TarExtractor::with_compression(
            Algorithm::Zstd,
        )),
        "tar.xz" | "txz" => Box::new(tar_extractor::TarExtractor::with_compression(Algorithm::Xz)),
        "tar.br" => Box::new(tar_extractor::TarExtractor::with_compression(
            Algorithm::Brotli,
        )),
        _ => match ext {
            "tar" => Box::new(tar_extractor::TarExtractor::new()),
            "gz" if stem.ends_with(".tar") => Box::new(
                tar_extractor::TarExtractor::with_compression(Algorithm::Gzip),
            ),
            "zst" if stem.ends_with(".tar") => Box::new(
                tar_extractor::TarExtractor::with_compression(Algorithm::Zstd),
            ),
            "xz" if stem.ends_with(".tar") => {
                Box::new(tar_extractor::TarExtractor::with_compression(Algorithm::Xz))
            }
            "br" if stem.ends_with(".tar") => Box::new(
                tar_extractor::TarExtractor::with_compression(Algorithm::Brotli),
            ),
            "zip" => Box::new(zip_extractor::ZipExtractor::new()),
            "7z" => Box::new(sevenz_extractor::SevenZExtractor::new()),
            _ => return Err(Error::UnsupportedFormat(ext.to_string())),
        },
    };

    if secure {
        Ok(Box::new(secure_extractor::SecureExtractor::new(
            base_extractor,
        )))
    } else {
        Ok(base_extractor)
    }
}

/// Pack options for archive creation
///
/// When packing multiple small files (< 1KB), the library automatically
/// uses tar format first before applying compression. This "batch then compress"
/// approach significantly improves compression ratio and performance for
/// directories with many small files.
///
/// # Small File Batching Strategy
///
/// The library employs an intelligent batching strategy for small files:
///
/// 1. **Detection**: Files smaller than 1KB are identified during directory scanning
/// 2. **Batching**: These files are first archived into a tar format without compression
/// 3. **Compression**: The entire tar archive is then compressed using the selected algorithm
///
/// This approach provides several benefits:
/// - Better compression ratios (small files share dictionary/context)
/// - Reduced metadata overhead
/// - Faster compression/decompression
/// - More efficient memory usage
///
/// # Example
///
/// ```no_run
/// use flux_core::archive::{pack, PackOptions};
///
/// // Packing a directory with many small config files
/// let options = PackOptions {
///     smart: true,  // Enables intelligent batching
///     ..Default::default()
/// };
///
/// // The library will automatically batch small files
/// // pack("config_dir", "configs.tar.zst", options)?;
/// # Ok::<(), flux_core::Error>(())
/// ```
pub struct PackOptions {
    /// Enable smart compression strategy
    pub smart: bool,
    /// Compression algorithm (overrides smart strategy)
    pub algorithm: Option<String>,
    /// Compression level
    pub level: Option<u32>,
    /// Number of threads
    pub threads: Option<usize>,
    /// Force compression on already compressed files
    pub force_compress: bool,
    /// Follow symlinks (pack link targets instead of links)
    pub follow_symlinks: bool,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            smart: true,
            algorithm: None,
            level: None,
            threads: None,
            force_compress: false,
            follow_symlinks: false,
        }
    }
}

/// Extract options
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Overwrite existing files
    pub overwrite: bool,
    /// Skip existing files (default)
    pub skip: bool,
    /// Rename files if they exist
    pub rename: bool,
    /// Remove the specified number of leading path elements
    pub strip_components: Option<usize>,
    /// If the archive contains a single folder, hoist its contents to the output directory
    pub hoist: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            overwrite: false,
            skip: true,
            rename: false,
            strip_components: None,
            hoist: false,
        }
    }
}

/// Pack files with compression strategy
pub fn pack_with_strategy<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    format: Option<&str>,
    options: PackOptions,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    // Determine compression strategy
    let mut strategy = if options.smart && options.algorithm.is_none() {
        // Use smart strategy
        if input.is_dir() {
            CompressionStrategy::smart_for_directory(input, options.level, options.threads)?
        } else {
            CompressionStrategy::smart(input, options.level, options.threads)
        }
    } else if let Some(algo_str) = &options.algorithm {
        // Use specified algorithm
        let algorithm = algo_str
            .parse::<Algorithm>()
            .map_err(|_| Error::UnsupportedFormat(format!("Unknown algorithm: {}", algo_str)))?;
        CompressionStrategy {
            algorithm,
            level: options.level.unwrap_or(3),
            threads: options.threads.unwrap_or_else(rayon::current_num_threads),
            force_compress: options.force_compress,
            long_mode: false,
        }
    } else {
        // Use default strategy
        CompressionStrategy::default()
    };

    strategy.force_compress = options.force_compress;

    // Get file size for thread adjustment
    let file_size = if input.is_file() {
        input
            .metadata()
            .map(|m| m.len())
            .unwrap_or(100 * 1024 * 1024)
    } else {
        // For directories, estimate based on total size
        100 * 1024 * 1024 // Default to 100MB
    };
    strategy.adjust_for_parallel(file_size);

    info!("Using compression strategy: {:?}", strategy);

    // Determine output format
    let format = if let Some(fmt) = format {
        fmt.to_string()
    } else {
        // Infer from output filename
        let ext = output
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        // Check for double extensions
        if let Some(stem) = output.file_stem().and_then(|s| s.to_str()) {
            if stem.ends_with(".tar") {
                format!("tar.{}", ext)
            } else if matches!(ext, "gz" | "zst" | "xz" | "br") {
                // These are compression extensions, assume tar
                format!("tar.{}", ext)
            } else if ext == "tar" {
                "tar".to_string()
            } else {
                // No clear format, use smart default based on algorithm
                match strategy.algorithm {
                    Algorithm::Gzip => "tar.gz",
                    Algorithm::Zstd => "tar.zst",
                    Algorithm::Xz => "tar.xz",
                    Algorithm::Brotli => "tar.br",
                    Algorithm::Store => "tar",
                }
                .to_string()
            }
        } else {
            // No clear format, use smart default based on algorithm
            match strategy.algorithm {
                Algorithm::Gzip => "tar.gz",
                Algorithm::Zstd => "tar.zst",
                Algorithm::Xz => "tar.xz",
                Algorithm::Brotli => "tar.br",
                Algorithm::Store => "tar",
            }
            .to_string()
        }
    };

    // Support both tar and zip formats
    match format.as_str() {
        "tar" => tar::pack_tar_with_options(input, output, options.follow_symlinks),
        "tar.gz" | "tgz" => tar::pack_tar_compressed_with_options(
            input,
            output,
            Algorithm::Gzip,
            strategy.level,
            options.follow_symlinks,
        ),
        "tar.zst" | "tzst" => tar::pack_tar_compressed_with_options(
            input,
            output,
            Algorithm::Zstd,
            strategy.level,
            options.follow_symlinks,
        ),
        "tar.xz" | "txz" => tar::pack_tar_compressed_with_options(
            input,
            output,
            Algorithm::Xz,
            strategy.level,
            options.follow_symlinks,
        ),
        "tar.br" => tar::pack_tar_compressed_with_options(
            input,
            output,
            Algorithm::Brotli,
            strategy.level,
            options.follow_symlinks,
        ),
        "zip" => zip::pack_zip_with_options(input, output, options.follow_symlinks),
        "7z" => sevenz::pack_7z(input, output), // Note: 7z packing not yet supported
        _ => Err(Error::UnsupportedFormat(format)),
    }
}

/// Extract files from an archive with options
pub fn extract_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    archive: P,
    output_dir: Q,
    options: ExtractOptions,
) -> Result<()> {
    let archive = archive.as_ref();
    let output_dir = output_dir.as_ref();

    // Store whether hoist is enabled before moving options
    let should_hoist = options.hoist;

    // Detect format by extension
    let ext = archive
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Check for double extensions
    let stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let double_ext = if stem.ends_with(".tar") {
        format!("tar.{}", ext)
    } else {
        ext.to_string()
    };

    // Perform the extraction
    let result = match double_ext.as_str() {
        "tar" => tar::extract_tar_with_options(archive, output_dir, options),
        "tar.gz" | "tgz" => {
            tar::extract_tar_compressed_with_options(archive, output_dir, Algorithm::Gzip, options)
        }
        "tar.zst" | "tzst" => {
            tar::extract_tar_compressed_with_options(archive, output_dir, Algorithm::Zstd, options)
        }
        "tar.xz" | "txz" => {
            tar::extract_tar_compressed_with_options(archive, output_dir, Algorithm::Xz, options)
        }
        "tar.br" => tar::extract_tar_compressed_with_options(
            archive,
            output_dir,
            Algorithm::Brotli,
            options,
        ),
        _ => match ext {
            "tar" => tar::extract_tar_with_options(archive, output_dir, options),
            "gz" if stem.ends_with(".tar") => tar::extract_tar_compressed_with_options(
                archive,
                output_dir,
                Algorithm::Gzip,
                options,
            ),
            "zst" if stem.ends_with(".tar") => tar::extract_tar_compressed_with_options(
                archive,
                output_dir,
                Algorithm::Zstd,
                options,
            ),
            "xz" if stem.ends_with(".tar") => tar::extract_tar_compressed_with_options(
                archive,
                output_dir,
                Algorithm::Xz,
                options,
            ),
            "br" if stem.ends_with(".tar") => tar::extract_tar_compressed_with_options(
                archive,
                output_dir,
                Algorithm::Brotli,
                options,
            ),
            "zip" => zip::extract_zip_with_options(archive, output_dir, options),
            "7z" => sevenz::extract_7z_with_options(archive, output_dir, options),
            _ => Err(Error::UnsupportedFormat(ext.to_string())),
        },
    };

    // If extraction succeeded and hoist is enabled, perform directory hoisting
    if result.is_ok() && should_hoist {
        if let Err(e) = hoist_single_directory(output_dir) {
            info!("Directory hoisting failed: {}", e);
            // We don't fail the entire operation if hoisting fails
        }
    }

    result
}

/// Hoist the contents of a single subdirectory to the parent directory
///
/// This function checks if the output directory contains exactly one subdirectory,
/// and if so, moves all contents of that subdirectory up one level and removes
/// the now-empty subdirectory.
pub fn hoist_single_directory(output_dir: &Path) -> Result<()> {
    use std::fs;

    // Ensure the output directory exists
    if !output_dir.exists() {
        return Ok(());
    }

    // Read the directory entries
    let entries: Vec<_> = fs::read_dir(output_dir)?.filter_map(|e| e.ok()).collect();

    // Check if there's exactly one entry and it's a directory
    if entries.len() == 1 {
        let entry = &entries[0];
        let entry_path = entry.path();

        if entry_path.is_dir() {
            info!("Found single directory to hoist: {:?}", entry_path);

            // Move all contents from the subdirectory to the parent
            let subdir_entries = fs::read_dir(&entry_path)?;

            for sub_entry in subdir_entries {
                let sub_entry = sub_entry?;
                let source = sub_entry.path();
                let dest_name = source
                    .file_name()
                    .ok_or_else(|| Error::Other("Invalid filename".to_string()))?;
                let dest = output_dir.join(dest_name);

                info!("Moving {:?} to {:?}", source, dest);
                fs::rename(&source, &dest)?;
            }

            // Remove the now-empty directory
            fs::remove_dir(&entry_path)?;
            info!("Removed empty directory: {:?}", entry_path);
        }
    }

    Ok(())
}
