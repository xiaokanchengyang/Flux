//! CloudReader - provides synchronous Read and Seek traits for cloud objects

use crate::{buffer::ReadBuffer, error::Result, runtime::get_runtime, CloudConfig, CloudError};
use bytes::Bytes;
use lru::LruCache;
use object_store::{ObjectStore, path::Path as ObjectPath};
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace};

/// A reader that provides synchronous access to cloud storage objects
#[derive(Debug)]
pub struct CloudReader {
    /// The object store
    store: Arc<dyn ObjectStore>,
    /// Path to the object
    path: ObjectPath,
    /// Current read position
    position: u64,
    /// Total size of the object
    size: u64,
    /// Configuration
    config: CloudConfig,
    /// Cache of recently read chunks
    cache: Arc<Mutex<LruCache<u64, ReadBuffer>>>,
}

impl CloudReader {
    /// Create a new CloudReader
    ///
    /// # Arguments
    /// * `store` - The object store to read from
    /// * `path` - Path to the object
    /// * `config` - Configuration for the reader
    pub async fn new(
        store: Arc<dyn ObjectStore>,
        path: ObjectPath,
        config: CloudConfig,
    ) -> Result<Self> {
        // Get object metadata to determine size
        let meta = store.head(&path).await?;
        let size = meta.size as u64;
        
        let cache_size = NonZeroUsize::new(config.read_cache_size.max(1))
            .expect("cache size must be at least 1");
        
        Ok(Self {
            store,
            path,
            position: 0,
            size,
            config,
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
        })
    }
    
    /// Create a new CloudReader with default configuration
    pub async fn new_with_defaults(
        store: Arc<dyn ObjectStore>,
        path: ObjectPath,
    ) -> Result<Self> {
        Self::new(store, path, CloudConfig::default()).await
    }
    
    /// Get the size of the object
    pub fn size(&self) -> u64 {
        self.size
    }
    
    /// Get the current position
    pub fn position(&self) -> u64 {
        self.position
    }
    
    /// Download a chunk of data
    async fn download_chunk(&self, start: u64, len: usize) -> Result<Bytes> {
        let end = (start + len as u64).min(self.size);
        let range = start..end;
        
        debug!(
            "Downloading chunk from {} for object {}: {:?}",
            self.store.to_string(),
            self.path,
            range
        );
        
        let result = self.store.get_range(&self.path, range.clone()).await?;
        
        Ok(result)
    }
    
    /// Get data from cache or download
    fn get_chunk(&self, position: u64) -> Result<ReadBuffer> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(buffer) = cache.get(&position) {
                if buffer.contains(position) {
                    trace!("Cache hit for position {}", position);
                    return Ok(ReadBuffer::new(
                        buffer.data.clone(),
                        buffer.range.clone(),
                    ));
                }
            }
        }
        
        // Cache miss - download chunk
        trace!("Cache miss for position {}", position);
        
        let chunk_start = (position / self.config.read_buffer_size as u64) 
            * self.config.read_buffer_size as u64;
        let chunk_len = self.config.read_buffer_size;
        
        let runtime = get_runtime();
        let data = runtime.block_on(self.download_chunk(chunk_start, chunk_len))?;
        
        let range = chunk_start..(chunk_start + data.len() as u64);
        let buffer = ReadBuffer::new(data, range.clone());
        
        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(chunk_start, ReadBuffer::new(buffer.data.clone(), range));
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
        let data = chunk.get_from(self.position)
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get data from chunk",
            ))?;
        
        let to_read = buf.len().min(data.len()).min((self.size - self.position) as usize);
        buf[..to_read].copy_from_slice(&data[..to_read]);
        
        self.position += to_read as u64;
        Ok(to_read)
    }
}

impl Seek for CloudReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => {
                if n > 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek beyond end of file",
                    ));
                }
                (self.size as i64 + n) as u64
            }
            SeekFrom::Current(n) => {
                let new = self.position as i64 + n;
                if new < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek before beginning of file",
                    ));
                }
                new as u64
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
    
    // Tests will be added here
}