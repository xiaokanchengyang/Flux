#!/bin/bash
# Demo script for archive modification features

set -e

echo "====================================="
echo "Flux Archive Modification Demo"
echo "====================================="
echo ""

# Create demo directory
DEMO_DIR=$(mktemp -d)
cd "$DEMO_DIR"
echo "Working in: $DEMO_DIR"
echo ""

# Create initial files
echo "1. Creating initial files..."
for i in {1..5}; do
    echo "This is document $i" > "document_$i.txt"
done
echo "Created 5 documents"
echo ""

# Create initial archive
echo "2. Creating archive with initial files..."
tar czf documents.tar.gz document_*.txt
echo "Archive created: documents.tar.gz"
echo "Contents:"
tar tzf documents.tar.gz | head -10
echo ""

# Demonstrate adding files
echo "3. Adding new files to the archive..."
echo "This is a README" > README.md
echo "Important notes" > notes.txt
/workspace/target/release/flux add documents.tar.gz README.md notes.txt
echo ""
echo "Updated contents:"
tar tzf documents.tar.gz | head -10
echo ""

# Demonstrate removing files
echo "4. Removing specific files..."
/workspace/target/release/flux remove documents.tar.gz "document_2.txt" "document_4.txt"
echo ""
echo "After removal:"
tar tzf documents.tar.gz | head -10
echo ""

# Demonstrate pattern removal
echo "5. Creating archive with mixed file types..."
for i in {1..3}; do
    echo "Log entry $i" > "app_$i.log"
    echo "Config $i" > "config_$i.conf"
done
tar czf mixed.tar.gz *.txt *.log *.conf
echo ""
echo "Mixed archive contents:"
tar tzf mixed.tar.gz | sort
echo ""

echo "6. Removing all .log files using pattern..."
/workspace/target/release/flux remove mixed.tar.gz "*.log"
echo ""
echo "After removing *.log:"
tar tzf mixed.tar.gz | sort
echo ""

# Demonstrate ZIP modification
echo "7. Testing ZIP archive modification..."
zip -q archive.zip document_*.txt
echo "Initial ZIP contents:"
unzip -l archive.zip | grep -E "document_.*\.txt"
echo ""

echo "Adding file to ZIP..."
echo "ZIP specific content" > zip_only.txt
/workspace/target/release/flux add archive.zip zip_only.txt
echo ""
echo "Updated ZIP contents:"
unzip -l archive.zip | grep -E "\.txt"
echo ""

# Clean up
cd /
rm -rf "$DEMO_DIR"

echo "====================================="
echo "Demo completed successfully!"
echo "====================================="
echo ""
echo "Key features demonstrated:"
echo "✅ Add files to TAR.GZ archives"
echo "✅ Remove specific files from archives"
echo "✅ Remove files by pattern (e.g., *.log)"
echo "✅ Modify ZIP archives"
echo "✅ Preserve archive format and compression"
echo ""
echo "GUI features:"
echo "🖱️  Drag & drop files onto archive browser to add"
echo "🖱️  Right-click files to remove from context menu"
echo "🖱️  Add/Remove buttons in archive browser toolbar"