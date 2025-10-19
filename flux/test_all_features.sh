#!/bin/bash
set -e

echo "=== Testing Flux Archive Tool ==="
echo

# Test 1: Smart compression with different file types
echo "1. Testing smart compression strategy..."
./target/release/flux pack test_data -o test_smart.tar.zst --smart -v
echo

# Test 2: Inspect command with table output
echo "2. Testing inspect command (table format)..."
./target/release/flux inspect test_smart.tar.zst
echo

# Test 3: Inspect command with JSON output
echo "3. Testing inspect command (JSON format)..."
./target/release/flux inspect test_smart.tar.zst --json | head -20
echo

# Test 4: Extract with default options
echo "4. Testing extract with default options..."
mkdir -p extract_default
./target/release/flux extract test_smart.tar.zst -o extract_default
ls -la extract_default/test_data/
echo

# Test 5: Extract with strip-components
echo "5. Testing extract with strip-components..."
mkdir -p extract_strip
./target/release/flux extract test_smart.tar.zst -o extract_strip --strip-components 1
ls -la extract_strip/
echo

# Test 6: Extract with overwrite option
echo "6. Testing extract with overwrite..."
./target/release/flux extract test_smart.tar.zst -o extract_default --overwrite
echo

# Test 7: Different compression algorithms
echo "7. Testing different compression algorithms..."
echo "   - Store (no compression)..."
./target/release/flux pack test_data/file1.txt -o test_store.tar --algo store
echo "   - Gzip compression..."
./target/release/flux pack test_data/file1.txt -o test_gzip.tar.gz --algo gzip
echo "   - XZ compression..."
./target/release/flux pack test_data/file1.txt -o test_xz.tar.xz --algo xz
echo "   - Brotli compression..."
./target/release/flux pack test_data/file1.txt -o test_br.tar.br --algo brotli
echo

# Test 8: Inspect different archive formats
echo "8. Testing inspect on different formats..."
for archive in test_*.tar*; do
    echo "   Inspecting $archive:"
    ./target/release/flux inspect "$archive" | head -3
done
echo

# Test 9: Error handling with exit codes
echo "9. Testing error handling and exit codes..."
echo "   - Invalid file (should return exit code 3)..."
./target/release/flux pack /nonexistent/file -o test_error.tar 2>&1 || echo "Exit code: $?"
echo "   - Invalid archive format (should return exit code 3)..."
./target/release/flux extract test_data/file1.txt -o extract_error 2>&1 || echo "Exit code: $?"
echo

# Cleanup
echo "Cleaning up test files..."
rm -rf test_*.tar* extract_*
echo

echo "=== All tests completed successfully! ==="