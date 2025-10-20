use flux_core::archive::{
    extract_with_options, inspect, pack_with_strategy, ExtractOptions, PackOptions,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_pack_extract_tar_gz() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar.gz");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file1.txt"), "Content 1").unwrap();
    fs::write(source_dir.join("file2.txt"), "Content 2").unwrap();
    fs::create_dir_all(source_dir.join("subdir")).unwrap();
    fs::write(source_dir.join("subdir/file3.txt"), "Content 3").unwrap();

    // Pack with smart strategy
    let pack_options = PackOptions {
        smart: true,
        ..Default::default()
    };
    pack_with_strategy(&source_dir, &archive_path, Some("tar.gz"), pack_options).unwrap();
    assert!(archive_path.exists());

    // Extract
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

    // Verify - files are extracted preserving directory structure
    assert!(extract_dir.join("source/file1.txt").exists());
    assert!(extract_dir.join("source/file2.txt").exists());
    assert!(extract_dir.join("source/subdir/file3.txt").exists());
    assert_eq!(
        fs::read_to_string(extract_dir.join("source/file1.txt")).unwrap(),
        "Content 1"
    );
}

#[test]
fn test_pack_extract_zip() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test file
    fs::write(&source_file, "Test content for ZIP").unwrap();

    // Pack
    pack_with_strategy(
        &source_file,
        &archive_path,
        Some("zip"),
        PackOptions::default(),
    )
    .unwrap();
    assert!(archive_path.exists());

    // Extract
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

    // Verify
    let extracted_file = extract_dir.join("test.txt");
    assert!(extracted_file.exists());
    assert_eq!(
        fs::read_to_string(extracted_file).unwrap(),
        "Test content for ZIP"
    );
}

#[test]
fn test_extract_with_skip_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create and pack files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file.txt"), "Original").unwrap();
    pack_with_strategy(
        &source_dir,
        &archive_path,
        Some("tar"),
        PackOptions::default(),
    )
    .unwrap();

    // Extract once
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

    // Modify the extracted file
    fs::write(extract_dir.join("source/file.txt"), "Modified").unwrap();

    // Extract again with skip option (which is the default)
    let skip_options = ExtractOptions {
        skip: true,
        overwrite: false,
        rename: false,
        strip_components: None,
    };
    extract_with_options(&archive_path, &extract_dir, skip_options).unwrap();

    // File should still contain modified content
    assert_eq!(
        fs::read_to_string(extract_dir.join("source/file.txt")).unwrap(),
        "Modified"
    );
}

#[test]
fn test_extract_with_overwrite_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create and pack files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file.txt"), "Original").unwrap();
    pack_with_strategy(
        &source_dir,
        &archive_path,
        Some("tar"),
        PackOptions::default(),
    )
    .unwrap();

    // Extract once
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

    // Modify the extracted file
    fs::write(extract_dir.join("file.txt"), "Modified").unwrap();

    // Extract again with overwrite option
    let overwrite_options = ExtractOptions {
        skip: false,
        overwrite: true,
        rename: false,
        strip_components: None,
    };
    extract_with_options(&archive_path, &extract_dir, overwrite_options).unwrap();

    // File should contain original content (preserved directory structure)
    assert_eq!(
        fs::read_to_string(extract_dir.join("source/file.txt")).unwrap(),
        "Original"
    );
}

#[test]
fn test_extract_with_rename_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create and pack files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file.txt"), "Original").unwrap();
    pack_with_strategy(
        &source_dir,
        &archive_path,
        Some("tar"),
        PackOptions::default(),
    )
    .unwrap();

    // Extract once
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

    // Extract again with rename option
    let rename_options = ExtractOptions {
        skip: false,
        overwrite: false,
        rename: true,
        strip_components: None,
    };
    extract_with_options(&archive_path, &extract_dir, rename_options).unwrap();

    // Both files should exist (preserved directory structure)
    assert!(extract_dir.join("source/file.txt").exists());
    assert!(extract_dir.join("source/file (1).txt").exists());
}

#[test]
fn test_extract_with_strip_components() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create nested directory structure
    let nested = source_dir.join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("file.txt"), "Content").unwrap();

    // Pack
    pack_with_strategy(
        &source_dir,
        &archive_path,
        Some("tar"),
        PackOptions::default(),
    )
    .unwrap();

    // Extract with strip_components=3 (removes "source/a/b/")
    let strip_options = ExtractOptions {
        strip_components: Some(3),
        ..Default::default()
    };
    extract_with_options(&archive_path, &extract_dir, strip_options).unwrap();

    // File should be at c/file.txt instead of source/a/b/c/file.txt
    assert!(extract_dir.join("c/file.txt").exists());
    assert!(!extract_dir.join("source/a/b/c/file.txt").exists());
}

#[test]
fn test_inspect_archive() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar.gz");

    // Create test files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("small.txt"), "Small").unwrap();
    fs::write(
        source_dir.join("large.txt"),
        "Large file content that is actually not that large",
    )
    .unwrap();

    // Pack
    pack_with_strategy(
        &source_dir,
        &archive_path,
        Some("tar.gz"),
        PackOptions::default(),
    )
    .unwrap();

    // Inspect
    let entries = inspect(&archive_path).unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries
        .iter()
        .any(|e| e.path.to_str().unwrap().contains("small.txt")));
    assert!(entries
        .iter()
        .any(|e| e.path.to_str().unwrap().contains("large.txt")));

    // Check that entries have size information
    for entry in entries {
        assert!(entry.size > 0);
        assert!(!entry.is_dir);
    }
}

#[test]
fn test_pack_with_custom_algorithm() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.tar.xz");

    // Create test file
    fs::write(&source_file, "Test content for XZ compression").unwrap();

    // Pack with specific algorithm
    let pack_options = PackOptions {
        smart: false,
        algorithm: Some("xz".to_string()),
        level: Some(6),
        ..Default::default()
    };

    pack_with_strategy(&source_file, &archive_path, Some("tar.xz"), pack_options).unwrap();
    assert!(archive_path.exists());

    // Verify by extracting
    let extract_dir = temp_dir.path().join("extracted");
    extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();
    assert!(extract_dir.join("test.txt").exists());
}
