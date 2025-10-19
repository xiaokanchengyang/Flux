#!/bin/bash
# Basic usage examples for flux

echo "=== Flux Basic Usage Examples ==="
echo ""

# Build the project first
echo "Building flux..."
cargo build --release
FLUX="cargo run --release -p flux-cli --"

# Create some test data
echo "Creating test data..."
mkdir -p test_data
echo "Hello from Flux!" > test_data/hello.txt
echo "This is a test file" > test_data/test.txt
mkdir -p test_data/subdir
echo "Nested file content" > test_data/subdir/nested.txt

echo ""
echo "1. Pack a directory into a tar archive:"
echo "   $ flux pack test_data -o archive.tar"
$FLUX pack test_data -o archive.tar

echo ""
echo "2. Extract an archive:"
echo "   $ flux extract archive.tar -o extracted_data"
$FLUX extract archive.tar -o extracted_data

echo ""
echo "3. Pack with verbose output:"
echo "   $ flux pack test_data -o verbose.tar --verbose"
$FLUX pack test_data -o verbose.tar --verbose

echo ""
echo "4. Pack a single file:"
echo "   $ flux pack test_data/hello.txt -o single_file.tar"
$FLUX pack test_data/hello.txt -o single_file.tar

echo ""
echo "5. Extract to specific directory:"
echo "   $ flux extract archive.tar -o /tmp/flux_test"
$FLUX extract archive.tar -o /tmp/flux_test

echo ""
echo "Cleaning up..."
rm -rf test_data extracted_data archive.tar verbose.tar single_file.tar /tmp/flux_test

echo ""
echo "Examples completed!"