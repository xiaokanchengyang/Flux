//! Tar archive operations

use crate::metadata::FileMetadata;
use crate::{Error, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tar::{Archive, Builder};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Pack files into a tar archive
pub fn pack_tar<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output: Q) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    info!("Packing {:?} into {:?}", input, output);

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output)?;
    let mut builder = Builder::new(file);

    if input.is_file() {
        // Pack single file
        pack_file(&mut builder, input, Path::new(input.file_name().unwrap()))?;
    } else if input.is_dir() {
        // Pack directory recursively
        pack_directory(&mut builder, input)?;
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
fn pack_file<W: Write>(builder: &mut Builder<W>, path: &Path, archive_path: &Path) -> Result<()> {
    debug!("Adding file: {:?} as {:?}", path, archive_path);

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

/// Pack a directory recursively into the tar builder
fn pack_directory<W: Write>(builder: &mut Builder<W>, dir: &Path) -> Result<()> {
    let base_path = dir.parent().unwrap_or(Path::new(""));

    for entry in WalkDir::new(dir) {
        let entry = entry.map_err(|e| Error::Io(e.into()))?;
        let path = entry.path();

        // Skip the directory itself
        if path == dir {
            continue;
        }

        // Calculate relative path for the archive
        let relative_path = path
            .strip_prefix(base_path)
            .map_err(|_| Error::InvalidPath(format!("Failed to strip prefix from {:?}", path)))?;

        if path.is_file() {
            pack_file(builder, path, relative_path)?;
        } else if path.is_dir() {
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
