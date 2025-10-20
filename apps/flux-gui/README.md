# Flux GUI

A modern graphical interface for the Flux file archiver.

## Features

### MVP Implementation (v1.4.0 Alpha)

✅ **Basic Window and UI**
- Clean, modern interface using egui/eframe
- Dark theme with custom styling

✅ **Background Task Framework**
- Separate worker thread for archive operations
- Channel-based communication between UI and worker
- Progress reporting with file-level granularity

✅ **Drag & Drop Support**
- Drop files/folders to automatically detect mode
- Single archive file → Extract mode
- Multiple files or folders → Pack mode

✅ **Core Functionality**
- **Pack Mode**: Create archives from selected files
  - Support for tar.gz, tar.zst, tar.xz, and zip formats
  - Smart compression strategy
- **Extract Mode**: Extract archive contents
  - Utilizes the new Extractor API for fine-grained progress
  - Support for all major archive formats

## Architecture

The GUI is structured with clear separation of concerns:

1. **Main Thread (UI)**
   - Handles all UI rendering and user interactions
   - Sends commands to worker thread
   - Receives progress updates and results

2. **Worker Thread**
   - Executes long-running archive operations
   - Reports progress back to UI thread
   - Prevents UI freezing during operations

3. **Communication**
   - `TaskCommand`: UI → Worker commands
   - `ToUi`: Worker → UI messages (progress, results)

## Usage

```bash
cargo run -p flux-gui
```

## Next Steps

- Add more advanced options (compression levels, thread count)
- Implement file preview
- Add operation history
- Support for incremental packing
- Better error handling and recovery