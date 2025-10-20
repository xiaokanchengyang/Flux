#!/bin/bash
# Test release build locally before pushing tags

set -e

echo "🔧 Testing release build process..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
echo -e "${YELLOW}Building Flux v${VERSION}...${NC}"

# Test native build
echo -e "${YELLOW}Testing native build...${NC}"
cargo build --release --package flux-cli

# Get the binary name based on OS
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    BINARY="flux.exe"
else
    BINARY="flux"
fi

# Test the binary
echo -e "${YELLOW}Testing binary...${NC}"
./target/release/$BINARY --version
./target/release/$BINARY --help

# Create test archive
echo -e "${YELLOW}Creating test archive...${NC}"
mkdir -p test-release-data
echo "Hello, Flux!" > test-release-data/hello.txt
echo "Test content" > test-release-data/test.md
./target/release/$BINARY pack test-release-data -o test.tar.zst --progress

# Test extraction
echo -e "${YELLOW}Testing extraction...${NC}"
mkdir -p test-extract
./target/release/$BINARY extract test.tar.zst -o test-extract --progress

# Verify
if diff -r test-release-data test-extract/test-release-data; then
    echo -e "${GREEN}✓ Archive/extract test passed!${NC}"
else
    echo "❌ Archive/extract test failed!"
    exit 1
fi

# Test inspect
echo -e "${YELLOW}Testing inspect...${NC}"
./target/release/$BINARY inspect test.tar.zst

# Cleanup
rm -rf test-release-data test-extract test.tar.zst

echo -e "${GREEN}✓ All release tests passed!${NC}"
echo ""
echo "To create a release:"
echo "  1. Ensure all changes are committed"
echo "  2. Run: git tag -a v${VERSION} -m \"Release version ${VERSION}\""
echo "  3. Run: git push origin v${VERSION}"
echo ""
echo "The GitHub Actions workflow will automatically:"
echo "  - Build binaries for all platforms"
echo "  - Create a GitHub release"
echo "  - Upload the binaries"
echo "  - Publish to crates.io"