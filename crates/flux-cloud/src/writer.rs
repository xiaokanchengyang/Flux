use crate::{CloudError, CloudPath, CloudStore, Result};
use bytes::{BufMut, BytesMut};
use object_store::path::Path;
use object_store::MultipartUpload;
use std::io::Write;

const DEFAULT_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB buffer
const MULTIPART_THRESHOLD: usize = 16 * 1024 * 1024; // 16MB threshold for multipart

/// A writer that adapts cloud storage to implement std::io::Write
pub struct CloudWriter {
    store: CloudStore,
    path: Path,
    /// Buffer for accumulating data before upload
    buffer: BytesMut,
    /// Maximum buffer size before forcing a flush
    buffer_size: usize,
    /// Total bytes written
    total_written: u64,
    /// Multipart upload handle (for large files)
    multipart: Option<Box<dyn MultipartUpload>>,
    /// Part number for multipart uploads
    part_number: usize,
}

impl CloudWriter {
    /// Create a new CloudWriter for the given cloud URL
    pub fn new(url: &str) -> Result<Self> {
        Self::with_buffer_size(url, DEFAULT_BUFFER_SIZE)
    }

    /// Create a new CloudWriter with a custom buffer size
    pub fn with_buffer_size(url: &str, buffer_size: usize) -> Result<Self> {
        let cloud_path = CloudPath::parse(url)?;
        let store = CloudStore::new(&cloud_path)?;

        Ok(CloudWriter {
            store,
            path: cloud_path.path,
            buffer: BytesMut::with_capacity(buffer_size),
            buffer_size,
            total_written: 0,
            multipart: None,
            part_number: 0,
        })
    }

    /// Create a CloudWriter from an existing CloudStore and path
    pub fn from_store(store: CloudStore, path: Path) -> Result<Self> {
        Ok(CloudWriter {
            store,
            path,
            buffer: BytesMut::with_capacity(DEFAULT_BUFFER_SIZE),
            buffer_size: DEFAULT_BUFFER_SIZE,
            total_written: 0,
            multipart: None,
            part_number: 0,
        })
    }

    /// Flush the current buffer to cloud storage
    fn flush_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let data = self.buffer.split().freeze();

        if self.multipart.is_some() {
            // We're in multipart mode, upload as a part
            self.upload_part(data)?;
        } else if self.total_written + data.len() as u64 > MULTIPART_THRESHOLD as u64 {
            // Switch to multipart mode
            self.start_multipart()?;
            self.upload_part(data)?;
        } else {
            // Still small enough for single upload, just buffer it
            // We'll upload everything on final flush/drop
            self.buffer.put(data);
        }

        Ok(())
    }

    /// Start a multipart upload
    fn start_multipart(&mut self) -> Result<()> {
        let upload = self
            .store
            .runtime()
            .block_on(async { self.store.store().put_multipart(&self.path).await })
            .map_err(CloudError::ObjectStore)?;

        self.multipart = Some(upload);
        self.part_number = 0;
        Ok(())
    }

    /// Upload a part in multipart upload
    fn upload_part(&mut self, data: bytes::Bytes) -> Result<()> {
        if let Some(ref mut upload) = self.multipart {
            self.store
                .runtime()
                .block_on(async { upload.put_part(data.into()).await })
                .map_err(CloudError::ObjectStore)?;
            self.part_number += 1;
        }
        Ok(())
    }

    /// Complete the upload (called on drop or explicit finish)
    fn finish_upload(&mut self) -> Result<()> {
        if let Some(mut upload) = self.multipart.take() {
            // Complete multipart upload
            self.flush_buffer()?;
            self.store
                .runtime()
                .block_on(async { upload.complete().await })
                .map_err(CloudError::ObjectStore)?;
        } else {
            // Simple put for small files
            let data = self.buffer.split().freeze();
            if !data.is_empty() {
                self.store
                    .runtime()
                    .block_on(async { self.store.store().put(&self.path, data.into()).await })
                    .map_err(CloudError::ObjectStore)?;
            }
        }
        Ok(())
    }
}

impl Write for CloudWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Check if adding this data would exceed buffer size
        if self.buffer.len() + buf.len() > self.buffer_size {
            self.flush_buffer()?;
        }

        // If the incoming data is larger than buffer size, handle it specially
        if buf.len() > self.buffer_size {
            // Flush any existing buffer first
            self.flush_buffer()?;

            // Start multipart if not already started
            if self.multipart.is_none() {
                self.start_multipart()?;
            }

            // Upload the large chunk directly
            self.upload_part(bytes::Bytes::copy_from_slice(buf))?;
        } else {
            // Normal case: add to buffer
            self.buffer.put_slice(buf);
        }

        self.total_written += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_buffer()?;
        Ok(())
    }
}

impl Drop for CloudWriter {
    fn drop(&mut self) {
        // Best effort to complete the upload
        let _ = self.finish_upload();
    }
}

/// A CloudWriter that completes the upload when explicitly finished
pub struct CloudWriterGuard {
    writer: Option<CloudWriter>,
}

impl CloudWriterGuard {
    pub fn new(writer: CloudWriter) -> Self {
        CloudWriterGuard {
            writer: Some(writer),
        }
    }

    /// Finish the upload and consume the writer
    pub fn finish(mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            writer.finish_upload()?;
        }
        Ok(())
    }
}

impl Write for CloudWriterGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer
            .as_mut()
            .ok_or_else(|| std::io::Error::other("Writer already finished"))?
            .write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer
            .as_mut()
            .ok_or_else(|| std::io::Error::other("Writer already finished"))?
            .flush()
    }
}

impl Drop for CloudWriterGuard {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            drop(writer); // Will call finish_upload in CloudWriter's drop
        }
    }
}
