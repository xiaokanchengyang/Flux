//! 7z archive support module

use crate::archive::ArchiveEntry;
use crate::{Error, Result};
use sevenz_rust::{Password, SevenZReader};
use std::fs::{self, File};
use std::io::{Cursor, Write, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use std::borrow::Cow;

/// Pack files into a 7z archive
pub fn pack_7z<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    let _input = input.as_ref();
    let _output = output.as_ref();

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

    // Read the entire archive into memory
    let archive_data = fs::read(archive_path)?;
    let cursor = Cursor::new(archive_data);
    
    // Open the archive (no password)
    let mut sz = SevenZReader::new(cursor, archive_path.metadata()?.len(), Password::empty())
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    // Collect file information first
    let mut files_to_extract = Vec::new();
    {
        let archive = sz.archive();
        for idx in 0..archive.files.len() {
            let entry = &archive.files[idx];
            if !entry.is_directory() {
                files_to_extract.push((
                    entry.name.clone(),
                    output_dir.join(&entry.name),
                    entry.windows_attributes,
                    entry.last_modified_date.to_raw(),
                ));
            }
        }
    }

    // Extract files
    for (name, path, attrs, mtime_raw) in &files_to_extract {
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        debug!("Extracting: {:?}", name);

        // Create output file
        let output_file = File::create(path)?;
        let mut output_file = BufWriter::new(output_file);
        
        // Decompress directly to file
        let mut extracted = false;
        let name_clone = name.clone();
        sz.for_each_entries(|reader_entry, reader| {
            if reader_entry.name == name_clone && !extracted {
                std::io::copy(reader, &mut output_file)
                    .map_err(|e| sevenz_rust::Error::Io(e, Cow::Borrowed("Failed to copy data")))?;
                extracted = true;
            }
            Ok(true) // continue
        }).map_err(|e| Error::ArchiveError(format!("Failed to extract entry: {}", e)))?;
        
        output_file.flush()?;

        // Set file permissions if available
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if attrs & 0x10 == 0 {  // Not a directory
                let mode = (attrs >> 16) & 0o777;
                if mode != 0 {
                    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        // Set modification time if available
        let mtime_secs = mtime_raw / 10_000_000 - 11_644_473_600;
        if mtime_secs > 0 {
            filetime::set_file_mtime(
                path,
                filetime::FileTime::from_unix_time(mtime_secs as i64, 0),
            )?;
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

    // Read the entire archive into memory
    let archive_data = fs::read(archive_path)?;
    let cursor = Cursor::new(archive_data);
    
    // Open the archive (no password)
    let mut sz = SevenZReader::new(cursor, archive_path.metadata()?.len(), Password::empty())
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    // Collect file information first
    let mut files_to_extract = Vec::new();
    {
        let archive = sz.archive();
        for idx in 0..archive.files.len() {
            let entry = &archive.files[idx];
            if !entry.is_directory() {
                let entry_path = PathBuf::from(&entry.name);
                
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
                let mut actual_path = final_path.clone();
                if actual_path.exists() {
                    if options.skip {
                        debug!("Skipping existing file: {:?}", actual_path);
                        continue;
                    } else if options.rename {
                        let mut counter = 1;
                        while actual_path.exists() {
                            let file_stem = final_path.file_stem().unwrap_or_default();
                            let extension = final_path.extension();
                            let new_name = if let Some(ext) = extension {
                                format!("{}_{}.{}", file_stem.to_string_lossy(), counter, ext.to_string_lossy())
                            } else {
                                format!("{}_{}", file_stem.to_string_lossy(), counter)
                            };
                            actual_path = final_path.with_file_name(new_name);
                            counter += 1;
                        }
                        debug!("Renaming to: {:?}", actual_path);
                    } else if !options.overwrite {
                        return Err(Error::FileExists(final_path));
                    }
                }

                files_to_extract.push((
                    entry.name.clone(),
                    actual_path,
                    entry.windows_attributes,
                    entry.last_modified_date.to_raw(),
                ));
            }
        }
    }

    // Extract files
    for (name, path, attrs, mtime_raw) in &files_to_extract {
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        debug!("Extracting: {:?}", name);

        // Create output file
        let output_file = File::create(path)?;
        let mut output_file = BufWriter::new(output_file);
        
        // Decompress directly to file
        let mut extracted = false;
        let name_clone = name.clone();
        sz.for_each_entries(|reader_entry, reader| {
            if reader_entry.name == name_clone && !extracted {
                std::io::copy(reader, &mut output_file)
                    .map_err(|e| sevenz_rust::Error::Io(e, Cow::Borrowed("Failed to copy data")))?;
                extracted = true;
            }
            Ok(true) // continue
        }).map_err(|e| Error::ArchiveError(format!("Failed to extract entry: {}", e)))?;
        
        output_file.flush()?;

        // Set file permissions if available
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if attrs & 0x10 == 0 {  // Not a directory
                let mode = (attrs >> 16) & 0o777;
                if mode != 0 {
                    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        // Set modification time if available
        let mtime_secs = mtime_raw / 10_000_000 - 11_644_473_600;
        if mtime_secs > 0 {
            filetime::set_file_mtime(
                path,
                filetime::FileTime::from_unix_time(mtime_secs as i64, 0),
            )?;
        }
    }

    info!("7z extraction complete");
    Ok(())
}

/// Inspect 7z archive contents
pub fn inspect_7z<P: AsRef<Path>>(archive: P) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive.as_ref();

    debug!("Inspecting 7z archive: {:?}", archive_path);

    // Read the entire archive into memory
    let archive_data = fs::read(archive_path)?;
    let cursor = Cursor::new(archive_data);
    
    // Open the archive (no password)
    let sz = SevenZReader::new(cursor, archive_path.metadata()?.len(), Password::empty())
        .map_err(|e| Error::ArchiveError(format!("Failed to open 7z archive: {}", e)))?;

    let mut entries = Vec::new();

    // Get archive info
    let archive = sz.archive();
    
    // Process all files
    for entry in &archive.files {
        // Get uncompressed size - use size if available, else use compressed_size
        let uncompressed_size = if entry.size > 0 {
            entry.size
        } else {
            entry.compressed_size
        };
        
        // Convert FileTime to Unix timestamp
        let mtime_secs = entry.last_modified_date.to_raw() / 10_000_000 - 11_644_473_600;
        
        entries.push(ArchiveEntry {
            path: PathBuf::from(&entry.name),
            size: uncompressed_size,
            compressed_size: Some(entry.compressed_size),
            mode: Some((entry.windows_attributes >> 16) & 0o777),
            mtime: Some(mtime_secs as i64),
            is_dir: entry.is_directory(),
            is_symlink: false, // 7z doesn't support symlinks
            link_target: None,
        });
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