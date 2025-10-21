use crate::{CloudError, CloudPath, CloudStore, Result};
use bytes::Bytes;
use object_store::path::Path;
use std::io::{Read, Seek, SeekFrom};

const DEFAULT_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB buffer

/// A reader that adapts cloud storage objects to implement `std::io::Read` and `Seek`
pub struct CloudReader {
    store: CloudStore,
    path: Path,
    /// Current position in the file
    position: u64,
    /// Total size of the object
    size: u64,
    /// Buffer for cached data
    buffer: Option<Buffer>,
}

struct Buffer {
    data: Bytes,
    /// Start position of this buffer in the file
    start: u64,
}

impl CloudReader {
    /// Create a new `CloudReader` for the given cloud URL
    ///
    /// # Errors
    /// Returns an error if the URL is invalid or the cloud store cannot be created
    pub fn new(url: &str) -> Result<Self> {
        let cloud_path = CloudPath::parse(url)?;
        let store = CloudStore::new(&cloud_path)?;

        // Get object metadata to know the size
        let meta = store
            .runtime()
            .block_on(async { store.store().head(&cloud_path.path).await })
            .map_err(CloudError::ObjectStore)?;

        Ok(CloudReader {
            store,
            path: cloud_path.path,
            position: 0,
            size: meta.size as u64,
            buffer: None,
        })
    }

    /// Create a `CloudReader` from an existing `CloudStore` and path
    ///
    /// # Errors
    /// Returns an error if the object metadata cannot be retrieved
    pub fn from_store(store: CloudStore, path: Path) -> Result<Self> {
        // Get object metadata to know the size
        let meta = store
            .runtime()
            .block_on(async { store.store().head(&path).await })
            .map_err(CloudError::ObjectStore)?;

        Ok(CloudReader {
            store,
            path,
            position: 0,
            size: meta.size as u64,
            buffer: None,
        })
    }

    /// Download a chunk of data from the cloud
    fn fetch_chunk(&mut self, start: u64, len: usize) -> Result<Bytes> {
        let end = (start + len as u64).min(self.size);

        let data = self
            .store
            .runtime()
            .block_on(async {
                self.store
                    .store()
                    .get_range(&self.path, start as usize..end as usize)
                    .await
            })
            .map_err(CloudError::ObjectStore)?;

        Ok(data)
    }

    /// Ensure we have buffered data at the current position
    fn ensure_buffer(&mut self) -> Result<()> {
        // Check if we already have data buffered at this position
        if let Some(ref buffer) = self.buffer {
            let buffer_end = buffer.start + buffer.data.len() as u64;
            if self.position >= buffer.start && self.position < buffer_end {
                return Ok(());
            }
        }

        // We need to fetch new data
        if self.position >= self.size {
            // Already at end of file
            return Ok(());
        }

        let chunk_size = DEFAULT_BUFFER_SIZE.min((self.size - self.position) as usize);
        let data = self.fetch_chunk(self.position, chunk_size)?;

        self.buffer = Some(Buffer {
            data,
            start: self.position,
        });

        Ok(())
    }
}

impl Read for CloudReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.size {
            return Ok(0); // EOF
        }

        self.ensure_buffer()?;

        if let Some(ref buffer) = self.buffer {
            let buffer_offset = (self.position - buffer.start) as usize;
            let available = buffer.data.len() - buffer_offset;
            let to_read = buf.len().min(available);

            if to_read > 0 {
                let src = &buffer.data[buffer_offset..buffer_offset + to_read];
                buf[..to_read].copy_from_slice(src);
                self.position += to_read as u64;
                return Ok(to_read);
            }
        }

        Ok(0)
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
