//! Archive operations module

pub mod tar;
pub mod zip;

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
            _ => Err(Error::UnsupportedFormat(ext.to_string())),
        },
    }
}

/// Pack options
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
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            overwrite: false,
            skip: true,
            rename: false,
            strip_components: None,
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
        let algorithm = algo_str.parse::<Algorithm>()
            .map_err(|_| Error::UnsupportedFormat(format!("Unknown algorithm: {}", algo_str)))?;
        CompressionStrategy {
            algorithm,
            level: options.level.unwrap_or(3),
            threads: options.threads.unwrap_or_else(rayon::current_num_threads),
            force_compress: options.force_compress,
        }
    } else {
        // Use default strategy
        CompressionStrategy::default()
    };

    strategy.force_compress = options.force_compress;
    strategy.adjust_for_parallel();

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
            _ => Err(Error::UnsupportedFormat(ext.to_string())),
        },
    }
}
