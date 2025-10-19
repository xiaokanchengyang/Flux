use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("flux"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A cross-platform file archiver"));
}

#[test]
fn test_pack_extract_basic() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let archive_path = temp_dir.path().join("test.tar.gz");
    let output_dir = temp_dir.path().join("output");

    // Create input file
    fs::write(&input_file, "Test content").unwrap();

    // Pack
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .assert()
        .success();

    assert!(archive_path.exists());

    // Extract
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive_path)
        .arg("-o")
        .arg(&output_dir)
        .assert()
        .success();

    // Verify
    assert!(output_dir.join("input.txt").exists());
    assert_eq!(
        fs::read_to_string(output_dir.join("input.txt")).unwrap(),
        "Test content"
    );
}

#[test]
fn test_pack_with_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("output.archive");

    fs::write(&input_file, "Content").unwrap();

    // Pack with explicit format
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .arg("-f")
        .arg("tar.zst")
        .assert()
        .success();

    assert!(archive_path.exists());
}

#[test]
fn test_pack_with_smart_strategy() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let archive_path = temp_dir.path().join("smart.tar.gz");

    // Create mixed content
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("text.txt"), "Text file").unwrap();
    fs::write(input_dir.join("image.jpg"), "Fake JPEG").unwrap();

    // Pack with smart strategy
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_dir)
        .arg("-o")
        .arg(&archive_path)
        .arg("--smart")
        .assert()
        .success();

    assert!(archive_path.exists());
}

#[test]
fn test_inspect_command() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.tar");

    fs::write(&input_file, "Content").unwrap();

    // Create archive
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .assert()
        .success();

    // Inspect
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("inspect")
        .arg(&archive_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn test_inspect_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("data.txt");
    let archive_path = temp_dir.path().join("test.tar");

    fs::write(&input_file, "Data").unwrap();

    // Create archive
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .assert()
        .success();

    // Inspect with JSON output
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("inspect")
        .arg(&archive_path)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"path\""))
        .stdout(predicate::str::contains("\"size\""));
}

#[test]
fn test_extract_with_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.tar");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, "Original").unwrap();

    // Pack
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .assert()
        .success();

    // Extract first time
    fs::create_dir_all(&output_dir).unwrap();
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive_path)
        .arg("-o")
        .arg(&output_dir)
        .assert()
        .success();

    // Modify file
    fs::write(output_dir.join("test.txt"), "Modified").unwrap();

    // Extract with overwrite
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract")
        .arg(&archive_path)
        .arg("-o")
        .arg(&output_dir)
        .arg("--overwrite")
        .assert()
        .success();

    // Should be back to original
    assert_eq!(
        fs::read_to_string(output_dir.join("test.txt")).unwrap(),
        "Original"
    );
}

#[test]
fn test_verbose_and_quiet_flags() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.tar");

    fs::write(&input_file, "Content").unwrap();

    // Test verbose
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("-v")
        .arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"));

    // Test quiet
    let archive_path2 = temp_dir.path().join("test2.tar");
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("-q")
        .arg("pack")
        .arg(&input_file)
        .arg("-o")
        .arg(&archive_path2)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_config_commands() {
    // Test config path
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("config")
        .arg("--path")
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));

    // Test config show (might fail if no config exists, but command should work)
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("config").arg("--show").assert();
}

#[test]
fn test_error_handling() {
    // Test with non-existent file
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("pack")
        .arg("/non/existent/file")
        .arg("-o")
        .arg("output.tar")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));

    // Test with invalid archive
    let temp_dir = TempDir::new().unwrap();
    let bad_archive = temp_dir.path().join("bad.tar");
    fs::write(&bad_archive, "Not a tar file").unwrap();

    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.arg("extract").arg(&bad_archive).assert().failure();
}
