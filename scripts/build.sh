#!/bin/bash
# Build script for flux

set -e

echo "Building flux..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust first."
    exit 1
fi

# Build in release mode
echo "Building release version..."
cargo build --release

echo "Build complete!"
echo "Binary available at: target/release/flux"

# Optional: Run tests
echo ""
echo "Running tests..."
cargo test

echo ""
echo "All tests passed!"
echo ""
echo "To install flux system-wide, run:"
echo "  cargo install --path flux-cli"