use std::sync::Arc;
use url::Url;
use object_store::DynObjectStore;
use object_store::path::Path;
use crate::{CloudError, Result};

/// Represents a path in cloud storage
#[derive(Debug, Clone)]
pub struct CloudPath {
    /// The cloud storage scheme (s3, gs, az)
    pub scheme: String,
    /// The bucket or container name
    pub bucket: String,
    /// The object path within the bucket
    pub path: Path,
}

impl CloudPath {
    /// Parse a cloud URL like "s3://bucket/path/to/object"
    pub fn parse(url: &str) -> Result<Self> {
        let parsed = Url::parse(url)
            .map_err(|e| CloudError::InvalidPath(format!("Invalid URL: {}", e)))?;
        
        let scheme = parsed.scheme().to_string();
        if !["s3", "gs", "az", "azblob"].contains(&scheme.as_str()) {
            return Err(CloudError::InvalidPath(
                format!("Unsupported scheme: {}. Use s3://, gs://, or az://", scheme)
            ));
        }
        
        let bucket = parsed.host_str()
            .ok_or_else(|| CloudError::InvalidPath("Missing bucket name".to_string()))?
            .to_string();
        
        let path = Path::from(parsed.path().trim_start_matches('/'));
        
        Ok(CloudPath { scheme, bucket, path })
    }
}

/// Manages the object store instance and Tokio runtime
#[derive(Clone)]
pub struct CloudStore {
    store: Arc<DynObjectStore>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl CloudStore {
    /// Create a new CloudStore for the given cloud path
    pub fn new(path: &CloudPath) -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| CloudError::Runtime(format!("Failed to create Tokio runtime: {}", e)))?;
        
        let store = runtime.block_on(async {
            create_object_store(&path.scheme, &path.bucket).await
        })?;
        
        Ok(CloudStore {
            store: Arc::new(store),
            runtime: Arc::new(runtime),
        })
    }
    
    /// Create a CloudStore from existing store and runtime (useful for testing)
    pub fn from_store_and_runtime(store: Arc<DynObjectStore>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        CloudStore { store, runtime }
    }
    
    /// Get the object store instance
    pub fn store(&self) -> &Arc<DynObjectStore> {
        &self.store
    }
    
    /// Get the Tokio runtime
    pub fn runtime(&self) -> &Arc<tokio::runtime::Runtime> {
        &self.runtime
    }
}

async fn create_object_store(scheme: &str, bucket: &str) -> Result<Box<DynObjectStore>> {
    match scheme {
        "s3" => {
            let store = object_store::aws::AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .build()
                .map_err(CloudError::ObjectStore)?;
            Ok(Box::new(store))
        }
        "gs" => {
            let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
                .with_bucket_name(bucket)
                .build()
                .map_err(CloudError::ObjectStore)?;
            Ok(Box::new(store))
        }
        "az" | "azblob" => {
            let store = object_store::azure::MicrosoftAzureBuilder::from_env()
                .with_container_name(bucket)
                .build()
                .map_err(CloudError::ObjectStore)?;
            Ok(Box::new(store))
        }
        _ => Err(CloudError::InvalidPath(format!("Unsupported scheme: {}", scheme))),
    }
}