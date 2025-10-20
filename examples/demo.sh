#!/bin/bash
# Flux Demo Script
# This script demonstrates Flux's key features

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Flux Demo - Intelligent File Compression ===${NC}"
echo

# Create diverse test data
echo -e "${YELLOW}Creating test data with various file types...${NC}"
mkdir -p demo-data/{docs,code,images,logs}

# Text documents
echo "# Flux Documentation" > demo-data/docs/README.md
cat << 'EOF' > demo-data/docs/guide.md
# Flux User Guide

Flux is an intelligent file archiver that automatically selects the best
compression algorithm based on file type and content. This document contains
various sections to demonstrate text compression.

## Features
- Smart compression strategy
- Cross-platform support
- Multiple algorithms
- Parallel processing

## Usage
See the examples below for common operations.
EOF

# Source code
cat << 'EOF' > demo-data/code/app.py
#!/usr/bin/env python3
"""Sample Python application for compression demo."""

import json
import logging
from datetime import datetime

class DataProcessor:
    def __init__(self):
        self.logger = logging.getLogger(__name__)
        self.data = []
    
    def process(self, items):
        """Process items and return results."""
        results = []
        for item in items:
            processed = {
                'id': item.get('id'),
                'timestamp': datetime.now().isoformat(),
                'value': item.get('value', 0) * 2
            }
            results.append(processed)
        return results

if __name__ == '__main__':
    processor = DataProcessor()
    sample_data = [{'id': i, 'value': i * 10} for i in range(100)]
    results = processor.process(sample_data)
    print(json.dumps(results, indent=2))
EOF

# JSON data
cat << 'EOF' > demo-data/code/data.json
{
  "name": "flux-demo",
  "version": "1.0.0",
  "description": "Demonstration of Flux compression",
  "dependencies": {
    "compression": "latest",
    "archive": "latest"
  },
  "config": {
    "threads": 4,
    "level": 6,
    "algorithm": "auto"
  }
}
EOF

# Large log file
echo -e "${YELLOW}Generating log file...${NC}"
for i in {1..1000}; do
    echo "[$(date -Iseconds)] INFO: Processing item $i - Status: OK - Duration: ${RANDOM}ms" >> demo-data/logs/app.log
done

# Binary data (simulated image)
dd if=/dev/urandom of=demo-data/images/photo.jpg bs=1K count=500 2>/dev/null
dd if=/dev/urandom of=demo-data/images/screenshot.png bs=1K count=200 2>/dev/null

# Show data structure
echo -e "${GREEN}Test data created:${NC}"
tree demo-data 2>/dev/null || find demo-data -type f | sort

echo
echo -e "${BLUE}=== Demonstration 1: Smart Compression ===${NC}"
echo -e "${YELLOW}Flux will automatically choose the best algorithm for each file type${NC}"
echo
echo "Command: flux pack demo-data -o smart-archive.tar.zst --progress"
flux pack demo-data -o smart-archive.tar.zst --progress

echo
echo -e "${BLUE}=== Demonstration 2: Inspect Archive ===${NC}"
echo "Command: flux inspect smart-archive.tar.zst"
flux inspect smart-archive.tar.zst

echo
echo -e "${BLUE}=== Demonstration 3: Different Algorithms ===${NC}"
echo -e "${YELLOW}Comparing different compression algorithms...${NC}"

# Zstandard (default)
echo
echo "1. Zstandard (balanced speed/ratio):"
time flux pack demo-data -o demo-zstd.tar.zst --algo zstd --level 6

# XZ (maximum compression)
echo
echo "2. XZ (maximum compression):"
time flux pack demo-data -o demo-xz.tar.xz --algo xz --level 6

# Gzip (compatibility)
echo
echo "3. Gzip (wide compatibility):"
time flux pack demo-data -o demo-gzip.tar.gz --algo gzip --level 6

echo
echo -e "${BLUE}=== Size Comparison ===${NC}"
ls -lh demo-*.tar.* smart-archive.tar.zst | awk '{print $9, $5}'

echo
echo -e "${BLUE}=== Demonstration 4: Extraction ===${NC}"
echo "Command: flux extract smart-archive.tar.zst -o extracted --progress"
flux extract smart-archive.tar.zst -o extracted --progress

echo
echo -e "${GREEN}✓ Demo completed successfully!${NC}"
echo
echo "Key takeaways:"
echo "- Flux automatically selected optimal compression for each file type"
echo "- Already-compressed files (images) were stored without recompression"
echo "- Text files received high compression ratios"
echo "- The process was fast due to parallel processing"

# Cleanup
echo
read -p "Press Enter to clean up demo files..."
rm -rf demo-data extracted demo-*.tar.* smart-archive.tar.zst
echo "Demo files cleaned up."