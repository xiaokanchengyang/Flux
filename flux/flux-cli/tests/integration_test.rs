use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A cross-platform file archiver and compressor",
        ));
}

#[test]
fn test_pack_extract_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_data");
    let archive = temp_dir.path().join("test.tar");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test data
    fs::create_dir_all(&test_dir).unwrap();
    fs::write(test_dir.join("file1.txt"), b"Content 1").unwrap();
    fs::write(test_dir.join("file2.txt"), b"Content 2").unwrap();

    // Pack
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&test_dir)
        .arg("-o")
        .arg(&archive)
        .assert()
        .success();

    assert!(archive.exists());

    // Extract
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive)
        .arg("-o")
        .arg(&extract_dir)
        .assert()
        .success();

    // Verify
    assert!(extract_dir.join("test_data").join("file1.txt").exists());
    assert!(extract_dir.join("test_data").join("file2.txt").exists());

    let content1 = fs::read(extract_dir.join("test_data").join("file1.txt")).unwrap();
    assert_eq!(content1, b"Content 1");

    let content2 = fs::read(extract_dir.join("test_data").join("file2.txt")).unwrap();
    assert_eq!(content2, b"Content 2");
}

#[test]
fn test_pack_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let archive = temp_dir.path().join("output.tar");

    // Create test file
    fs::write(&test_file, b"Hello, Flux!").unwrap();

    // Pack
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&test_file)
        .arg("-o")
        .arg(&archive)
        .assert()
        .success();

    assert!(archive.exists());
    assert!(archive.metadata().unwrap().len() > 0);
}

#[test]
fn test_pack_missing_input() {
    let temp_dir = TempDir::new().unwrap();
    let missing = temp_dir.path().join("nonexistent");
    let archive = temp_dir.path().join("output.tar");

    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&missing)
        .arg("-o")
        .arg(&archive)
        .assert()
        .failure();
}

#[test]
fn test_extract_missing_archive() {
    let temp_dir = TempDir::new().unwrap();
    let missing = temp_dir.path().join("nonexistent.tar");
    let output = temp_dir.path().join("output");

    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&missing)
        .arg("-o")
        .arg(&output)
        .assert()
        .failure();
}

#[test]
fn test_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let archive = temp_dir.path().join("output.tar");

    fs::write(&test_file, b"Test").unwrap();

    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("--verbose")
        .arg("pack")
        .arg(&test_file)
        .arg("-o")
        .arg(&archive)
        .env("RUST_LOG", "debug")
        .assert()
        .success()
        .stdout(predicate::str::contains("Packing"));
}

#[test]
fn test_quiet_flag() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let archive = temp_dir.path().join("output.tar");

    fs::write(&test_file, b"Test").unwrap();

    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("--quiet")
        .arg("pack")
        .arg(&test_file)
        .arg("-o")
        .arg(&archive)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
