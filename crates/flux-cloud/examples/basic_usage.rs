//! Basic usage example for flux-cloud
//!
//! This example demonstrates how to use CloudReader and CloudWriter
//! to work with cloud storage objects using synchronous I/O operations.

use flux_cloud::{CloudReader, CloudWriter, CloudConfig, parse_cloud_url};
use std::io::{Read, Write, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example S3 URL (you'll need to set up credentials via environment variables)
    // AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
    let url = "s3://my-bucket/test-object.txt";
    
    // Parse the URL to get store and path
    let (store, path) = parse_cloud_url(url)?;
    
    // Writing example
    println!("Writing to cloud storage...");
    {
        let mut writer = CloudWriter::from_store(store.clone(), path.clone())?;
        
        // Write some data
        writer.write_all(b"Hello from flux-cloud!\n")?;
        writer.write_all(b"This is a test of synchronous cloud I/O.\n")?;
        
        // Write more data to demonstrate buffering
        for i in 0..100 {
            writeln!(writer, "Line {}: Some test data to demonstrate buffering", i)?;
        }
        
        // Finalize the upload
        writer.finalize()?;
    }
    
    // Reading example
    println!("\nReading from cloud storage...");
    {
        let mut reader = CloudReader::from_store(store.clone(), path.clone())?;
        
        // Read the first 50 bytes
        let mut buffer = vec![0; 50];
        let n = reader.read(&mut buffer)?;
        println!("Read {} bytes: {:?}", n, String::from_utf8_lossy(&buffer[..n]));
        
        // Seek to a specific position
        reader.seek(SeekFrom::Start(100))?;
        println!("Seeked to position 100");
        
        // Read more data
        let mut buffer = vec![0; 100];
        let n = reader.read(&mut buffer)?;
        println!("Read {} bytes from position 100: {:?}", n, String::from_utf8_lossy(&buffer[..n]));
        
        // Seek relative to current position
        reader.seek(SeekFrom::Current(50))?;
        println!("Seeked forward 50 bytes");
        
        // Read to end
        let mut remaining = String::new();
        reader.read_to_string(&mut remaining)?;
        println!("Remaining content length: {} bytes", remaining.len());
    }
    
    // Advanced: Using custom configuration
    println!("\nUsing custom configuration...");
    {
        let config = CloudConfig {
            read_buffer_size: 4 * 1024 * 1024,  // 4MB chunks
            write_buffer_size: 16 * 1024 * 1024, // 16MB buffer
            read_cache_size: 8,                   // Cache 8 chunks
            use_multipart_upload: true,
            multipart_threshold: 32 * 1024 * 1024, // 32MB
        };
        
        let mut writer = CloudWriter::from_store_with_config(
            store.clone(), 
            path.clone(), 
            config.clone()
        )?;
        writer.write_all(b"Data with custom config\n")?;
        writer.finalize()?;
        
        let mut reader = CloudReader::from_store_with_config(store, path, config)?;
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        println!("Read with custom config: {}", content.lines().next().unwrap_or(""));
    }
    
    Ok(())
}