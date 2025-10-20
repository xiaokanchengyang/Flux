use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use std::num::NonZeroUsize;
use bytes::Bytes;
use lru::LruCache;
use object_store::path::Path;
use tracing::{debug, trace};
use crate::{CloudStore, CloudPath, CloudConfig, Result, CloudError};

/// A reader that adapts cloud storage objects to implement std::io::Read and Seek
pub struct CloudReader {
    store: CloudStore,
    path: Path,
    /// Current position in the file
    position: u64,
    /// Total size of the object
    size: u64,
    /// Configuration
    config: CloudConfig,
    /// Cache of recently read chunks
    cache: Arc<Mutex<LruCache<u64, Buffer>>>,
}

struct Buffer {
    data: Bytes,
    /// Start position of this buffer in the file
    start: u64,
}

impl CloudReader {
    /// Create a new CloudReader for the given cloud URL
    pub fn new(url: &str) -> Result<Self> {
        Self::with_config(url, CloudConfig::default())
    }
    
    /// Create a new CloudReader with custom configuration
    pub fn with_config(url: &str, config: CloudConfig) -> Result<Self> {
        let cloud_path = CloudPath::parse(url)?;
        let store = CloudStore::new(&cloud_path)?;
        Self::from_store_with_config(store, cloud_path.path, config)
    }
    
    /// Create a CloudReader from an existing CloudStore and path
    pub fn from_store(store: CloudStore, path: Path) -> Result<Self> {
        Self::from_store_with_config(store, path, CloudConfig::default())
    }
    
    /// Create a CloudReader from an existing CloudStore and path with custom config
    pub fn from_store_with_config(store: CloudStore, path: Path, config: CloudConfig) -> Result<Self> {
        // Get object metadata to know the size
        let meta = store.runtime().block_on(async {
            store.store().head(&path).await
        }).map_err(CloudError::ObjectStore)?;
        
        let cache_size = NonZeroUsize::new(config.read_cache_size.max(1))
            .expect("cache size must be at least 1");
        
        Ok(CloudReader {
            store,
            path,
            position: 0,
            size: meta.size as u64,
            config,
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
        })
    }
    
    /// Get the size of the object
    pub fn size(&self) -> u64 {
        self.size
    }
    
    /// Get the current position
    pub fn position(&self) -> u64 {
        self.position
    }
    
    /// Download a chunk of data from the cloud
    fn fetch_chunk(&mut self, start: u64, len: usize) -> Result<Bytes> {
        let end = (start + len as u64).min(self.size);
        
        debug!(
            "Downloading chunk from {}: {:?}",
            self.path,
            start..end
        );
        
        let data = self.store.runtime().block_on(async {
            self.store.store()
                .get_range(&self.path, start as usize..end as usize)
                .await
        }).map_err(CloudError::ObjectStore)?;
        
        Ok(data)
    }
    
    /// Get data from cache or download
    fn get_chunk(&mut self, position: u64) -> Result<Buffer> {
        // Calculate chunk boundaries aligned to buffer size
        let chunk_start = (position / self.config.read_buffer_size as u64) 
            * self.config.read_buffer_size as u64;
        
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(buffer) = cache.get(&chunk_start) {
                if position >= buffer.start && position < buffer.start + buffer.data.len() as u64 {
                    trace!("Cache hit for position {}", position);
                    return Ok(Buffer {
                        data: buffer.data.clone(),
                        start: buffer.start,
                    });
                }
            }
        }
        
        // Cache miss - download chunk
        trace!("Cache miss for position {}", position);
        
        let chunk_len = self.config.read_buffer_size
            .min((self.size - chunk_start) as usize);
        let data = self.fetch_chunk(chunk_start, chunk_len)?;
        
        let buffer = Buffer {
            data: data.clone(),
            start: chunk_start,
        };
        
        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(chunk_start, Buffer {
                data,
                start: chunk_start,
            });
        }
        
        Ok(buffer)
    }
}

impl Read for CloudReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.size {
            return Ok(0); // EOF
        }
        
        let chunk = self.get_chunk(self.position)?;
        let buffer_offset = (self.position - chunk.start) as usize;
        let available = chunk.data.len() - buffer_offset;
        let to_read = buf.len().min(available).min((self.size - self.position) as usize);
        
        if to_read > 0 {
            let src = &chunk.data[buffer_offset..buffer_offset + to_read];
            buf[..to_read].copy_from_slice(src);
            self.position += to_read as u64;
        }
        
        Ok(to_read)
    }
}

impl Seek for CloudReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset > 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek beyond end of file",
                    ));
                }
                (self.size as i64 + offset) as u64
            }
            SeekFrom::Current(offset) => {
                let new_pos = self.position as i64 + offset;
                if new_pos < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek before start of file",
                    ));
                }
                new_pos as u64
            }
        };
        
        if new_pos > self.size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot seek beyond end of file",
            ));
        }
        
        self.position = new_pos;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cloud_path_parsing() {
        let path = CloudPath::parse("s3://my-bucket/path/to/file.tar").unwrap();
        assert_eq!(path.scheme, "s3");
        assert_eq!(path.bucket, "my-bucket");
        assert_eq!(path.path.as_ref(), "path/to/file.tar");
        
        let path = CloudPath::parse("gs://gcs-bucket/archive.tar.gz").unwrap();
        assert_eq!(path.scheme, "gs");
        assert_eq!(path.bucket, "gcs-bucket");
        
        let path = CloudPath::parse("az://container/blob.tar").unwrap();
        assert_eq!(path.scheme, "az");
        assert_eq!(path.bucket, "container");
    }
    
    #[test]
    fn test_invalid_paths() {
        assert!(CloudPath::parse("http://not-cloud/file").is_err());
        assert!(CloudPath::parse("/local/path/file").is_err());
        assert!(CloudPath::parse("s3://").is_err());
    }
}