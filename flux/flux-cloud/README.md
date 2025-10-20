# flux-cloud

Cloud storage adaptation layer for Flux archive tool. This crate provides transparent cloud storage support by implementing standard Rust I/O traits (`Read`, `Write`, `Seek`) for cloud objects.

## Features

- **Transparent Integration**: Cloud objects behave like local files through standard I/O traits
- **Multi-Cloud Support**: Works with Amazon S3, Google Cloud Storage, and Azure Blob Storage
- **Efficient Streaming**: Optimized buffering and multipart uploads for large files
- **Synchronous API**: Seamlessly integrates with flux-core's synchronous architecture

## Architecture

```
flux-core (sync) <-> flux-cloud (async adapter) <-> Cloud Providers
```

The key innovation is that `flux-cloud` internally manages a Tokio runtime to execute async operations, but exposes a synchronous API that flux-core can use without any modifications.

## Usage

### As a Library

```rust
use flux_cloud::{CloudReader, CloudWriter};
use std::io::{Read, Write, copy};

// Read from cloud storage
let mut reader = CloudReader::new("s3://my-bucket/archive.tar.gz")?;
let mut buffer = Vec::new();
reader.read_to_end(&mut buffer)?;

// Write to cloud storage
let mut writer = CloudWriter::new("gs://my-bucket/backup.tar.zst")?;
writer.write_all(&data)?;
writer.flush()?;

// Copy between cloud providers
let mut source = CloudReader::new("s3://source-bucket/file.tar")?;
let mut dest = CloudWriter::new("az://dest-container/file.tar")?;
copy(&mut source, &mut dest)?;
```

### Environment Variables

#### Amazon S3
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1  # optional
```

#### Google Cloud Storage
```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
# OR
export GOOGLE_SERVICE_ACCOUNT='{"type": "service_account", ...}'
```

#### Azure Blob Storage
```bash
export AZURE_STORAGE_ACCOUNT_NAME=myaccount
export AZURE_STORAGE_ACCOUNT_KEY=your_account_key
# OR
export AZURE_STORAGE_SAS_TOKEN=your_sas_token
```

## Design Principles

1. **Zero Changes to flux-core**: The core library remains pure, synchronous, and cloud-agnostic
2. **Efficient Buffering**: Uses 8MB buffers by default to minimize round trips
3. **Smart Uploads**: Automatically switches to multipart upload for large files (>16MB)
4. **Error Handling**: Comprehensive error messages with context

## Performance Characteristics

- **Read Operations**: Fetches data in 8MB chunks, with read-ahead buffering
- **Write Operations**: Buffers up to 8MB before uploading, automatic multipart for large files
- **Seek Operations**: Optimized to avoid unnecessary downloads when seeking forward

## Integration with flux-cli

When flux-cli is built with the `cloud` feature, it automatically detects cloud URLs:

```bash
# Build with cloud support
cargo build --features cloud

# Use cloud URLs directly
flux pack -i ./data -o s3://bucket/backup.tar.zst
flux extract gs://bucket/archive.tar.gz -o ./output
flux inspect az://container/data.tar
```

## Future Enhancements

- [ ] Parallel multipart uploads for even faster large file handling
- [ ] Resumable uploads/downloads
- [ ] Cloud-to-cloud copying without local buffering
- [ ] Credential caching and management
- [ ] Progress callbacks for long operations

## License

Same as the parent Flux project - MIT OR Apache-2.0