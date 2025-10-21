#!/bin/bash
# Script to create a large test archive and open it with flux-gui

echo "Creating a large test archive with 50,000 files..."

# Create temporary directory
TEMP_DIR=$(mktemp -d)
echo "Working in: $TEMP_DIR"

# Create many files in subdirectories
for i in {1..100}; do
    DIR="$TEMP_DIR/dir_$(printf '%03d' $i)"
    mkdir -p "$DIR"
    
    for j in {1..500}; do
        FILE="$DIR/file_$(printf '%04d' $j).txt"
        echo "This is file $i-$j with some content to make it realistic" > "$FILE"
    done
    
    if [ $((i % 10)) -eq 0 ]; then
        echo "Created $((i * 500)) files..."
    fi
done

echo "Created 50,000 files. Creating archive..."

# Create the archive
cd "$TEMP_DIR"
tar czf large_test_archive.tar.gz dir_*

ARCHIVE_SIZE=$(du -h large_test_archive.tar.gz | cut -f1)
echo "Archive created: large_test_archive.tar.gz ($ARCHIVE_SIZE)"

# Move to workspace
mv large_test_archive.tar.gz /workspace/

# Clean up
cd /
rm -rf "$TEMP_DIR"

echo "Done! Archive is at /workspace/large_test_archive.tar.gz"
echo ""
echo "To test the GUI performance:"
echo "1. Run: cargo run -p flux-gui"
echo "2. Click 'Browse Archive' and select large_test_archive.tar.gz"
echo "3. The browser should handle 50k files smoothly with virtualized scrolling"