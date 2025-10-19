# Flux

A high-performance, cross-platform file archiver and compressor written in Rust.

Flux is a modern replacement for traditional archiving tools, offering intelligent compression strategies, automatic algorithm selection, concurrent processing, and a user-friendly CLI experience.

## Features

- **Cross-platform**: Works on Linux, macOS, and Windows
- **Smart compression**: Automatically selects optimal compression algorithms based on file type, size, and system resources
- **Multiple formats**: Supports common archive formats (tar, zip) and compression algorithms (zstd, xz, gzip, brotli)
- **Concurrent processing**: Leverages multiple CPU cores for faster compression/decompression
- **Metadata preservation**: Retains file permissions, timestamps, and other metadata
- **Library and CLI**: Use as a standalone tool or integrate into your Rust applications

## Installation

### From source

```bash
git clone https://github.com/your-username/flux.git
cd flux
cargo build --release
```

The binary will be available at `target/release/flux`.

### Using cargo

```bash
cargo install --path flux/flux-cli
```

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

## Smart Compression Strategy

Flux includes intelligent compression strategies that automatically select the best algorithm based on:

- File type and extension
- File size
- Available CPU cores
- System memory
- User preferences

For example:
- Text files → zstd or brotli for high compression
- Already compressed files (.jpg, .mp4) → store without recompression
- Large files → xz with limited threads to prevent memory issues
- Many small files → tar first, then compress

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

## Roadmap

### Milestone 0 - Skeleton ✓
- [x] Initialize workspace structure
- [x] Basic CLI with subcommands
- [x] Simple tar pack/extract functionality
- [x] README and project setup

### Milestone 1 - Basic Archive Support ✓
- [x] Tar archive support with multiple compression formats
- [x] Gzip compression
- [x] Progress bars (via tracing logs)
- [x] Metadata preservation (permissions, timestamps, symlinks)

### Milestone 2 - Smart Compression ✓
- [x] Compression strategy engine
- [x] Configuration file support
- [x] Concurrent file processing (via rayon)
- [x] Multiple compression algorithms (zstd, xz, brotli, gzip)
- [x] Intelligent algorithm selection based on file type and size
- [x] Memory-efficient processing for large files

### Milestone 3 - Extended Formats
- [ ] Additional archive formats
- [ ] Windows compatibility improvements
- [ ] Performance optimizations
- [ ] Comprehensive testing

### Milestone 4 - Polish
- [ ] Documentation
- [ ] Benchmarks
- [ ] CI/CD pipeline
- [ ] Release builds

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## License

This project is licensed under the MIT License - see the LICENSE file for details.