//! Integration tests for flux-cloud

use flux_cloud::{CloudReader, CloudWriter, CloudConfig};
use object_store::memory::InMemory;
use object_store::path::Path as ObjectPath;
use std::io::{Read, Write, Seek, SeekFrom};
use std::sync::Arc;

#[tokio::test]
async fn test_write_and_read() {
    // Use in-memory object store for testing
    let store = Arc::new(InMemory::new());
    let path = ObjectPath::from("test/file.txt");
    
    let test_data = b"Hello, flux-cloud! This is a test.";
    
    // Write data
    {
        let mut writer = CloudWriter::new_with_defaults(store.clone(), path.clone());
        writer.write_all(test_data).unwrap();
        writer.finalize().unwrap();
    }
    
    // Read data back
    {
        let mut reader = CloudReader::new_with_defaults(store.clone(), path.clone())
            .await
            .unwrap();
        
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).unwrap();
        
        assert_eq!(buffer, test_data);
    }
}

#[tokio::test]
async fn test_seek_operations() {
    let store = Arc::new(InMemory::new());
    let path = ObjectPath::from("test/seekable.txt");
    
    // Create test data
    let test_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    
    // Write data
    {
        let mut writer = CloudWriter::new_with_defaults(store.clone(), path.clone());
        writer.write_all(&test_data).unwrap();
        writer.finalize().unwrap();
    }
    
    // Test various seek operations
    {
        let mut reader = CloudReader::new_with_defaults(store.clone(), path.clone())
            .await
            .unwrap();
        
        // Seek to start
        assert_eq!(reader.seek(SeekFrom::Start(0)).unwrap(), 0);
        
        // Read first 10 bytes
        let mut buffer = vec![0; 10];
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, &test_data[0..10]);
        
        // Seek to position 500
        assert_eq!(reader.seek(SeekFrom::Start(500)).unwrap(), 500);
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, &test_data[500..510]);
        
        // Seek relative to current position
        assert_eq!(reader.seek(SeekFrom::Current(100)).unwrap(), 610);
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, &test_data[610..620]);
        
        // Seek from end
        assert_eq!(reader.seek(SeekFrom::End(-10)).unwrap(), 990);
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, &test_data[990..1000]);
    }
}

#[tokio::test]
async fn test_large_file_with_buffering() {
    let store = Arc::new(InMemory::new());
    let path = ObjectPath::from("test/large.bin");
    
    // Create 10MB of test data
    let test_size = 10 * 1024 * 1024;
    let test_data: Vec<u8> = (0..test_size).map(|i| (i % 256) as u8).collect();
    
    // Write with small buffer to test multipart logic
    let config = CloudConfig {
        write_buffer_size: 1024 * 1024, // 1MB buffer
        multipart_threshold: 2 * 1024 * 1024, // 2MB threshold
        ..Default::default()
    };
    
    {
        let mut writer = CloudWriter::new(store.clone(), path.clone(), config);
        writer.write_all(&test_data).unwrap();
        writer.finalize().unwrap();
    }
    
    // Read back and verify
    {
        let mut reader = CloudReader::new_with_defaults(store.clone(), path.clone())
            .await
            .unwrap();
        
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).unwrap();
        
        assert_eq!(buffer.len(), test_data.len());
        assert_eq!(buffer, test_data);
    }
}

#[tokio::test]
async fn test_cache_efficiency() {
    let store = Arc::new(InMemory::new());
    let path = ObjectPath::from("test/cache.txt");
    
    // Create test data with recognizable patterns
    let mut test_data = Vec::new();
    for i in 0..1000 {
        test_data.extend_from_slice(format!("Line {}: test data\n", i).as_bytes());
    }
    
    // Write data
    {
        let mut writer = CloudWriter::new_with_defaults(store.clone(), path.clone());
        writer.write_all(&test_data).unwrap();
        writer.finalize().unwrap();
    }
    
    // Read with caching
    let config = CloudConfig {
        read_buffer_size: 1024,
        read_cache_size: 4,
        ..Default::default()
    };
    
    {
        let mut reader = CloudReader::new(store.clone(), path.clone(), config)
            .await
            .unwrap();
        
        // Read same region multiple times (should hit cache)
        for _ in 0..3 {
            reader.seek(SeekFrom::Start(0)).unwrap();
            let mut buffer = vec![0; 100];
            reader.read_exact(&mut buffer).unwrap();
            assert_eq!(buffer, &test_data[0..100]);
        }
        
        // Jump around to test cache eviction
        for offset in [0, 5000, 10000, 15000, 0].iter() {
            reader.seek(SeekFrom::Start(*offset)).unwrap();
            let mut buffer = vec![0; 100];
            reader.read_exact(&mut buffer).unwrap();
            assert_eq!(buffer, &test_data[*offset as usize..(*offset as usize + 100)]);
        }
    }
}

#[tokio::test]
async fn test_empty_file() {
    let store = Arc::new(InMemory::new());
    let path = ObjectPath::from("test/empty.txt");
    
    // Write empty file
    {
        let writer = CloudWriter::new_with_defaults(store.clone(), path.clone());
        writer.finalize().unwrap();
    }
    
    // Read empty file
    {
        let mut reader = CloudReader::new_with_defaults(store.clone(), path.clone())
            .await
            .unwrap();
        
        let mut buffer = Vec::new();
        let n = reader.read_to_end(&mut buffer).unwrap();
        assert_eq!(n, 0);
        assert_eq!(buffer.len(), 0);
        assert_eq!(reader.size(), 0);
    }
}