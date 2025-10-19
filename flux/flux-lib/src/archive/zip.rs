//! Zip archive operations

use crate::archive::{ArchiveEntry, ExtractOptions};
use crate::{Error, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::{ZipArchive, ZipWriter};

/// Pack files into a zip archive
pub fn pack_zip<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    pack_zip_with_options(input, output, false)
}

/// Pack files into a zip archive with options
pub fn pack_zip_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    follow_symlinks: bool,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    info!(
        "Packing {:?} into ZIP {:?} (follow_symlinks: {})",
        input, output, follow_symlinks
    );

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    if input.is_file() {
        // Pack single file
        let file_name = input.file_name().unwrap().to_string_lossy();
        pack_file_to_zip(&mut zip, input, &file_name, options)?;
    } else if input.is_dir() {
        // Pack directory recursively
        pack_directory_to_zip(&mut zip, input, follow_symlinks)?;
    } else {
        return Err(Error::InvalidPath(format!(
            "{:?} is neither a file nor a directory",
            input
        )));
    }

    zip.finish()?;
    info!("Successfully packed ZIP archive: {:?}", output);

    Ok(())
}

/// Pack a single file into the zip
fn pack_file_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    path: &Path,
    name: &str,
    options: FileOptions<'static, ()>,
) -> Result<()> {
    debug!("Adding file to ZIP: {:?} as {}", path, name);

    let mut file = File::open(path)?;
    let metadata = file.metadata()?;

    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::PermissionsExt;
        options.unix_permissions(metadata.permissions().mode())
    };

    // Note: zip crate's FileOptions handles last modified time automatically from file metadata

    zip.start_file(name, options)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;

    Ok(())
}

/// Pack a directory into the zip
fn pack_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    follow_symlinks: bool,
) -> Result<()> {
    let base_path = dir.canonicalize()?;

    for entry in WalkDir::new(dir).follow_links(follow_symlinks) {
        let entry = entry.map_err(|e| Error::Other(e.to_string()))?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| Error::Other(e.to_string()))?;

        // Get relative path
        let relative_path = path
            .strip_prefix(&base_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"); // Ensure forward slashes in ZIP

        if metadata.is_dir() {
            // Add directory entry
            let dir_name = format!("{}/", relative_path);
            debug!("Adding directory to ZIP: {}", dir_name);

            let options =
                FileOptions::<'static, ()>::default().compression_method(CompressionMethod::Stored);

            #[cfg(unix)]
            let options = {
                use std::os::unix::fs::PermissionsExt;
                options.unix_permissions(metadata.permissions().mode())
            };

            zip.add_directory(&dir_name, options)?;
        } else if metadata.is_file() {
            // Add file
            let options = FileOptions::<'static, ()>::default()
                .compression_method(CompressionMethod::Deflated);

            pack_file_to_zip(zip, path, &relative_path, options)?;
        } else if metadata.is_symlink() && !follow_symlinks {
            warn!("ZIP format does not support symlinks, skipping: {:?}", path);
        }
    }

    Ok(())
}

/// Extract files from a zip archive
pub fn extract_zip<P: AsRef<Path>, Q: AsRef<Path>>(archive_path: P, output_dir: Q) -> Result<()> {
    extract_zip_with_options(archive_path, output_dir, ExtractOptions::default())
}

/// Extract files from a zip archive with options
pub fn extract_zip_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    archive_path: P,
    output_dir: Q,
    options: ExtractOptions,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    info!(
        "Extracting ZIP {:?} to {:?} with options: {:?}",
        archive_path, output_dir, options
    );

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Apply strip components
        let outpath = if let Some(strip) = options.strip_components {
            let components: Vec<_> = outpath.components().collect();
            if components.len() <= strip {
                // Skip this entry if we're stripping more components than it has
                continue;
            }
            PathBuf::from_iter(components.into_iter().skip(strip))
        } else {
            outpath
        };

        let dest_path = output_dir.join(&outpath);

        debug!("Extracting: {:?}", outpath);

        if file.name().ends_with('/') {
            // Directory
            fs::create_dir_all(&dest_path)?;
        } else {
            // File
            // Create parent directories if needed
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Handle existing files
            if dest_path.exists() {
                if options.skip {
                    info!("Skipping existing file: {:?}", dest_path);
                    continue;
                } else if options.rename {
                    let dest_path = get_unique_filename(&dest_path);
                    info!("Renaming to avoid conflict: {:?}", dest_path);
                    extract_zip_file(&mut file, &dest_path)?;
                } else if options.overwrite {
                    info!("Overwriting existing file: {:?}", dest_path);
                    extract_zip_file(&mut file, &dest_path)?;
                }
            } else {
                extract_zip_file(&mut file, &dest_path)?;
            }
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&dest_path, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    info!("Successfully extracted ZIP archive");
    Ok(())
}

/// Extract a single file from zip
fn extract_zip_file<R: Read>(file: &mut R, dest_path: &Path) -> Result<()> {
    let mut outfile = File::create(dest_path)?;
    std::io::copy(file, &mut outfile)?;
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

/// Inspect zip archive contents without extracting
pub fn inspect_zip<P: AsRef<Path>>(archive_path: P) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive_path.as_ref();
    info!("Inspecting ZIP archive: {:?}", archive_path);

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let entry = ArchiveEntry {
            path,
            size: file.size(),
            compressed_size: Some(file.compressed_size()),
            mode: file.unix_mode(),
            mtime: file.last_modified().map(|dt| dt.timepart() as i64),
            is_dir: file.is_dir(),
            is_symlink: false, // ZIP doesn't support symlinks
            link_target: None,
        };

        entries.push(entry);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pack_extract_zip() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        let archive_path = temp_dir.path().join("test.zip");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test file
        fs::write(&test_file, b"Test content")?;

        // Pack and extract
        pack_zip(&test_file, &archive_path)?;
        extract_zip(&archive_path, &extract_dir)?;

        // Verify
        let extracted_file = extract_dir.join("test.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read(&extracted_file)?, b"Test content");

        Ok(())
    }
}
