#!/bin/bash
# Script to generate platform-specific icons from the source PNG

set -e

# Check if ImageMagick is installed
if ! command -v convert &> /dev/null; then
    echo "Error: ImageMagick is required to generate icons."
    echo "Please install it:"
    echo "  - macOS: brew install imagemagick"
    echo "  - Ubuntu/Debian: sudo apt-get install imagemagick"
    echo "  - Fedora: sudo dnf install ImageMagick"
    exit 1
fi

# Source and destination paths
SOURCE_PNG="flux-gui/assets/icon.png"
DEST_DIR="flux-gui/assets"

# Check if source PNG exists
if [ ! -f "$SOURCE_PNG" ]; then
    echo "Error: Source icon not found at $SOURCE_PNG"
    exit 1
fi

echo "Generating icons from $SOURCE_PNG..."

# Generate ICO for Windows (multiple sizes)
echo "Generating Windows ICO..."
convert "$SOURCE_PNG" -resize 16x16 -depth 8 "$DEST_DIR/icon-16.png"
convert "$SOURCE_PNG" -resize 32x32 -depth 8 "$DEST_DIR/icon-32.png"
convert "$SOURCE_PNG" -resize 48x48 -depth 8 "$DEST_DIR/icon-48.png"
convert "$SOURCE_PNG" -resize 64x64 -depth 8 "$DEST_DIR/icon-64.png"
convert "$SOURCE_PNG" -resize 128x128 -depth 8 "$DEST_DIR/icon-128.png"
convert "$SOURCE_PNG" -resize 256x256 -depth 8 "$DEST_DIR/icon-256.png"

# Combine into ICO
convert "$DEST_DIR/icon-16.png" "$DEST_DIR/icon-32.png" "$DEST_DIR/icon-48.png" \
        "$DEST_DIR/icon-64.png" "$DEST_DIR/icon-128.png" "$DEST_DIR/icon-256.png" \
        "$DEST_DIR/icon.ico"

# Clean up temporary PNGs
rm -f "$DEST_DIR/icon-16.png" "$DEST_DIR/icon-32.png" "$DEST_DIR/icon-48.png" \
      "$DEST_DIR/icon-64.png" "$DEST_DIR/icon-128.png" "$DEST_DIR/icon-256.png"

# Generate ICNS for macOS
echo "Generating macOS ICNS..."
# Create iconset directory
ICONSET_DIR="$DEST_DIR/icon.iconset"
mkdir -p "$ICONSET_DIR"

# Generate required sizes for macOS
convert "$SOURCE_PNG" -resize 16x16 "$ICONSET_DIR/icon_16x16.png"
convert "$SOURCE_PNG" -resize 32x32 "$ICONSET_DIR/icon_16x16@2x.png"
convert "$SOURCE_PNG" -resize 32x32 "$ICONSET_DIR/icon_32x32.png"
convert "$SOURCE_PNG" -resize 64x64 "$ICONSET_DIR/icon_32x32@2x.png"
convert "$SOURCE_PNG" -resize 128x128 "$ICONSET_DIR/icon_128x128.png"
convert "$SOURCE_PNG" -resize 256x256 "$ICONSET_DIR/icon_128x128@2x.png"
convert "$SOURCE_PNG" -resize 256x256 "$ICONSET_DIR/icon_256x256.png"
convert "$SOURCE_PNG" -resize 512x512 "$ICONSET_DIR/icon_256x256@2x.png"
convert "$SOURCE_PNG" -resize 512x512 "$ICONSET_DIR/icon_512x512.png"
convert "$SOURCE_PNG" -resize 1024x1024 "$ICONSET_DIR/icon_512x512@2x.png"

# Convert to ICNS (macOS only)
if [[ "$OSTYPE" == "darwin"* ]]; then
    iconutil -c icns "$ICONSET_DIR" -o "$DEST_DIR/icon.icns"
else
    echo "Note: ICNS generation requires macOS. Skipping ICNS creation."
    echo "You can generate it on a Mac using: iconutil -c icns $ICONSET_DIR"
fi

# Clean up iconset directory
rm -rf "$ICONSET_DIR"

echo "Icon generation complete!"
echo "Generated files:"
[ -f "$DEST_DIR/icon.ico" ] && echo "  - $DEST_DIR/icon.ico (Windows)"
[ -f "$DEST_DIR/icon.icns" ] && echo "  - $DEST_DIR/icon.icns (macOS)"
echo "  - $SOURCE_PNG (Linux/Original)"