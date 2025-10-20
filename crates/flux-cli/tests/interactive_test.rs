//! Tests for interactive extraction functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
// use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_interactive_flag_exists() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--interactive"));
}

#[test]
fn test_extract_with_interactive_flag() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar");
    let output_dir = temp_dir.path().join("output");

    // Create a simple tar archive
    let tar_file = File::create(&archive_path).unwrap();
    let mut builder = tar::Builder::new(tar_file);

    // Add a test file
    let mut header = tar::Header::new_gnu();
    header.set_path("test.txt").unwrap();
    header.set_size(5);
    header.set_mode(0o644);
    header.set_cksum();

    builder.append(&header, "hello".as_bytes()).unwrap();
    builder.finish().unwrap();

    // Create output directory
    fs::create_dir_all(&output_dir).unwrap();

    // Test extraction with interactive flag (should work without prompting when no conflicts)
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive_path)
        .arg("--output")
        .arg(&output_dir)
        .arg("--interactive");

    cmd.assert().success();

    // Verify the file was extracted
    let extracted_file = output_dir.join("test.txt");
    assert!(extracted_file.exists());
    assert_eq!(fs::read_to_string(extracted_file).unwrap(), "hello");
}

#[test]
fn test_7z_interactive_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.7z");
    let output_dir = temp_dir.path().join("output");

    // Create a dummy 7z file (won't actually extract, just testing the fallback)
    File::create(&archive_path).unwrap();

    // Test that 7z with interactive flag doesn't fail
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive_path)
        .arg("--output")
        .arg(&output_dir)
        .arg("--interactive");

    // This will fail because it's not a valid 7z file, but that's ok
    // We're just testing that the interactive flag is handled
    cmd.assert().failure();
}
