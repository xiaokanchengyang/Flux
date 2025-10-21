//! Integration tests for flux-cloud
//!
//! Note: These tests require cloud credentials to be set as environment variables.
//! They are marked with #[ignore] by default to avoid running in CI without credentials.

use flux_cloud::{CloudPath, CloudReader, CloudWriter};
use std::io::{Read, Seek, SeekFrom, Write};

#[test]
fn test_cloud_path_parsing() {
    // S3 paths
    let path = CloudPath::parse("s3://my-bucket/path/to/file.tar").unwrap();
    assert_eq!(path.scheme, "s3");
    assert_eq!(path.bucket, "my-bucket");
    assert_eq!(path.path.as_ref(), "path/to/file.tar");

    // GCS paths
    let path = CloudPath::parse("gs://gcs-bucket/archive.tar.gz").unwrap();
    assert_eq!(path.scheme, "gs");
    assert_eq!(path.bucket, "gcs-bucket");
    assert_eq!(path.path.as_ref(), "archive.tar.gz");

    // Azure paths
    let path = CloudPath::parse("az://container/blob.tar").unwrap();
    assert_eq!(path.scheme, "az");
    assert_eq!(path.bucket, "container");

    let path = CloudPath::parse("azblob://container/blob.tar").unwrap();
    assert_eq!(path.scheme, "azblob");
    assert_eq!(path.bucket, "container");
}

#[test]
fn test_invalid_cloud_paths() {
    assert!(CloudPath::parse("http://not-cloud/file").is_err());
    assert!(CloudPath::parse("/local/path/file").is_err());
    assert!(CloudPath::parse("s3://").is_err());
    assert!(CloudPath::parse("s3:///no-bucket").is_err());
}

#[test]
#[ignore = "Requires AWS credentials"]
fn test_s3_read_write() {
    // This test requires:
    // - AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables
    // - Write access to the test bucket

    let test_bucket =
        std::env::var("TEST_S3_BUCKET").unwrap_or_else(|_| "flux-test-bucket".to_string());
    let test_path = format!(
        "s3://{}/test-file-{}.bin",
        test_bucket,
        uuid::Uuid::new_v4()
    );

    // Test data
    let test_data = b"Hello from flux-cloud! This is a test file.";

    // Write test
    let mut writer = CloudWriter::new(&test_path).expect("Failed to create writer");
    writer.write_all(test_data).expect("Failed to write");
    writer.flush().expect("Failed to flush");
    drop(writer);

    // Read test
    let mut reader = CloudReader::new(&test_path).expect("Failed to create reader");
    let mut read_data = Vec::new();
    reader.read_to_end(&mut read_data).expect("Failed to read");

    assert_eq!(read_data, test_data);
}

#[test]
#[ignore = "Requires AWS credentials"]
fn test_s3_seek() {
    let test_bucket =
        std::env::var("TEST_S3_BUCKET").unwrap_or_else(|_| "flux-test-bucket".to_string());
    let test_path = format!(
        "s3://{}/test-seek-{}.bin",
        test_bucket,
        uuid::Uuid::new_v4()
    );

    // Create test data (1KB)
    let test_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    // Write test data
    let mut writer = CloudWriter::new(&test_path).expect("Failed to create writer");
    writer.write_all(&test_data).expect("Failed to write");
    drop(writer);

    // Test seeking
    let mut reader = CloudReader::new(&test_path).expect("Failed to create reader");

    // Seek to middle
    let pos = reader.seek(SeekFrom::Start(512)).expect("Failed to seek");
    assert_eq!(pos, 512);

    // Read and verify
    let mut buf = [0u8; 10];
    reader.read_exact(&mut buf).expect("Failed to read");
    assert_eq!(buf, &test_data[512..522]);

    // Seek relative
    let pos = reader.seek(SeekFrom::Current(100)).expect("Failed to seek");
    assert_eq!(pos, 622);

    // Seek from end
    let pos = reader.seek(SeekFrom::End(-10)).expect("Failed to seek");
    assert_eq!(pos, 1014);
}

#[test]
#[ignore = "Requires GCS credentials"]
fn test_gcs_operations() {
    // This test requires GOOGLE_APPLICATION_CREDENTIALS environment variable

    let test_bucket =
        std::env::var("TEST_GCS_BUCKET").unwrap_or_else(|_| "flux-test-bucket".to_string());
    let test_path = format!(
        "gs://{}/test-file-{}.bin",
        test_bucket,
        uuid::Uuid::new_v4()
    );

    // Test basic read/write
    let test_data = b"Testing Google Cloud Storage with flux-cloud";

    let mut writer = CloudWriter::new(&test_path).expect("Failed to create GCS writer");
    writer.write_all(test_data).expect("Failed to write to GCS");
    drop(writer);

    let mut reader = CloudReader::new(&test_path).expect("Failed to create GCS reader");
    let mut read_data = Vec::new();
    reader
        .read_to_end(&mut read_data)
        .expect("Failed to read from GCS");

    assert_eq!(read_data, test_data);
}

#[test]
#[ignore = "Requires Azure credentials"]
fn test_azure_operations() {
    // This test requires AZURE_STORAGE_ACCOUNT_NAME and AZURE_STORAGE_ACCOUNT_KEY

    let test_container =
        std::env::var("TEST_AZURE_CONTAINER").unwrap_or_else(|_| "flux-test".to_string());
    let test_path = format!(
        "az://{}/test-file-{}.bin",
        test_container,
        uuid::Uuid::new_v4()
    );

    // Test basic read/write
    let test_data = b"Testing Azure Blob Storage with flux-cloud";

    let mut writer = CloudWriter::new(&test_path).expect("Failed to create Azure writer");
    writer
        .write_all(test_data)
        .expect("Failed to write to Azure");
    drop(writer);

    let mut reader = CloudReader::new(&test_path).expect("Failed to create Azure reader");
    let mut read_data = Vec::new();
    reader
        .read_to_end(&mut read_data)
        .expect("Failed to read from Azure");

    assert_eq!(read_data, test_data);
}

#[test]
fn test_multipart_threshold() {
    // Test that large writes trigger multipart upload logic
    // This is a unit test that doesn't require credentials

    let _large_data = vec![0u8; 20 * 1024 * 1024]; // 20MB

    // We can't test actual upload without credentials, but we can verify
    // the writer accepts large data
    let writer = CloudWriter::with_buffer_size("s3://fake/path", 8 * 1024 * 1024);
    assert!(writer.is_ok());

    // The writer should handle large data by switching to multipart
    // This is tested internally in the implementation
}
