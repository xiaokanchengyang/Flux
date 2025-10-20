# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Interactive extraction mode** (`--interactive` flag)
  - Prompts for each file conflict during extraction
  - Options: Overwrite, Skip, Rename, Overwrite All, Skip All, Abort
  - Provides fine-grained control over extraction process
- **New Extractor trait architecture**
  - `entries()` method for listing archive contents without extraction
  - `extract_entry()` method for extracting individual files
  - Enables streaming and interactive extraction workflows
  - Foundation for future GUI real-time feedback
- **Partial failure exit code (4)**
  - Better error reporting when some files fail to extract
  - Distinguishes between complete and partial failures

### Changed
- **Major refactoring of extraction API**
  - Moved from monolithic `extract()` to granular `entries()` and `extract_entry()`
  - All archive formats (tar, zip, 7z) updated to new API
  - Better separation of concerns and error handling
- **7z format limitations documented**
  - Interactive extraction falls back to standard mode for 7z
  - Clear error messages for unsupported operations

### Technical Improvements
- Implemented `ConflictHandler` trait for extensible conflict resolution
- Added `ExtractEntryOptions` for fine-grained extraction control
- Better progress reporting during interactive extraction
- Improved error handling with specific error types

## [1.7.0] - 2024-10-20

### Added
- Comprehensive performance benchmark suite
  - Compression benchmarks for different algorithms and levels
  - Extraction performance tests
  - Comparison with system tar command
- GitHub Actions workflow for automated benchmarking
- Support for benchmark result reporting in CI/CD

### Changed
- Updated minimum Rust version to 1.90.0

## [1.6.0] - 2024-10-20

### Added
- Incremental backup support with manifest-based change tracking
  - `--incremental` flag for pack command
  - Automatic manifest generation for future incremental backups
  - Blake3-based file hashing for change detection
- `pack_multiple_files` function in tar module for incremental archives

### Fixed
- Improved error handling for archive operations

## [1.5.0] - 2024-10-20

### Added
- Enhanced GUI features:
  - Cancel operation support
  - Real-time log window
  - Toggle for showing/hiding logs
  - Improved error messages

### Changed
- GUI now shows more detailed progress information
- Better worker thread management in GUI

## [1.4.0] - 2024-10-20

### Added
- **flux-gui**: Brand new graphical user interface
  - Built with egui/eframe for native performance
  - Drag-and-drop file support
  - Auto-detection of pack/extract mode
  - Visual compression settings
  - Real-time progress indication
  - Cross-platform native application

### Changed
- Project now includes three components: library, CLI, and GUI

## [1.3.0] - 2024-10-20

### Added
- Interactive mode for extract command (`-i, --interactive`)
  - Prompts for file conflict resolution
  - Options: Overwrite, Skip, Rename, All, None, Quit
- Partial failure exit code (4) for operations with some failures
- New error types: `PartialFailure`, `FileExists`, `UnsupportedOperation`

### Dependencies
- Added `dialoguer` for interactive prompts

## [1.2.0] - 2024-10-20

### Added
- Size-based compression rules in configuration
  - Automatic algorithm selection based on file size thresholds
  - Configurable via `strategy.size_rules` in config.toml
- Automatic thread adjustment for XZ compression (forced to single thread)
- Warning messages when XZ is selected with multiple threads

### Changed
- Enhanced smart compression strategy with file size considerations
- Improved thread management for different algorithms

## [1.1.0] - 2024-10-20

### Added
- 7z archive format support (extraction only)
  - Full support for extracting 7z archives
  - Metadata preservation where possible
  - Note: 7z creation not yet supported due to library limitations
- `sevenz-rust` dependency for 7z support

### Changed
- Updated archive format detection to include 7z files

## [1.0.0] - 2024-10-20

### Added
- Initial stable release
- Core features:
  - TAR archive support with multiple compression algorithms
  - ZIP archive support with metadata preservation
  - Smart compression strategy based on file analysis
  - Configurable compression rules
  - Progress indication
  - Cross-platform support (Linux, macOS, Windows)
- Compression algorithms:
  - Zstandard (zstd) - default
  - XZ/LZMA2
  - Gzip
  - Brotli
  - Store (no compression)
- CLI commands:
  - `pack` - Create archives with intelligent compression
  - `extract` - Extract archives with flexible options
  - `inspect` - View archive contents without extraction
  - `config` - Manage configuration
- Advanced features:
  - Parallel processing support
  - Metadata preservation (permissions, timestamps, symlinks)
  - Entropy-based compression detection
  - Custom compression rules via configuration
  - Force compression option for already-compressed files

[1.7.0]: https://github.com/your-username/flux/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/your-username/flux/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/your-username/flux/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/your-username/flux/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/your-username/flux/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/your-username/flux/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/your-username/flux/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/your-username/flux/releases/tag/v1.0.0