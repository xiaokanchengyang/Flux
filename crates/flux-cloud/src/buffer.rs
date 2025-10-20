//! Buffer management utilities

use bytes::{Bytes, BytesMut};
use std::ops::Range;

/// A read buffer that caches downloaded chunks
#[derive(Debug)]
pub(crate) struct ReadBuffer {
    /// The actual data
    data: Bytes,
    /// The range in the source this buffer represents
    range: Range<u64>,
}

impl ReadBuffer {
    /// Create a new read buffer
    pub fn new(data: Bytes, range: Range<u64>) -> Self {
        Self { data, range }
    }
    
    /// Check if this buffer contains the given position
    pub fn contains(&self, pos: u64) -> bool {
        self.range.contains(&pos)
    }
    
    /// Get data starting from the given position
    pub fn get_from(&self, pos: u64) -> Option<&[u8]> {
        if !self.contains(pos) {
            return None;
        }
        
        let offset = (pos - self.range.start) as usize;
        Some(&self.data[offset..])
    }
}

/// A write buffer that accumulates data before uploading
#[derive(Debug)]
pub(crate) struct WriteBuffer {
    /// The buffer
    buffer: BytesMut,
    /// Maximum capacity
    capacity: usize,
}

impl WriteBuffer {
    /// Create a new write buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
            capacity,
        }
    }
    
    /// Get the current length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.buffer.len()
    }
    
    /// Write data to the buffer
    pub fn write(&mut self, data: &[u8]) -> usize {
        let to_write = data.len().min(self.remaining());
        self.buffer.extend_from_slice(&data[..to_write]);
        to_write
    }
    
    /// Take the buffer contents, leaving it empty
    pub fn take(&mut self) -> Bytes {
        self.buffer.split().freeze()
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}