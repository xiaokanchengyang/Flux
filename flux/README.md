# Flux

[![CI](https://github.com/your-username/flux/actions/workflows/ci.yml/badge.svg)](https://github.com/your-username/flux/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.78%2B-blue.svg)](https://www.rust-lang.org)

A high-performance, cross-platform file archiver and compressor written in Rust.

Flux is a modern replacement for traditional archiving tools, offering intelligent compression strategies, automatic algorithm selection, concurrent processing, and a user-friendly CLI experience.

## Features

- **Cross-platform**: Works seamlessly on Linux, macOS, and Windows
- **Smart compression**: Automatically selects optimal compression algorithms based on file type, size, and system resources
- **Multiple formats**: Supports common archive formats (tar, zip) and compression algorithms (zstd, xz, gzip, brotli)
- **Concurrent processing**: Leverages multiple CPU cores for faster compression/decompression
- **Metadata preservation**: Retains file permissions, timestamps, and symlinks (on Unix systems)
- **Flexible extraction**: Options for overwrite, skip, rename, and strip path components
- **Library and CLI**: Use as a standalone tool or integrate into your Rust applications
- **Memory efficient**: Handles large files without excessive memory usage
- **Progress indication**: Optional progress bars for long operations

## Installation

### From GitHub Releases

Download pre-built binaries for your platform from the [latest release](https://github.com/your-username/flux/releases/latest).

### Using Cargo

```bash
cargo install flux-cli
```

### From source

```bash
git clone https://github.com/your-username/flux.git
cd flux
cargo build --release
```

The binary will be available at `target/release/flux`.

## Usage

### Pack files into an archive

```bash
# Basic tar archive
flux pack ./my-folder -o archive.tar

# With compression (auto-detected from extension)
flux pack ./my-folder -o archive.tar.zst

# With smart compression strategy (default when no --algo specified)
flux pack ./my-folder -o archive.tar.zst --smart

# Specify compression algorithm and level
flux pack ./my-folder -o archive.tar.zst --algo zstd --level 6

# Use multiple threads
flux pack ./my-folder -o archive.tar.zst --threads 8

# Follow symlinks instead of archiving them as links
flux pack ./my-folder -o archive.tar.zst --follow-symlinks

# Force compression on already compressed files
flux pack ./my-folder -o archive.tar.zst --force-compress
```

### Extract files from an archive

```bash
# Extract to current directory
flux extract archive.tar.zst

# Extract to specific directory
flux extract archive.tar.zst -o ./extracted

# Overwrite existing files
flux extract archive.tar.zst -o ./extracted --overwrite

# Skip existing files (default behavior)
flux extract archive.tar.zst -o ./extracted --skip

# Rename conflicting files
flux extract archive.tar.zst -o ./extracted --rename

# Strip leading path components
flux extract archive.tar.zst -o ./extracted --strip-components 1
```

### Inspect archive contents

```bash
# List contents
flux inspect archive.tar.zst

# Output as JSON
flux inspect archive.tar.zst --json
```

### Configuration

```bash
# Show configuration
flux config --show

# Edit configuration
flux config --edit

# Show config file path
flux config --path
```

## Exit Codes

Flux uses standardized exit codes to indicate different types of errors:

- `0` - Success
- `1` - General error (e.g., configuration error, unknown error)
- `2` - I/O error (e.g., file not found, permission denied)
- `3` - Invalid arguments (e.g., unsupported format, invalid path)
- `4` - Partial failure (e.g., archive or compression error)

## Configuration

Flux can be configured through a TOML configuration file. The default location is:
- Linux/macOS: `~/.config/flux/config.toml`
- Windows: `%APPDATA%\flux\config.toml`

### Example Configuration

```toml
[strategy]
# Default compression level (1-9 for most algorithms)
default_level = 6

# Minimum file size to consider for compression (in bytes)
min_file_size = 1024

# Thread count (0 = auto-detect)
threads = 0

# Force compression on already compressed files
force_compress = false

[[strategy.rules]]
# Rule for text files
extensions = ["txt", "md", "json", "xml", "yml", "yaml", "toml", "ini", "cfg", "conf", "log", "html", "css", "js", "ts", "jsx", "tsx", "vue", "py", "rb", "go", "rs", "c", "cpp", "h", "hpp", "java", "kt", "swift", "sh", "bash", "zsh", "fish", "ps1", "psm1", "psd1", "bat", "cmd"]
algorithm = "zstd"
level = 6

[[strategy.rules]]
# Rule for already compressed files
extensions = ["jpg", "jpeg", "png", "gif", "webp", "mp3", "mp4", "avi", "mkv", "zip", "7z", "rar", "gz", "bz2", "xz", "zst"]
algorithm = "store"

[[strategy.rules]]
# Rule for large files
min_size = 104857600  # 100 MB
algorithm = "xz"
level = 6
```

## Smart Compression Strategy

Flux includes intelligent compression strategies that automatically select the best algorithm based on:

- **File type and extension**: Different algorithms for text, binary, and already-compressed files
- **File size**: Adjusts algorithm and thread usage for optimal performance
- **Available CPU cores**: Parallelizes when beneficial
- **System memory**: Prevents excessive memory usage
- **User preferences**: Respects configuration file settings

The smart strategy provides optimal compression without manual tuning.

## Development

### Project Structure

```
flux/
├── flux-lib/        # Core library
│   └── src/
│       ├── archive/ # Archive operations
│       ├── compress/ # Compression algorithms
│       └── strategy/ # Smart compression logic
├── flux-cli/        # CLI application
│   └── src/
│       └── main.rs
└── Cargo.toml       # Workspace configuration
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with verbose logging
RUST_LOG=debug cargo run -- pack ./test -o test.tar
```

## Supported Formats

### Archive Formats
- **tar**: Full support for packing and extracting
- **zip**: Full support for packing and extracting (note: symlinks are not supported in ZIP format)

### Compression Algorithms
- **zstd**: Fast compression with good ratios (recommended default)
- **gzip**: Wide compatibility
- **xz**: Best compression ratio, slower speed
- **brotli**: Excellent for text files
- **store**: No compression (for already compressed files)

### Format Extensions
- `.tar` - Uncompressed tar archive
- `.tar.gz`, `.tgz` - Gzip compressed tar
- `.tar.zst`, `.tzst` - Zstandard compressed tar
- `.tar.xz`, `.txz` - XZ compressed tar
- `.tar.br` - Brotli compressed tar
- `.zip` - ZIP archive (deflate compression)

## Advanced Features

### Symlink Handling

Flux properly handles symbolic links on Unix systems:
- **Default behavior**: Preserves symlinks as-is in the archive
- **`--follow-symlinks`**: Follows symlinks and archives the target files instead

### Metadata Preservation

On Unix systems, Flux preserves:
- File permissions (mode)
- Modification timestamps
- Symbolic links
- Directory structure

### Parallel Processing

Flux automatically uses multiple CPU cores when beneficial:
- Parallel file scanning during packing
- Concurrent compression for suitable algorithms
- Automatic thread count selection based on system resources

## Performance

Flux is designed for speed and efficiency:
- **Fast defaults**: Uses zstd by default for optimal speed/compression balance
- **Memory efficient**: Streams large files instead of loading them into memory
- **Parallel processing**: Utilizes all available CPU cores when beneficial
- **Smart strategy**: Avoids recompressing already-compressed files

## Roadmap to v2.0

### v1.0.0 (Current Focus)
- [x] Core archiving functionality (tar, zip)
- [x] Multiple compression algorithms (zstd, gzip, xz, brotli)
- [x] Smart compression strategy
- [x] CLI with all essential features
- [x] Cross-platform support
- [x] Comprehensive test suite
- [x] CI/CD pipeline

### v1.1.0 - Extended Format Support
- [ ] 7z archive support (read-only)
- [ ] RAR archive support (read-only)
- [ ] LZ4 compression algorithm
- [ ] Improved Windows compatibility

### v1.2.0 - Cloud Integration
- [ ] Direct pack/extract to S3
- [ ] Google Cloud Storage support
- [ ] Azure Blob Storage support
- [ ] Incremental backup support

### v2.0.0 - Next Generation
- [ ] Plugin system for custom formats
- [ ] GUI application (using Tauri)
- [ ] Shell completions
- [ ] Internationalization

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## License

This project is licensed under the MIT License - see the LICENSE file for details.