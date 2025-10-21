//! Tests for extract options functionality

use flux_core::{extract_with_options, pack, ExtractOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_overwrite_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file1.txt"), "Original content").unwrap();
    fs::write(source_dir.join("file2.txt"), "File 2 content").unwrap();

    // Pack the files
    pack(&source_dir, &archive_path, None).unwrap();

    // Extract first time
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        ..Default::default()
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Modify extracted file
    let file1_path = extract_dir.join("source/file1.txt");
    fs::write(&file1_path, "Modified content").unwrap();

    // Extract again with overwrite
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify file was overwritten
    let content = fs::read_to_string(&file1_path).unwrap();
    assert_eq!(
        content, "Original content",
        "File should have been overwritten"
    );
}

#[test]
fn test_skip_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file1.txt"), "Original content").unwrap();
    fs::write(source_dir.join("file2.txt"), "File 2 content").unwrap();

    // Pack the files
    pack(&source_dir, &archive_path, None).unwrap();

    // Extract first time
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        ..Default::default()
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Modify extracted file
    let file1_path = extract_dir.join("source/file1.txt");
    fs::write(&file1_path, "Modified content").unwrap();

    // Extract again with skip
    let extract_opts = ExtractOptions {
        overwrite: false,
        skip: true,
        rename: false,
        strip_components: None,
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify file was skipped (kept modified content)
    let content = fs::read_to_string(&file1_path).unwrap();
    assert_eq!(content, "Modified content", "File should have been skipped");
}

#[test]
fn test_rename_option() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("file1.txt"), "Original content").unwrap();

    // Pack the files
    pack(&source_dir, &archive_path, None).unwrap();

    // Extract first time
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Extract again with rename
    let extract_opts = ExtractOptions {
        overwrite: false,
        skip: false,
        rename: true,
        strip_components: None,
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify renamed file exists
    let renamed_file = extract_dir.join("source/file1 (1).txt");
    assert!(renamed_file.exists(), "Renamed file should exist");

    // Verify original file still exists
    let original_file = extract_dir.join("source/file1.txt");
    assert!(original_file.exists(), "Original file should still exist");
}

#[test]
fn test_strip_components() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create nested directory structure
    let nested_dir = source_dir.join("level1/level2/level3");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("deep.txt"), "Deep file content").unwrap();
    fs::write(source_dir.join("root.txt"), "Root content").unwrap();
    fs::write(source_dir.join("level1/mid.txt"), "Mid content").unwrap();

    // Pack the directory
    pack(&source_dir, &archive_path, None).unwrap();

    // Extract with strip_components=1
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: Some(1),
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify files are extracted without the first component
    assert!(
        !extract_dir.join("source").exists(),
        "source directory should not exist"
    );
    assert!(
        extract_dir.join("root.txt").exists(),
        "root.txt should exist at top level"
    );
    assert!(
        extract_dir.join("level1/mid.txt").exists(),
        "mid.txt should exist under level1"
    );
    assert!(
        extract_dir.join("level1/level2/level3/deep.txt").exists(),
        "deep.txt should exist"
    );
}

#[test]
fn test_strip_components_deep() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create nested directory structure
    let nested_dir = source_dir.join("level1/level2/level3");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("deep.txt"), "Deep file content").unwrap();
    fs::write(source_dir.join("level1/level2/mid.txt"), "Mid content").unwrap();

    // Pack the directory
    pack(&source_dir, &archive_path, None).unwrap();

    // Extract with strip_components=3 (stripping source/level1/level2)
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: Some(3),
        hoist: false,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify only files with enough components are extracted
    assert!(
        extract_dir.join("level3/deep.txt").exists(),
        "deep.txt should exist under level3"
    );
    assert!(
        extract_dir.join("mid.txt").exists(),
        "mid.txt should exist at top level"
    );

    // Files with fewer components should be skipped
    assert!(!extract_dir.join("source").exists());
    assert!(!extract_dir.join("level1").exists());
}
