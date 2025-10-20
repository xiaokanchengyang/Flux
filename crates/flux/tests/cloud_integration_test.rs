//! Cloud integration tests for flux-cli
//!
//! These tests verify that the CLI properly handles cloud URLs when built
//! with the cloud feature.

#![cfg(feature = "cloud")]

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_cloud_url_detection() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    
    // Test that cloud URLs are recognized (will fail due to missing credentials)
    cmd.args(&["extract", "s3://bucket/file.tar", "-o", "/tmp/out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AWS credentials not found"));
    
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.args(&["extract", "gs://bucket/file.tar", "-o", "/tmp/out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Google Cloud credentials not found"));
    
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.args(&["extract", "az://container/file.tar", "-o", "/tmp/out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Azure credentials not found"));
}

#[test]
fn test_pack_to_cloud_url() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();
    
    let mut cmd = Command::cargo_bin("flux").unwrap();
    
    // Test that packing to cloud URL is recognized
    cmd.args(&["pack", "-i", test_file.to_str().unwrap(), "-o", "s3://bucket/output.tar"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AWS credentials not found"));
}

#[test]
fn test_inspect_cloud_archive() {
    let mut cmd = Command::cargo_bin("flux").unwrap();
    
    // Test that inspect command recognizes cloud URLs
    cmd.args(&["inspect", "s3://bucket/archive.tar"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AWS credentials not found"));
}

#[test]
fn test_cloud_url_formats() {
    // Test various cloud URL formats are accepted
    let urls = [
        "s3://bucket/file.tar",
        "s3://bucket/path/to/file.tar.gz",
        "gs://bucket/archive.tar.zst",
        "az://container/backup.tar.xz",
        "azblob://container/data.tar",
    ];
    
    for url in &urls {
        let mut cmd = Command::cargo_bin("flux").unwrap();
        cmd.args(&["inspect", url])
            .assert()
            .failure()
            .stderr(predicate::str::contains("credentials not found"));
    }
}

#[test]
#[ignore = "Requires AWS credentials and test bucket"]
fn test_e2e_s3_pack_extract() {
    // This is a full end-to-end test that requires real AWS credentials
    let test_bucket = std::env::var("TEST_S3_BUCKET").expect("TEST_S3_BUCKET not set");
    let test_key = format!("flux-test-{}.tar.gz", uuid::Uuid::new_v4());
    let s3_url = format!("s3://{}/{}", test_bucket, test_key);
    
    // Create test data
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    fs::create_dir(&input_dir).unwrap();
    fs::write(input_dir.join("file1.txt"), "Hello from file 1").unwrap();
    fs::write(input_dir.join("file2.txt"), "Hello from file 2").unwrap();
    
    // Pack to S3
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.args(&["pack", "-i", input_dir.to_str().unwrap(), "-o", &s3_url])
        .assert()
        .success();
    
    // Extract from S3
    let output_dir = temp_dir.path().join("output");
    let mut cmd = Command::cargo_bin("flux").unwrap();
    cmd.args(&["extract", &s3_url, "-o", output_dir.to_str().unwrap()])
        .assert()
        .success();
    
    // Verify extracted files
    assert_eq!(
        fs::read_to_string(output_dir.join("file1.txt")).unwrap(),
        "Hello from file 1"
    );
    assert_eq!(
        fs::read_to_string(output_dir.join("file2.txt")).unwrap(),
        "Hello from file 2"
    );
}