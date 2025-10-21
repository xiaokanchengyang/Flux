//! Cloud storage handler for flux-cli
//!
//! This module provides cloud storage integration, allowing flux to work with
//! S3, Google Cloud Storage, and Azure Blob Storage.

use anyhow::{Context, Result};
use flux_cloud::{CloudPath, CloudReader, CloudWriter};
use std::io::{Read, Seek, Write};

/// Check if a path is a cloud URL
pub fn is_cloud_path(path: &str) -> bool {
    path.starts_with("s3://")
        || path.starts_with("gs://")
        || path.starts_with("az://")
        || path.starts_with("azblob://")
}

/// Trait that combines Read + Seek for cloud storage
pub trait CloudReadSeek: Read + Seek + Send {}

/// Implement CloudReadSeek for CloudReader
impl CloudReadSeek for CloudReader {}

/// Create a reader for cloud storage
pub fn create_cloud_reader(url: &str) -> Result<Box<dyn CloudReadSeek>> {
    let reader = CloudReader::new(url)
        .with_context(|| format!("Failed to create cloud reader for {}", url))?;
    Ok(Box::new(reader))
}

/// Create a writer for cloud storage
pub fn create_cloud_writer(url: &str) -> Result<Box<dyn Write + Send>> {
    let writer = CloudWriter::new(url)
        .with_context(|| format!("Failed to create cloud writer for {}", url))?;
    Ok(Box::new(writer))
}

/// Parse and validate a cloud path
pub fn parse_cloud_path(url: &str) -> Result<CloudPath> {
    CloudPath::parse(url).with_context(|| format!("Failed to parse cloud URL: {}", url))
}

/// Get a human-readable description of the cloud location
pub fn describe_cloud_location(url: &str) -> String {
    match CloudPath::parse(url) {
        Ok(path) => {
            let provider = match path.scheme.as_str() {
                "s3" => "Amazon S3",
                "gs" => "Google Cloud Storage",
                "az" | "azblob" => "Azure Blob Storage",
                _ => "Unknown Cloud",
            };
            format!("{} bucket '{}' at '{}'", provider, path.bucket, path.path)
        }
        Err(_) => url.to_string(),
    }
}

/// Check if cloud credentials are available for the given URL
pub fn check_cloud_credentials(url: &str) -> Result<()> {
    let cloud_path = parse_cloud_path(url)?;

    // Check for required environment variables based on provider
    match cloud_path.scheme.as_str() {
        "s3" => {
            if std::env::var("AWS_ACCESS_KEY_ID").is_err()
                || std::env::var("AWS_SECRET_ACCESS_KEY").is_err()
            {
                anyhow::bail!(
                    "AWS credentials not found. Please set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables."
                );
            }
        }
        "gs" => {
            if std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err()
                && std::env::var("GOOGLE_SERVICE_ACCOUNT").is_err()
            {
                anyhow::bail!(
                    "Google Cloud credentials not found. Please set GOOGLE_APPLICATION_CREDENTIALS or GOOGLE_SERVICE_ACCOUNT environment variable."
                );
            }
        }
        "az" | "azblob" => {
            if std::env::var("AZURE_STORAGE_ACCOUNT_NAME").is_err()
                || (std::env::var("AZURE_STORAGE_ACCOUNT_KEY").is_err()
                    && std::env::var("AZURE_STORAGE_SAS_TOKEN").is_err())
            {
                anyhow::bail!(
                    "Azure credentials not found. Please set AZURE_STORAGE_ACCOUNT_NAME and either AZURE_STORAGE_ACCOUNT_KEY or AZURE_STORAGE_SAS_TOKEN."
                );
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cloud_path() {
        assert!(is_cloud_path("s3://bucket/file.tar"));
        assert!(is_cloud_path("gs://bucket/file.tar"));
        assert!(is_cloud_path("az://container/file.tar"));
        assert!(is_cloud_path("azblob://container/file.tar"));
        assert!(!is_cloud_path("/local/path/file.tar"));
        assert!(!is_cloud_path("http://example.com/file.tar"));
    }

    #[test]
    fn test_describe_cloud_location() {
        let desc = describe_cloud_location("s3://my-bucket/path/to/file.tar");
        assert!(desc.contains("Amazon S3"));
        assert!(desc.contains("my-bucket"));

        let desc = describe_cloud_location("gs://gcs-bucket/archive.tar.gz");
        assert!(desc.contains("Google Cloud Storage"));
        assert!(desc.contains("gcs-bucket"));
    }
}
