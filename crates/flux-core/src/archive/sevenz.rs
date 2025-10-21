//! 7z archive support module

use crate::archive::ArchiveEntry;
use crate::{Error, Result};
use sevenz_rust::{Password, SevenZReader};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Pack files into a 7z archive
pub fn pack_7z<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    let _input = input.as_ref();
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
    let mut sz = SevenZReader::open(archive_path, Password::empty())
        .map_err(|e| Error::Archive(format!("Failed to open 7z archive: {}", e)))?;

    // Extract all entries
    sz.for_each_entries(|entry, reader| {
        let path = output_dir.join(&entry.name);

        if entry.is_directory {
            fs::create_dir_all(&path)?;
        } else {
            // Create parent directories
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            debug!("Extracting: {:?}", entry.name);

            // Extract the file
            let mut output_file = File::create(&path)?;
            io::copy(reader, &mut output_file)?;
        }

        Ok(true) // Continue extraction
    })
    .map_err(|e| Error::Archive(format!("Failed to extract 7z archive: {}", e)))?;

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
    let mut sz = SevenZReader::open(archive_path, Password::empty())
        .map_err(|e| Error::Archive(format!("Failed to open 7z archive: {}", e)))?;

    // Extract all entries
    sz.for_each_entries(|entry, reader| {
        let entry_path = PathBuf::from(&entry.name);

        // Handle strip components
        let final_path = if let Some(strip) = options.strip_components {
            let components: Vec<_> = entry_path.components().collect();
            if components.len() <= strip {
                return Ok(true); // Skip this entry
            }
            output_dir.join(components[strip..].iter().collect::<PathBuf>())
        } else {
            output_dir.join(&entry_path)
        };

        // Handle existing files
        if final_path.exists() {
            if options.skip {
                debug!("Skipping existing file: {:?}", final_path);
                return Ok(true);
            } else if options.rename {
                let mut counter = 1;
                let mut new_path = final_path.clone();
                while new_path.exists() {
                    let file_stem = final_path.file_stem().unwrap_or_default();
                    let extension = final_path.extension();
                    let new_name = if let Some(ext) = extension {
                        format!(
                            "{}_{}.{}",
                            file_stem.to_string_lossy(),
                            counter,
                            ext.to_string_lossy()
                        )
                    } else {
                        format!("{}_{}", file_stem.to_string_lossy(), counter)
                    };
                    new_path = final_path.with_file_name(new_name);
                    counter += 1;
                }
                debug!("Renaming to: {:?}", new_path);
            } else if !options.overwrite {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("File exists: {:?}", final_path),
                )
                .into());
            }
        }

        if entry.is_directory {
            fs::create_dir_all(&final_path)?;
        } else {
            // Create parent directories
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)?;
            }

            debug!("Extracting: {:?}", entry.name);

            // Extract the file
            let mut output_file = File::create(&final_path)?;
            io::copy(reader, &mut output_file)?;
        }

        Ok(true) // Continue extraction
    })
    .map_err(|e| Error::Archive(format!("Failed to extract 7z archive: {}", e)))?;

    info!("7z extraction complete");
    Ok(())
}

/// Inspect 7z archive contents
pub fn inspect_7z<P: AsRef<Path>>(archive: P) -> Result<Vec<ArchiveEntry>> {
    let archive_path = archive.as_ref();

    debug!("Inspecting 7z archive: {:?}", archive_path);

    // Note: The newer sevenz-rust API doesn't provide a good way to list entries without extracting
    // For now, we'll return a more limited implementation
    Err(Error::UnsupportedOperation(
        "7z inspection with full metadata is not currently supported".to_string(),
    ))
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
        assert!(matches!(
            result.unwrap_err(),
            Error::UnsupportedOperation(_)
        ));
    }
}
