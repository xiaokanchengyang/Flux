//! Tar archive operations

use crate::archive::{ArchiveEntry, ExtractOptions};
use crate::metadata::FileMetadata;
use crate::strategy::Algorithm;
use crate::{Error, Result};
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use tracing::{debug, info, warn};
use walkdir::WalkDir;
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

/// Pack files into a tar archive
pub fn pack_tar<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    pack_tar_with_options(input, output, false)
}

/// Pack files into a tar archive with options
pub fn pack_tar_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    follow_symlinks: bool,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    info!(
        "Packing {:?} into {:?} (follow_symlinks: {})",
        input, output, follow_symlinks
    );

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output)?;
    let mut builder = Builder::new(file);

    if input.is_file() {
        // Pack single file
        pack_file(
            &mut builder,
            input,
            Path::new(input.file_name().unwrap()),
            follow_symlinks,
        )?;
    } else if input.is_dir() {
        // Pack directory recursively
        pack_directory_with_options(&mut builder, input, follow_symlinks)?;
    } else {
        return Err(Error::InvalidPath(format!(
            "{:?} is neither a file nor a directory",
            input
        )));
    }

    builder.finish()?;
    info!("Successfully packed archive: {:?}", output);

    Ok(())
}

/// Pack a single file into the tar builder
fn pack_file<W: Write>(
    builder: &mut Builder<W>,
    path: &Path,
    archive_path: &Path,
    follow_symlinks: bool,
) -> Result<()> {
    debug!("Adding file: {:?} as {:?}", path, archive_path);

    let file_metadata = path.symlink_metadata()?;

    // Check if it's a symlink
    #[cfg(unix)]
    if file_metadata.file_type().is_symlink() && !follow_symlinks {
        // Pack the symlink itself
        let link_target = fs::read_link(path)?;
        debug!("Adding symlink: {:?} -> {:?}", path, link_target);

        let metadata = FileMetadata::from_metadata(&file_metadata)?;
        let mut header = tar::Header::new_ustar();

        header.set_entry_type(tar::EntryType::Symlink);
        header.set_path(archive_path)?;
        header.set_link_name(&link_target)?;
        header.set_size(0);

        // Set Unix-specific metadata
        #[cfg(unix)]
        {
            if let Some(mode) = metadata.mode {
                header.set_mode(mode);
            }
            if let Some(uid) = metadata.uid {
                header.set_uid(uid as u64);
            }
            if let Some(gid) = metadata.gid {
                header.set_gid(gid as u64);
            }
        }

        // Set timestamps
        if let Some(mtime) = metadata.modified {
            if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
                header.set_mtime(duration.as_secs());
            }
        }

        header.set_cksum();
        builder.append(&header, &mut std::io::empty())?;
        return Ok(());
    }

    // Regular file handling
    let metadata = FileMetadata::from_path(path)?;
    let mut file = File::open(path)?;
    let mut header = tar::Header::new_ustar();

    // Set basic metadata
    header.set_size(file.metadata()?.len());
    header.set_path(archive_path)?;

    // Set Unix-specific metadata
    #[cfg(unix)]
    {
        if let Some(mode) = metadata.mode {
            header.set_mode(mode);
        }
        if let Some(uid) = metadata.uid {
            header.set_uid(uid as u64);
        }
        if let Some(gid) = metadata.gid {
            header.set_gid(gid as u64);
        }
    }

    // Set timestamps
    if let Some(mtime) = metadata.modified {
        if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
            header.set_mtime(duration.as_secs());
        }
    }

    // Calculate and set checksum
    header.set_cksum();

    builder.append(&header, &mut file)?;
    Ok(())
}

/// Pack a directory recursively into the tar builder with options
fn pack_directory_with_options<W: Write>(
    builder: &mut Builder<W>,
    dir: &Path,
    follow_symlinks: bool,
) -> Result<()> {
    let base_path = dir.parent().unwrap_or(Path::new(""));

    let walker = if follow_symlinks {
        WalkDir::new(dir).follow_links(true).max_depth(100) // Prevent infinite recursion
    } else {
        WalkDir::new(dir).follow_links(false)
    };

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // Handle WalkDir errors (e.g., symlink loops)
                warn!("Error walking directory: {}", e);
                // Check if it's a loop error
                if e.io_error().is_some() && e.path().is_some() {
                    let path = e.path().unwrap();
                    if e.loop_ancestor().is_some() {
                        return Err(Error::Archive(format!(
                            "Symlink loop detected at {:?}",
                            path
                        )));
                    }
                }
                continue; // Skip this entry
            }
        };
        let path = entry.path();

        // Skip the directory itself
        if path == dir {
            continue;
        }

        // Calculate relative path for the archive
        let relative_path = path
            .strip_prefix(base_path)
            .map_err(|_| Error::InvalidPath(format!("Failed to strip prefix from {:?}", path)))?;

        let file_type = entry.file_type();

        if file_type.is_file() || (file_type.is_symlink() && follow_symlinks) {
            pack_file(builder, path, relative_path, follow_symlinks)?;
        } else if file_type.is_dir() {
            // Add directory entry
            debug!("Adding directory: {:?}", relative_path);
            let metadata = FileMetadata::from_path(path)?;
            let mut header = tar::Header::new_ustar();

            header.set_entry_type(tar::EntryType::Directory);
            header.set_path(relative_path)?;
            header.set_size(0);

            #[cfg(unix)]
            {
                if let Some(mode) = metadata.mode {
                    header.set_mode(mode);
                }
                if let Some(uid) = metadata.uid {
                    header.set_uid(uid as u64);
                }
                if let Some(gid) = metadata.gid {
                    header.set_gid(gid as u64);
                }
            }

            // Calculate and set checksum
            header.set_cksum();

            builder.append(&header, &mut std::io::empty())?;
        } else if file_type.is_symlink() && !follow_symlinks {
            // Handle symlinks when not following them
            pack_file(builder, path, relative_path, follow_symlinks)?;
        } else {
            warn!("Skipping special file: {:?}", path);
        }
    }

    Ok(())
}

/// Extract files from a tar archive
pub fn extract_tar<P: AsRef<Path>, Q: AsRef<Path>>(archive_path: P, output_dir: Q) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    info!("Extracting {:?} to {:?}", archive_path, output_dir);

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    // Extract all entries
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let dest_path = output_dir.join(&path);

        debug!("Extracting: {:?}", path);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the entry
        entry.unpack(&dest_path)?;

        // Try to preserve metadata
        let header = entry.header().clone();
        apply_tar_metadata(&dest_path, &header);
    }

    info!("Successfully extracted archive");
    Ok(())
}

/// Apply metadata from tar header to extracted file
fn apply_tar_metadata(path: &Path, header: &tar::Header) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Set permissions
        if let Ok(mode) = header.mode() {
            if let Err(e) = fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
                debug!("Failed to set permissions on {:?}: {}", path, e);
            }
        }
    }

    // Set modification time
    if let Ok(mtime) = header.mtime() {
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime);
        if let Err(e) = filetime::set_file_mtime(path, filetime::FileTime::from_system_time(mtime))
        {
            debug!("Failed to set mtime on {:?}: {}", path, e);
        }
    }
}

/// Inspect tar archive contents without extracting
pub fn inspect_tar<P: AsRef<Path>>(archive_path: P) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive_path.as_ref();
    info!("Inspecting tar archive: {:?}", archive_path);

    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);
    let mut entries = Vec::new();

    // Read all entries
    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;

        let archive_entry = ArchiveEntry {
            path: path.to_path_buf(),
            size: header.size()?,
            compressed_size: None, // TAR doesn't compress individual files
            mode: header.mode().ok(),
            mtime: header.mtime().ok().map(|t| t as i64),
            is_dir: header.entry_type() == tar::EntryType::Directory,
            is_symlink: header.entry_type() == tar::EntryType::Symlink,
            link_target: if header.entry_type() == tar::EntryType::Symlink {
                header.link_name()?.map(|p| p.to_path_buf())
            } else {
                None
            },
        };

        entries.push(archive_entry);
    }

    info!("Found {} entries in archive", entries.len());
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_single_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        let archive_path = temp_dir.path().join("test.tar");

        // Create test file
        fs::write(&test_file, b"Hello, World!")?;

        // Pack the file
        pack_tar(&test_file, &archive_path)?;

        // Verify archive exists
        assert!(archive_path.exists());
        assert!(archive_path.metadata()?.len() > 0);

        Ok(())
    }

    #[test]
    fn test_pack_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_dir = temp_dir.path().join("test_dir");
        let archive_path = temp_dir.path().join("test.tar");

        // Create test directory structure
        fs::create_dir_all(&test_dir)?;
        fs::write(test_dir.join("file1.txt"), b"File 1")?;
        fs::write(test_dir.join("file2.txt"), b"File 2")?;
        fs::create_dir(test_dir.join("subdir"))?;
        fs::write(test_dir.join("subdir").join("file3.txt"), b"File 3")?;

        // Pack the directory
        pack_tar(&test_dir, &archive_path)?;

        // Verify archive exists
        assert!(archive_path.exists());
        assert!(archive_path.metadata()?.len() > 0);

        Ok(())
    }

    #[test]
    fn test_extract_archive() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_dir = temp_dir.path().join("test_dir");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test directory structure
        fs::create_dir_all(&test_dir)?;
        fs::write(test_dir.join("file1.txt"), b"File 1")?;
        fs::write(test_dir.join("file2.txt"), b"File 2")?;

        // Pack and then extract
        pack_tar(&test_dir, &archive_path)?;
        extract_tar(&archive_path, &extract_dir)?;

        // Verify extracted files
        assert!(extract_dir.join("test_dir").join("file1.txt").exists());
        assert!(extract_dir.join("test_dir").join("file2.txt").exists());

        // Verify content
        let content1 = fs::read(extract_dir.join("test_dir").join("file1.txt"))?;
        assert_eq!(content1, b"File 1");

        let content2 = fs::read(extract_dir.join("test_dir").join("file2.txt"))?;
        assert_eq!(content2, b"File 2");

        Ok(())
    }

    #[test]
    fn test_pack_extract_preserves_content() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test file with specific content
        let original_content = b"This is a test file with some content!";
        fs::write(&test_file, original_content)?;

        // Pack, extract, and verify
        pack_tar(&test_file, &archive_path)?;
        extract_tar(&archive_path, &extract_dir)?;

        let extracted_file = extract_dir.join("test.txt");
        assert!(extracted_file.exists());

        let extracted_content = fs::read(&extracted_file)?;
        assert_eq!(original_content, &extracted_content[..]);

        Ok(())
    }
}

/// Pack files into a compressed tar archive
pub fn pack_tar_compressed<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    algorithm: Algorithm,
    level: u32,
) -> Result<()> {
    pack_tar_compressed_with_options(input, output, algorithm, level, false)
}

/// Pack files into a compressed tar archive with options
pub fn pack_tar_compressed_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    algorithm: Algorithm,
    level: u32,
    follow_symlinks: bool,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    info!(
        "Packing {:?} into {:?} with {:?} compression",
        input, output, algorithm
    );

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output)?;

    match algorithm {
        Algorithm::Store => {
            // No compression, just create tar
            pack_tar_with_options(input, output, follow_symlinks)
        }
        Algorithm::Gzip => {
            let encoder = GzEncoder::new(file, GzCompression::new(level));
            let mut builder = Builder::new(encoder);

            if input.is_file() {
                pack_file(
                    &mut builder,
                    input,
                    Path::new(input.file_name().unwrap()),
                    follow_symlinks,
                )?;
            } else if input.is_dir() {
                pack_directory_with_options(&mut builder, input, follow_symlinks)?;
            } else {
                return Err(Error::InvalidPath(format!(
                    "{:?} is neither a file nor a directory",
                    input
                )));
            }

            let encoder = builder.into_inner()?;
            encoder.finish()?;
            info!("Successfully packed compressed archive: {:?}", output);
            Ok(())
        }
        Algorithm::Zstd => {
            let encoder = ZstdEncoder::new(file, level as i32)?;
            let mut builder = Builder::new(encoder);

            if input.is_file() {
                pack_file(
                    &mut builder,
                    input,
                    Path::new(input.file_name().unwrap()),
                    follow_symlinks,
                )?;
            } else if input.is_dir() {
                pack_directory_with_options(&mut builder, input, follow_symlinks)?;
            } else {
                return Err(Error::InvalidPath(format!(
                    "{:?} is neither a file nor a directory",
                    input
                )));
            }

            let encoder = builder.into_inner()?;
            encoder.finish()?;
            info!("Successfully packed compressed archive: {:?}", output);
            Ok(())
        }
        Algorithm::Xz => {
            let encoder = XzEncoder::new(file, level);
            let mut builder = Builder::new(encoder);

            if input.is_file() {
                pack_file(
                    &mut builder,
                    input,
                    Path::new(input.file_name().unwrap()),
                    follow_symlinks,
                )?;
            } else if input.is_dir() {
                pack_directory_with_options(&mut builder, input, follow_symlinks)?;
            } else {
                return Err(Error::InvalidPath(format!(
                    "{:?} is neither a file nor a directory",
                    input
                )));
            }

            let encoder = builder.into_inner()?;
            encoder.finish()?;
            info!("Successfully packed compressed archive: {:?}", output);
            Ok(())
        }
        Algorithm::Brotli => {
            let encoder = brotli::CompressorWriter::new(file, 4096, level, 22);
            let mut builder = Builder::new(encoder);

            if input.is_file() {
                pack_file(
                    &mut builder,
                    input,
                    Path::new(input.file_name().unwrap()),
                    follow_symlinks,
                )?;
            } else if input.is_dir() {
                pack_directory_with_options(&mut builder, input, follow_symlinks)?;
            } else {
                return Err(Error::InvalidPath(format!(
                    "{:?} is neither a file nor a directory",
                    input
                )));
            }

            builder.finish()?;
            info!("Successfully packed compressed archive: {:?}", output);
            Ok(())
        }
    }
}

/// Extract compressed tar archives
pub fn extract_tar_compressed<P: AsRef<Path>, Q: AsRef<Path>>(
    archive_path: P,
    output_dir: Q,
    algorithm: Algorithm,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    info!(
        "Extracting compressed {:?} archive {:?} to {:?}",
        algorithm, archive_path, output_dir
    );

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;

    match algorithm {
        Algorithm::Store => {
            // No compression
            extract_tar(archive_path, output_dir)
        }
        Algorithm::Gzip => {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            extract_archive_entries(&mut archive, output_dir)
        }
        Algorithm::Zstd => {
            let decoder = zstd::stream::read::Decoder::new(file)?;
            let mut archive = Archive::new(decoder);
            extract_archive_entries(&mut archive, output_dir)
        }
        Algorithm::Xz => {
            let decoder = xz2::read::XzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            extract_archive_entries(&mut archive, output_dir)
        }
        Algorithm::Brotli => {
            let decoder = brotli::Decompressor::new(file, 4096);
            let mut archive = Archive::new(decoder);
            extract_archive_entries(&mut archive, output_dir)
        }
    }
}

/// Extract entries from a tar archive reader
fn extract_archive_entries<R: Read>(archive: &mut Archive<R>, output_dir: &Path) -> Result<()> {
    // Extract all entries
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let dest_path = output_dir.join(&path);

        debug!("Extracting: {:?}", path);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the entry
        entry.unpack(&dest_path)?;

        // Try to preserve metadata
        let header = entry.header().clone();
        apply_tar_metadata(&dest_path, &header);
    }

    info!("Successfully extracted archive");
    Ok(())
}

/// Inspect compressed tar archive contents
pub fn inspect_tar_compressed<P: AsRef<Path>>(
    archive_path: P,
    algorithm: Algorithm,
) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive_path.as_ref();
    info!(
        "Inspecting compressed {:?} archive: {:?}",
        algorithm, archive_path
    );

    let file = File::open(archive_path)?;
    let mut entries = Vec::new();

    match algorithm {
        Algorithm::Store => {
            // No compression
            inspect_tar(archive_path)
        }
        Algorithm::Gzip => {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            read_archive_entries(&mut archive, &mut entries)?;
            Ok(entries)
        }
        Algorithm::Zstd => {
            let decoder = zstd::stream::read::Decoder::new(file)?;
            let mut archive = Archive::new(decoder);
            read_archive_entries(&mut archive, &mut entries)?;
            Ok(entries)
        }
        Algorithm::Xz => {
            let decoder = xz2::read::XzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            read_archive_entries(&mut archive, &mut entries)?;
            Ok(entries)
        }
        Algorithm::Brotli => {
            let decoder = brotli::Decompressor::new(file, 4096);
            let mut archive = Archive::new(decoder);
            read_archive_entries(&mut archive, &mut entries)?;
            Ok(entries)
        }
    }
}

/// Read entries from a tar archive reader
fn read_archive_entries<R: Read>(
    archive: &mut Archive<R>,
    entries: &mut Vec<ArchiveEntry>,
) -> Result<()> {
    // Read all entries
    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;

        let archive_entry = ArchiveEntry {
            path: path.to_path_buf(),
            size: header.size()?,
            compressed_size: None, // TAR doesn't compress individual files
            mode: header.mode().ok(),
            mtime: header.mtime().ok().map(|t| t as i64),
            is_dir: header.entry_type() == tar::EntryType::Directory,
            is_symlink: header.entry_type() == tar::EntryType::Symlink,
            link_target: if header.entry_type() == tar::EntryType::Symlink {
                header.link_name()?.map(|p| p.to_path_buf())
            } else {
                None
            },
        };

        entries.push(archive_entry);
    }

    info!("Found {} entries in archive", entries.len());
    Ok(())
}

/// Extract tar archive with options
pub fn extract_tar_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    archive_path: P,
    output_dir: Q,
    options: ExtractOptions,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    info!(
        "Extracting {:?} to {:?} with options: {:?}",
        archive_path, output_dir, options
    );

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    // Extract all entries
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Apply strip components
        let path = if let Some(strip) = options.strip_components {
            let components: Vec<_> = path.components().collect();
            if components.len() <= strip {
                // Skip this entry if we're stripping more components than it has
                continue;
            }
            PathBuf::from_iter(components.into_iter().skip(strip))
        } else {
            path.to_path_buf()
        };

        let dest_path = output_dir.join(&path);

        // Handle existing files
        if dest_path.exists() && !entry.header().entry_type().is_dir() {
            if options.skip {
                info!("Skipping existing file: {:?}", dest_path);
                continue;
            } else if options.rename {
                let dest_path = get_unique_filename(&dest_path);
                info!("Renaming to avoid conflict: {:?}", dest_path);
                extract_entry(&mut entry, &dest_path)?;
            } else if options.overwrite {
                info!("Overwriting existing file: {:?}", dest_path);
                extract_entry(&mut entry, &dest_path)?;
            }
        } else {
            extract_entry(&mut entry, &dest_path)?;
        }
    }

    info!("Successfully extracted archive");
    Ok(())
}

/// Extract compressed tar archive with options
pub fn extract_tar_compressed_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    archive_path: P,
    output_dir: Q,
    algorithm: Algorithm,
    options: ExtractOptions,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    info!(
        "Extracting compressed {:?} archive {:?} to {:?} with options",
        algorithm, archive_path, output_dir
    );

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;

    match algorithm {
        Algorithm::Store => {
            // No compression
            extract_tar_with_options(archive_path, output_dir, options)
        }
        Algorithm::Gzip => {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            extract_archive_entries_with_options(&mut archive, output_dir, options)
        }
        Algorithm::Zstd => {
            let decoder = zstd::stream::read::Decoder::new(file)?;
            let mut archive = Archive::new(decoder);
            extract_archive_entries_with_options(&mut archive, output_dir, options)
        }
        Algorithm::Xz => {
            let decoder = xz2::read::XzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            extract_archive_entries_with_options(&mut archive, output_dir, options)
        }
        Algorithm::Brotli => {
            let decoder = brotli::Decompressor::new(file, 4096);
            let mut archive = Archive::new(decoder);
            extract_archive_entries_with_options(&mut archive, output_dir, options)
        }
    }
}

/// Extract entries from a tar archive reader with options
fn extract_archive_entries_with_options<R: Read>(
    archive: &mut Archive<R>,
    output_dir: &Path,
    options: ExtractOptions,
) -> Result<()> {
    // Extract all entries
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Apply strip components
        let path = if let Some(strip) = options.strip_components {
            let components: Vec<_> = path.components().collect();
            if components.len() <= strip {
                // Skip this entry if we're stripping more components than it has
                continue;
            }
            PathBuf::from_iter(components.into_iter().skip(strip))
        } else {
            path.to_path_buf()
        };

        let dest_path = output_dir.join(&path);

        // Handle existing files
        if dest_path.exists() && !entry.header().entry_type().is_dir() {
            if options.skip {
                info!("Skipping existing file: {:?}", dest_path);
                continue;
            } else if options.rename {
                let dest_path = get_unique_filename(&dest_path);
                info!("Renaming to avoid conflict: {:?}", dest_path);
                extract_entry(&mut entry, &dest_path)?;
            } else if options.overwrite {
                info!("Overwriting existing file: {:?}", dest_path);
                extract_entry(&mut entry, &dest_path)?;
            }
        } else {
            extract_entry(&mut entry, &dest_path)?;
        }
    }

    info!("Successfully extracted archive");
    Ok(())
}

/// Extract a single entry to a destination path
fn extract_entry<R: Read>(entry: &mut tar::Entry<R>, dest_path: &Path) -> Result<()> {
    debug!("Extracting: {:?}", dest_path);

    let header = entry.header();
    let entry_type = header.entry_type();

    // Create parent directories if needed
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Handle different entry types
    match entry_type {
        tar::EntryType::Symlink => {
            // Extract symlink
            if let Some(link_target) = header.link_name()? {
                debug!("Creating symlink: {:?} -> {:?}", dest_path, link_target);

                // Remove existing file if it exists
                if dest_path.exists() {
                    fs::remove_file(dest_path).ok();
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs;
                    fs::symlink(&link_target, dest_path)?;
                }

                #[cfg(not(unix))]
                {
                    warn!(
                        "Symlinks are not supported on this platform, skipping: {:?}",
                        dest_path
                    );
                }
            }
        }
        _ => {
            // Regular file or directory
            entry.unpack(dest_path)?;

            // Try to preserve metadata
            let header = entry.header().clone();
            apply_tar_metadata(dest_path, &header);
        }
    }

    Ok(())
}

/// Get a unique filename by appending a number
fn get_unique_filename(path: &Path) -> PathBuf {
    let mut counter = 1;
    let stem = path.file_stem().unwrap_or_default();
    let extension = path.extension();
    let parent = path.parent().unwrap_or(Path::new(""));

    loop {
        let new_name = if let Some(ext) = extension {
            format!(
                "{} ({}).{}",
                stem.to_string_lossy(),
                counter,
                ext.to_string_lossy()
            )
        } else {
            format!("{} ({})", stem.to_string_lossy(), counter)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}
