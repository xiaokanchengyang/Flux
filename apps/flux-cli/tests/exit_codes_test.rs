//! Integration tests for exit codes

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_success_exit_code() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let archive = temp_dir.path().join("test.tar");

    fs::write(&test_file, "test content").unwrap();

    // Pack command should succeed with exit code 0
    Command::cargo_bin("flux")
        .unwrap()
        .arg("pack")
        .arg(&test_file)
        .arg("-o")
        .arg(&archive)
        .assert()
        .success()
        .code(0);
}

#[test]
fn test_io_error_exit_code() {
    // Try to pack a non-existent file (this is actually an InvalidPath error)
    Command::cargo_bin("flux")
        .unwrap()
        .arg("pack")
        .arg("/non/existent/file")
        .arg("-o")
        .arg("test.tar")
        .assert()
        .failure()
        .code(3); // InvalidPath maps to exit code 3
}

#[test]
fn test_invalid_arguments_exit_code() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Invalid compression algorithm
    Command::cargo_bin("flux")
        .unwrap()
        .arg("pack")
        .arg(&test_file)
        .arg("-o")
        .arg("test.tar")
        .arg("--algo")
        .arg("invalid_algo")
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_extract_non_existent_archive() {
    // Try to extract a non-existent archive
    Command::cargo_bin("flux")
        .unwrap()
        .arg("extract")
        .arg("/non/existent/archive.tar")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_pack_to_readonly_directory() {
    // Try to write to a directory without permissions
    #[cfg(unix)]
    {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();

        // Make directory read-only
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let archive = readonly_dir.join("test.tar");

        // Should fail with IO error
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&test_file)
            .arg("-o")
            .arg(&archive)
            .assert()
            .failure()
            .code(2);
    }
}

#[test]
fn test_inspect_invalid_archive() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_archive = temp_dir.path().join("invalid.tar");

    // Create an invalid archive file
    fs::write(&invalid_archive, "not a valid tar file").unwrap();

    // Inspect should fail with IO error code (tar parsing IO error)
    Command::cargo_bin("flux")
        .unwrap()
        .arg("inspect")
        .arg(&invalid_archive)
        .assert()
        .failure()
        .code(2); // IO error when reading invalid tar
}
