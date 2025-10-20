//! CloudWriter - provides synchronous Write trait for cloud objects

use crate::{buffer::WriteBuffer, error::Result, runtime::get_runtime, CloudConfig, CloudError};
use bytes::Bytes;
use object_store::{ObjectStore, path::Path as ObjectPath, MultipartUpload, PutPayload};
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, trace};

/// A writer that provides synchronous write access to cloud storage objects
#[derive(Debug)]
pub struct CloudWriter {
    /// The object store
    store: Arc<dyn ObjectStore>,
    /// Path to the object
    path: ObjectPath,
    /// Write buffer
    buffer: WriteBuffer,
    /// Configuration
    config: CloudConfig,
    /// Total bytes written
    bytes_written: u64,
    /// Multipart upload handle (if using multipart)
    multipart: Option<Box<dyn MultipartUpload>>,
}

impl CloudWriter {
    /// Create a new CloudWriter
    ///
    /// # Arguments
    /// * `store` - The object store to write to
    /// * `path` - Path to the object
    /// * `config` - Configuration for the writer
    pub fn new(
        store: Arc<dyn ObjectStore>,
        path: ObjectPath,
        config: CloudConfig,
    ) -> Self {
        Self {
            store,
            path,
            buffer: WriteBuffer::new(config.write_buffer_size),
            config,
            bytes_written: 0,
            multipart: None,
        }
    }
    
    /// Create a new CloudWriter with default configuration
    pub fn new_with_defaults(
        store: Arc<dyn ObjectStore>,
        path: ObjectPath,
    ) -> Self {
        Self::new(store, path, CloudConfig::default())
    }
    
    /// Get the total number of bytes written
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
    
    /// Upload the current buffer contents
    async fn upload_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        
        let data = self.buffer.take();
        
        if let Some(multipart) = &mut self.multipart {
            // Multipart upload
            debug!("Uploading multipart chunk of {} bytes", data.len());
            multipart.put_part(data.into()).await?;
        } else if self.config.use_multipart_upload 
            && self.bytes_written + data.len() as u64 > self.config.multipart_threshold as u64 
        {
            // Start multipart upload
            debug!("Starting multipart upload for {}", self.path);
            let runtime = get_runtime();
            let store = self.store.clone();
            let path = self.path.clone();
            
            let mut multipart = runtime.block_on(async {
                store.put_multipart(&path).await
            })?;
            
            multipart.put_part(data.into()).await?;
            self.multipart = Some(multipart);
        } else {
            // For small writes, we'll accumulate and do a single PUT at the end
            // Put the data back in the buffer
            self.buffer.write(&data);
        }
        
        Ok(())
    }
    
    /// Finalize the write operation
    async fn finalize_internal(&mut self) -> Result<()> {
        if let Some(multipart) = self.multipart.take() {
            // Complete multipart upload
            debug!("Completing multipart upload for {}", self.path);
            
            // Upload any remaining buffer
            if !self.buffer.is_empty() {
                let data = self.buffer.take();
                multipart.put_part(data.into()).await?;
            }
            
            multipart.complete().await?;
        } else {
            // Single PUT operation
            let data = self.buffer.take();
            if !data.is_empty() {
                debug!("Uploading {} bytes to {}", data.len(), self.path);
                self.store.put(&self.path, data.into()).await?;
            }
        }
        
        Ok(())
    }
    
    /// Finalize the write operation
    ///
    /// This must be called to ensure all data is uploaded to the cloud.
    /// It's automatically called on drop, but calling it explicitly allows
    /// for proper error handling.
    pub fn finalize(mut self) -> Result<()> {
        let runtime = get_runtime();
        runtime.block_on(self.finalize_internal())
    }
}

impl Write for CloudWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        
        let mut written = 0;
        let mut remaining = buf;
        
        while !remaining.is_empty() {
            let n = self.buffer.write(remaining);
            written += n;
            self.bytes_written += n as u64;
            remaining = &remaining[n..];
            
            // If buffer is full, upload it
            if self.buffer.remaining() == 0 {
                trace!("Buffer full, uploading {} bytes", self.buffer.len());
                let runtime = get_runtime();
                runtime.block_on(self.upload_buffer())?;
            }
        }
        
        Ok(written)
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        // In cloud storage, flush doesn't immediately upload
        // We only upload on buffer full or finalize
        Ok(())
    }
}

impl Drop for CloudWriter {
    fn drop(&mut self) {
        if self.buffer.len() > 0 || self.multipart.is_some() {
            // Try to finalize, but we can't propagate errors from drop
            let runtime = get_runtime();
            if let Err(e) = runtime.block_on(self.finalize_internal()) {
                eprintln!("Warning: Failed to finalize CloudWriter on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests will be added here
}