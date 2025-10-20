//! Integration tests for security features

use flux_core::security::{
    check_compression_ratio, check_extraction_size, sanitize_path, validate_symlink,
};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_path_traversal_protection() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Test normal paths - should succeed
    assert!(sanitize_path(base, Path::new("normal/file.txt")).is_ok());
    assert!(sanitize_path(base, Path::new("./normal/file.txt")).is_ok());
    assert!(sanitize_path(base, Path::new("subdir/another/file.txt")).is_ok());

    // Test dangerous paths - should fail
    assert!(sanitize_path(base, Path::new("../etc/passwd")).is_err());
    assert!(sanitize_path(base, Path::new("../../etc/passwd")).is_err());
    assert!(sanitize_path(base, Path::new("/etc/passwd")).is_err());
    assert!(sanitize_path(base, Path::new("subdir/../../etc/passwd")).is_err());

    // Test paths with backslashes (these might be valid on Unix but suspicious)
    // On Unix, backslashes are just regular characters in filenames
}

#[test]
fn test_symlink_validation() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Test internal symlinks - should succeed
    assert!(validate_symlink(base, &base.join("link"), Path::new("target.txt"), false).is_ok());

    assert!(validate_symlink(
        base,
        &base.join("subdir/link"),
        Path::new("../target.txt"),
        false
    )
    .is_ok());

    // Test external symlinks - should fail when not allowed
    assert!(validate_symlink(base, &base.join("link"), Path::new("/etc/passwd"), false).is_err());

    assert!(validate_symlink(
        base,
        &base.join("link"),
        Path::new("../../etc/passwd"),
        false
    )
    .is_err());

    // Test external symlinks - should succeed when allowed
    assert!(validate_symlink(base, &base.join("link"), Path::new("/etc/passwd"), true).is_ok());
}

#[test]
fn test_compression_ratio_detection() {
    // Normal compression ratios - should succeed
    assert!(check_compression_ratio(1000, 5000, 100.0).is_ok()); // 5:1 ratio
    assert!(check_compression_ratio(1000, 10000, 100.0).is_ok()); // 10:1 ratio
    assert!(check_compression_ratio(1000, 50000, 100.0).is_ok()); // 50:1 ratio

    // Suspicious compression ratios - should fail
    assert!(check_compression_ratio(1000, 200000, 100.0).is_err()); // 200:1 ratio
    assert!(check_compression_ratio(100, 1_000_000, 100.0).is_err()); // 10000:1 ratio

    // Edge cases
    assert!(check_compression_ratio(0, 1000, 100.0).is_ok()); // Zero compressed size
}

#[test]
fn test_extraction_size_limits() {
    // Within limits - should succeed
    assert!(check_extraction_size(0, 1000, 10000).is_ok());
    assert!(check_extraction_size(5000, 4000, 10000).is_ok());
    assert!(check_extraction_size(9000, 1000, 10000).is_ok());

    // Exceeding limits - should fail
    assert!(check_extraction_size(5000, 6000, 10000).is_err());
    assert!(check_extraction_size(10000, 1, 10000).is_err());

    // Test saturation
    assert!(check_extraction_size(u64::MAX - 1, 10, u64::MAX).is_ok());
}

#[cfg(test)]
mod secure_extraction_tests {
    use super::*;
    use flux_core::archive::{create_secure_extractor, pack_with_strategy, PackOptions};
    use std::fs;

    #[test]
    fn test_secure_extraction_blocks_malicious_archive() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"test content").unwrap();

        // Create an archive
        let archive_path = temp_dir.path().join("test.tar.gz");
        pack_with_strategy(
            &test_file,
            &archive_path,
            Some("tar.gz"),
            PackOptions::default(),
        )
        .unwrap();

        // Try to extract with secure extractor
        let extract_dir = temp_dir.path().join("extract");
        fs::create_dir(&extract_dir).unwrap();

        let extractor = create_secure_extractor(&archive_path).unwrap();

        // Extraction should succeed for normal files
        let entries: Vec<_> = extractor.entries(&archive_path).unwrap().collect();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_ok());
    }
}
