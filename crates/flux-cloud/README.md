# flux-cloud

Cloud storage adaptation layer for Flux. This crate provides synchronous `Read`, `Write`, and `Seek` interfaces for cloud storage objects, allowing `flux-core` to work seamlessly with cloud storage without dealing with async complexity.

## Features

- **Synchronous API**: Implements standard `std::io::{Read, Write, Seek}` traits
- **Multi-cloud support**: Works with AWS S3, Google Cloud Storage, Azure Blob Storage
- **Intelligent buffering**: Configurable read/write buffers for optimal performance
- **Caching**: LRU cache for recently read chunks to minimize API calls
- **Multipart uploads**: Automatic multipart upload for large files
- **Zero-copy where possible**: Uses `bytes::Bytes` internally

## Usage

```rust
use flux_cloud::{CloudReader, CloudWriter, parse_cloud_url};
use std::io::{Read, Write};

// Parse a cloud URL
let (store, path) = parse_cloud_url("s3://my-bucket/archive.tar.gz")?;

// Write to cloud storage
let mut writer = CloudWriter::new_with_defaults(store.clone(), path.clone());
writer.write_all(b"Hello, cloud!")?;
writer.finalize()?;

// Read from cloud storage
let mut reader = CloudReader::new_with_defaults(store, path).await?;
let mut content = String::new();
reader.read_to_string(&mut content)?;
```

## Configuration

```rust
use flux_cloud::CloudConfig;

let config = CloudConfig {
    read_buffer_size: 8 * 1024 * 1024,    // 8MB read chunks
    write_buffer_size: 16 * 1024 * 1024,  // 16MB write buffer
    read_cache_size: 4,                    // Cache 4 chunks
    use_multipart_upload: true,            // Enable multipart
    multipart_threshold: 64 * 1024 * 1024, // 64MB threshold
};
```

## Authentication

Cloud credentials are read from environment variables:

- **AWS S3**: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`
- **Google Cloud**: `GOOGLE_SERVICE_ACCOUNT` or Application Default Credentials
- **Azure**: `AZURE_STORAGE_ACCOUNT`, `AZURE_STORAGE_ACCESS_KEY`

## Architecture

The crate uses an internal Tokio runtime to bridge async `object_store` operations with synchronous `std::io` traits. This design allows `flux-core` to remain purely synchronous while still leveraging high-performance async cloud APIs.

### Key Components

- `CloudReader`: Provides buffered, seekable reads from cloud objects
- `CloudWriter`: Provides buffered writes with automatic multipart upload
- `ReadBuffer`: Manages downloaded chunks with caching
- `WriteBuffer`: Accumulates data before uploading

## Performance Considerations

1. **Chunk Size**: Larger read buffers reduce API calls but increase memory usage
2. **Cache Size**: More cached chunks improve sequential read performance
3. **Multipart Threshold**: Lower thresholds start streaming uploads sooner
4. **Network Latency**: Consider your network conditions when tuning buffer sizes

## License

Licensed under either of Apache License 2.0 or MIT license at your option.