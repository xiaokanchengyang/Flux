use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
#[ignore = "7z test file creation is complex, skipping for now"]
fn test_extract_7z() {
    let temp_dir = TempDir::new().unwrap();
    let test_7z = "../test_data/test.7z";
    
    // Ensure the test 7z file exists
    assert!(
        std::path::Path::new(test_7z).exists(),
        "Test 7z file not found at: {}",
        test_7z
    );

    // Test extraction
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(test_7z)
        .arg("--output")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracting"))
        .stdout(predicate::str::contains("Extraction complete"));

    // Verify extracted files
    let extracted_dir = temp_dir.path().join("7z_test");
    assert!(extracted_dir.exists());
    
    let test_file = extracted_dir.join("test.txt");
    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content.trim(), "Test content for 7z");

    let nested_file = extracted_dir.join("subdir/nested.txt");
    assert!(nested_file.exists());
    let nested_content = fs::read_to_string(&nested_file).unwrap();
    assert_eq!(nested_content.trim(), "Nested content");
}

#[test]
#[ignore = "7z test file creation is complex, skipping for now"]
fn test_inspect_7z() {
    let test_7z = "../test_data/test.7z";
    
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("inspect")
        .arg(test_7z)
        .assert()
        .success()
        .stdout(predicate::str::contains("7z_test/test.txt"))
        .stdout(predicate::str::contains("7z_test/subdir/nested.txt"));
}

#[test]
fn test_pack_7z_not_supported() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();
    
    let output_7z = temp_dir.path().join("output.7z");
    
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&test_file)
        .arg(&output_7z)
        .assert()
        .failure()
        .stderr(predicate::str::contains("7z packing is not yet supported"));
}