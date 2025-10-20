//! 7z archive support module

use crate::archive::ArchiveEntry;
use crate::{Error, Result};
use sevenz_rust::{Archive, BlockDecoder, SevenZReader};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Pack files into a 7z archive
pub fn pack_7z<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    // Create parent directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    // For now, we'll return an error for packing as sevenz-rust doesn't support writing
    // This is a known limitation that we'll document
    Err(Error::UnsupportedOperation(
        "7z packing is not yet supported. Only extraction is available.".to_string(),
    ))
}

/// Extract files from a 7z archive
pub fn extract_7z<P: AsRef<Path>, Q: AsRef<Path>>(archive: P, output_dir: Q) -> Result<()> {
    let archive_path = archive.as_ref();
    let output_dir = output_dir.as_ref();

    info!("Extracting 7z archive: {:?}", archive_path);

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Open the archive
    let file = File::open(archive_path)?;
    let len = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mut archive = Archive::read(reader, len, None)
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    // Extract all entries
    let folder_count = archive.folders_count();
    for folder_index in 0..folder_count {
        let folder = archive
            .folder(folder_index)
            .map_err(|e| Error::ArchiveError(format!("Failed to read folder: {}", e)))?;
        
        let mut decoder = BlockDecoder::new(folder.index(), &mut archive);
        
        for entry in folder.files() {
            if entry.has_stream() {
                let path = output_dir.join(&entry.name());
                
                // Create parent directories
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                debug!("Extracting: {:?}", entry.name());

                // Read and write the file
                let mut output_file = File::create(&path)?;
                let mut buf = vec![0u8; 4096];
                
                loop {
                    let n = decoder
                        .read(&mut buf)
                        .map_err(|e| Error::ArchiveError(format!("Failed to read entry: {}", e)))?;
                    if n == 0 {
                        break;
                    }
                    output_file.write_all(&buf[..n])?;
                }

                // Set file permissions if available
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(attrs) = entry.attributes() {
                        if attrs & 0x10 == 0 {  // Not a directory
                            let mode = (attrs >> 16) & 0o777;
                            if mode != 0 {
                                fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
                            }
                        }
                    }
                }

                // Set modification time if available
                if let Some(mtime) = entry.last_modified_date() {
                    let mtime_secs = mtime / 10_000_000 - 11_644_473_600; // Convert from Windows ticks to Unix time
                    if mtime_secs > 0 {
                        filetime::set_file_mtime(
                            &path,
                            filetime::FileTime::from_unix_time(mtime_secs as i64, 0),
                        )?;
                    }
                }
            }
        }
    }

    info!("7z extraction complete");
    Ok(())
}

/// Extract files from a 7z archive with options
pub fn extract_7z_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    archive: P,
    output_dir: Q,
    options: crate::archive::ExtractOptions,
) -> Result<()> {
    let archive_path = archive.as_ref();
    let output_dir = output_dir.as_ref();

    info!("Extracting 7z archive with options: {:?}", archive_path);

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Open the archive
    let file = File::open(archive_path)?;
    let len = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mut archive = Archive::read(reader, len, None)
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    // Extract all entries
    let folder_count = archive.folders_count();
    for folder_index in 0..folder_count {
        let folder = archive
            .folder(folder_index)
            .map_err(|e| Error::ArchiveError(format!("Failed to read folder: {}", e)))?;
        
        let mut decoder = BlockDecoder::new(folder.index(), &mut archive);
        
        for entry in folder.files() {
            if entry.has_stream() {
                let entry_path = PathBuf::from(&entry.name());
                
                // Handle strip components
                let final_path = if let Some(strip) = options.strip_components {
                    let components: Vec<_> = entry_path.components().collect();
                    if components.len() <= strip {
                        continue; // Skip this entry
                    }
                    output_dir.join(components[strip..].iter().collect::<PathBuf>())
                } else {
                    output_dir.join(&entry_path)
                };

                // Handle existing files
                if final_path.exists() {
                    if options.skip {
                        debug!("Skipping existing file: {:?}", final_path);
                        continue;
                    } else if options.rename {
                        let mut counter = 1;
                        let mut new_path = final_path.clone();
                        while new_path.exists() {
                            let file_stem = final_path.file_stem().unwrap_or_default();
                            let extension = final_path.extension();
                            let new_name = if let Some(ext) = extension {
                                format!("{}_{}.{}", file_stem.to_string_lossy(), counter, ext.to_string_lossy())
                            } else {
                                format!("{}_{}", file_stem.to_string_lossy(), counter)
                            };
                            new_path = final_path.with_file_name(new_name);
                            counter += 1;
                        }
                        debug!("Renaming to: {:?}", new_path);
                    } else if !options.overwrite {
                        return Err(Error::FileExists(final_path));
                    }
                }

                // Create parent directories
                if let Some(parent) = final_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                debug!("Extracting: {:?}", entry.name());

                // Read and write the file
                let mut output_file = File::create(&final_path)?;
                let mut buf = vec![0u8; 4096];
                
                loop {
                    let n = decoder
                        .read(&mut buf)
                        .map_err(|e| Error::ArchiveError(format!("Failed to read entry: {}", e)))?;
                    if n == 0 {
                        break;
                    }
                    output_file.write_all(&buf[..n])?;
                }

                // Set file permissions if available
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(attrs) = entry.attributes() {
                        if attrs & 0x10 == 0 {  // Not a directory
                            let mode = (attrs >> 16) & 0o777;
                            if mode != 0 {
                                fs::set_permissions(&final_path, fs::Permissions::from_mode(mode))?;
                            }
                        }
                    }
                }

                // Set modification time if available
                if let Some(mtime) = entry.last_modified_date() {
                    let mtime_secs = mtime / 10_000_000 - 11_644_473_600; // Convert from Windows ticks to Unix time
                    if mtime_secs > 0 {
                        filetime::set_file_mtime(
                            &final_path,
                            filetime::FileTime::from_unix_time(mtime_secs as i64, 0),
                        )?;
                    }
                }
            }
        }
    }

    info!("7z extraction complete");
    Ok(())
}

/// Inspect 7z archive contents
pub fn inspect_7z<P: AsRef<Path>>(archive: P) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive.as_ref();

    debug!("Inspecting 7z archive: {:?}", archive_path);

    let file = File::open(archive_path)?;
    let len = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mut archive = Archive::read(reader, len, None)
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    let mut entries = Vec::new();

    // Iterate through all folders
    let folder_count = archive.folders_count();
    for folder_index in 0..folder_count {
        let folder = archive
            .folder(folder_index)
            .map_err(|e| Error::ArchiveError(format!("Failed to read folder: {}", e)))?;
        
        for entry in folder.files() {
            let is_dir = entry.attributes().map(|a| a & 0x10 != 0).unwrap_or(false);
            
            entries.push(ArchiveEntry {
                path: PathBuf::from(entry.name()),
                size: entry.size(),
                compressed_size: Some(entry.compressed_size()),
                mode: entry.attributes().map(|a| (a >> 16) & 0o777),
                mtime: entry.last_modified_date().map(|t| {
                    // Convert Windows ticks to Unix timestamp
                    let unix_secs = t / 10_000_000 - 11_644_473_600;
                    unix_secs as i64
                }),
                is_dir,
                is_symlink: false, // 7z doesn't support symlinks
                link_target: None,
            });
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_7z_not_supported_for_packing() {
        let temp_dir = TempDir::new().unwrap();
        let input = temp_dir.path().join("test.txt");
        let output = temp_dir.path().join("test.7z");
        
        fs::write(&input, "test content").unwrap();
        
        let result = pack_7z(&input, &output);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::UnsupportedOperation(_)));
    }
}