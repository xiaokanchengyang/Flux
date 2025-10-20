//! Integration tests for path traversal security

use flux_lib::archive::{create_secure_extractor, extractor::ExtractEntryOptions};
use flux_lib::security::{sanitize_path, validate_symlink};
use flux_lib::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Test the sanitize_path function directly
#[test]
fn test_sanitize_path_protection() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    
    // Test normal paths (should succeed)
    assert!(sanitize_path(base, Path::new("normal.txt")).is_ok());
    assert!(sanitize_path(base, Path::new("subdir/file.txt")).is_ok());
    assert!(sanitize_path(base, Path::new("./file.txt")).is_ok());
    
    // Test malicious paths (should fail)
    assert!(sanitize_path(base, Path::new("../evil.txt")).is_err());
    assert!(sanitize_path(base, Path::new("../../etc/passwd")).is_err());
    assert!(sanitize_path(base, Path::new("/etc/passwd")).is_err());
    assert!(sanitize_path(base, Path::new("subdir/../../../evil.txt")).is_err());
    
    // Windows-style paths
    #[cfg(windows)]
    {
        assert!(sanitize_path(base, Path::new("C:\\Windows\\System32\\cmd.exe")).is_err());
        assert!(sanitize_path(base, Path::new("..\\..\\evil.txt")).is_err());
    }
}

/// Test symlink validation
#[test]
fn test_symlink_validation() {
    let base = Path::new("/tmp/extract");
    
    // Safe symlinks (relative within base)
    assert!(validate_symlink(
        base,
        &base.join("link"),
        Path::new("target.txt"),
        false
    ).is_ok());
    
    assert!(validate_symlink(
        base,
        &base.join("subdir/link"),
        Path::new("../file.txt"),
        false
    ).is_ok());
    
    // Dangerous symlinks (should fail when allow_external is false)
    assert!(validate_symlink(
        base,
        &base.join("link"),
        Path::new("/etc/passwd"),
        false
    ).is_err());
    
    assert!(validate_symlink(
        base,
        &base.join("subdir/link"),
        Path::new("../../outside.txt"),
        false
    ).is_err());
    
    // But should succeed when allow_external is true
    assert!(validate_symlink(
        base,
        &base.join("link"),
        Path::new("/etc/passwd"),
        true
    ).is_ok());
}

/// Create a malicious zip archive with path traversal attempts
fn create_malicious_zip(path: &Path) -> zip::result::ZipResult<()> {
    use zip::{ZipWriter, write::FileOptions};
    
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
    
    // Normal file (safe)
    zip.start_file("normal.txt", options)?;
    zip.write_all(b"Normal file")?;
    
    // Attempt 1: Parent directory traversal
    zip.start_file("../evil1.txt", options)?;
    zip.write_all(b"Evil file type 1")?;
    
    // Attempt 2: Deep parent directory traversal
    zip.start_file("../../evil2.txt", options)?;
    zip.write_all(b"Evil file type 2")?;
    
    // Attempt 3: Windows-style path
    zip.start_file("..\\..\\evil3.txt", options)?;
    zip.write_all(b"Evil file type 3")?;
    
    // Attempt 4: Absolute path
    zip.start_file("/etc/passwd", options)?;
    zip.write_all(b"Evil file type 4")?;
    
    // Attempt 5: Hidden traversal
    zip.start_file("subdir/../../../evil5.txt", options)?;
    zip.write_all(b"Evil file type 5")?;
    
    zip.finish()?;
    Ok(())
}

#[test]
fn test_zip_path_traversal_protection() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("malicious.zip");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir(&extract_dir).unwrap();
    
    // Create malicious archive
    create_malicious_zip(&archive_path).unwrap();
    
    // Try to extract with secure extractor
    let extractor = create_secure_extractor(&archive_path).unwrap();
    let options = ExtractEntryOptions {
        overwrite: true,
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
    };
    
    let mut extracted_count = 0;
    let mut blocked_count = 0;
    
    // Extract entries
    for entry_result in extractor.entries(&archive_path).unwrap() {
        match entry_result {
            Ok(entry) => {
                match extractor.extract_entry(&archive_path, &entry, &extract_dir, options.clone()) {
                    Ok(_) => {
                        extracted_count += 1;
                        println!("Extracted: {:?}", entry.path);
                    }
                    Err(e) => {
                        blocked_count += 1;
                        println!("Blocked: {:?} - {}", entry.path, e);
                        
                        // Verify it's a security error
                        match e {
                            Error::InvalidPath(_) | Error::SecurityError(_) => {
                                // Expected error types for path traversal
                            }
                            _ => {
                                // Windows-style paths might be allowed on Unix but won't escape
                                if !entry.path.to_string_lossy().contains("\\") {
                                    panic!("Unexpected error type: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                blocked_count += 1;
                println!("Failed to read entry: {}", e);
            }
        }
    }
    
    println!("Extracted: {}, Blocked: {}", extracted_count, blocked_count);
    
    // Verify results
    // On Windows, backslash paths might be treated as normal files
    // On Unix, they would be extracted but won't escape the directory
    assert!(extracted_count <= 2, "At most normal.txt and Windows-style path should be extracted");
    assert!(blocked_count >= 4, "At least 4 malicious entries should be blocked");
    
    // Verify only safe files were extracted
    assert!(extract_dir.join("normal.txt").exists());
    
    // Verify malicious files were NOT extracted outside the extraction directory
    assert!(!temp_dir.path().join("evil1.txt").exists());
    assert!(!temp_dir.path().join("evil2.txt").exists());
    assert!(!temp_dir.path().join("evil3.txt").exists());
    assert!(!temp_dir.path().join("evil5.txt").exists());
    assert!(!Path::new("/etc/passwd").exists() || 
            fs::read_to_string("/etc/passwd").unwrap_or_default() != "Evil file type 4");
}

/// Test that the secure extractor properly handles tar files
#[test] 
fn test_tar_secure_extraction() {
    use tar::{Builder, Header};
    
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir(&extract_dir).unwrap();
    
    // Create a simple tar archive
    {
        let file = File::create(&archive_path).unwrap();
        let mut builder = Builder::new(file);
        
        // Add normal files
        let mut header = Header::new_gnu();
        header.set_path("normal.txt").unwrap();
        header.set_size(11);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, "Normal file".as_bytes()).unwrap();
        
        // Add a file in subdirectory
        let mut header = Header::new_gnu();
        header.set_path("subdir/file.txt").unwrap();
        header.set_size(15);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, "Subdir file".as_bytes()).unwrap();
        
        builder.finish().unwrap();
    }
    
    // Extract with secure extractor
    let extractor = create_secure_extractor(&archive_path).unwrap();
    let options = ExtractEntryOptions {
        overwrite: true,
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
    };
    
    let mut extracted_count = 0;
    for entry_result in extractor.entries(&archive_path).unwrap() {
        if let Ok(entry) = entry_result {
            if extractor.extract_entry(&archive_path, &entry, &extract_dir, options.clone()).is_ok() {
                extracted_count += 1;
            }
        }
    }
    
    assert_eq!(extracted_count, 2, "Both files should be extracted");
    assert!(extract_dir.join("normal.txt").exists());
    assert!(extract_dir.join("subdir/file.txt").exists());
}

/// Test compression bomb detection
#[test]
fn test_compression_bomb_detection() {
    use flux_lib::security::{check_compression_ratio, DEFAULT_MAX_COMPRESSION_RATIO};
    
    // Normal compression ratio (should pass)
    assert!(check_compression_ratio(1000, 5000, DEFAULT_MAX_COMPRESSION_RATIO).is_ok());
    
    // Suspicious compression ratio (should fail)
    assert!(check_compression_ratio(100, 1_000_000, DEFAULT_MAX_COMPRESSION_RATIO).is_err());
    
    // Edge case: exactly at limit (should pass)
    assert!(check_compression_ratio(100, 10_000, DEFAULT_MAX_COMPRESSION_RATIO).is_ok());
    
    // Edge case: just over limit (should fail)  
    assert!(check_compression_ratio(100, 10_001, DEFAULT_MAX_COMPRESSION_RATIO).is_err());
}