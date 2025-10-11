#!/bin/bash
# Test script to demonstrate the date inference feature

set -e

echo "=== Date Inference Feature Test ==="
echo

# Create test directories
TEST_DIR="/tmp/photo_inference_test"
SOURCE_DIR="$TEST_DIR/source"
TARGET_DIR="$TEST_DIR/target"

# Clean up any existing test directory
rm -rf "$TEST_DIR"

# Create source directory structure
mkdir -p "$SOURCE_DIR/vacation_2023"
mkdir -p "$SOURCE_DIR/old_scans"

echo "Creating test files..."

# Create a recent photo (will have reliable date from file timestamp)
echo "Recent photo content" > "$SOURCE_DIR/vacation_2023/IMG_001.jpg"

# Create an old document in same directory (will get date inferred)
echo "Old document content" > "$SOURCE_DIR/vacation_2023/notes.txt"
# Set timestamp to 15 years ago
touch -t $(date -d '15 years ago' +%Y%m%d%H%M) "$SOURCE_DIR/vacation_2023/notes.txt"

# Create isolated old files (will go to undated directory)
echo "Old scan content" > "$SOURCE_DIR/old_scans/scan1.jpg"
echo "Another old scan" > "$SOURCE_DIR/old_scans/scan2.txt"
# Set timestamps to 15 years ago
touch -t $(date -d '15 years ago' +%Y%m%d%H%M) "$SOURCE_DIR/old_scans/scan1.jpg"
touch -t $(date -d '15 years ago' +%Y%m%d%H%M) "$SOURCE_DIR/old_scans/scan2.txt"

echo "Test files created:"
ls -la "$SOURCE_DIR/vacation_2023/"
ls -la "$SOURCE_DIR/old_scans/"
echo

# Run the organizer with text logging
echo "Running photo organizer..."
cargo run --quiet -- \
    --source "$SOURCE_DIR" \
    --target "$TARGET_DIR" \
    --extensions jpg,txt \
    --preserve-structure \
    --text-log-file "$TEST_DIR/test.log" \
    --force-reprocess

echo
echo "=== Results ==="
echo

echo "Target directory structure:"
find "$TARGET_DIR" -type f | sort
echo

echo "=== Log Output ==="
cat "$TEST_DIR/test.log"
echo

echo "=== Summary ==="
echo "✓ IMG_001.jpg should be organized by date (reliable recent timestamp)"
echo "✓ notes.txt should be organized by the SAME date (inferred from IMG_001.jpg)"
echo "✓ scan1.jpg and scan2.txt should be in undated/old_scans/ (no reliable peers)"
echo

# Cleanup
echo "Cleaning up test directory..."
rm -rf "$TEST_DIR"
echo "Done!"