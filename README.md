<p align="center">
  <img src="assets/logo.png" alt="Photo Organizer logo" width="180">
</p>

# Photo Organizer

A powerful Rust-based command-line tool for organizing large collections of photos and raw image files by date, using EXIF data when available. The tool creates a structured `yy/mm/dd` directory hierarchy and handles duplicates intelligently.

## Features

- **EXIF Date Extraction**: Reads actual photo capture dates from EXIF metadata
- **Fallback to File Timestamps**: Uses file modification time when EXIF data is unavailable
- **Intelligent Deduplication**: Compares file content (not just names) to detect true duplicates
- **Smart Conflict Resolution**: Generates unique filenames for files with same name but different content
- **Resumable Operations**: Can be interrupted and resumed without losing progress
- **Progress Tracking**: Real-time progress display with file counts and ETA
- **Context Preservation**: Creates `.tag` files for directories with interesting names
- **Comprehensive Logging**: CSV log of all operations for audit trails
- **Dry Run Mode**: Preview what would be done without making changes
- **Source Structure Preservation**: For files without reliable dates, preserve original directory structure
- **Text Logging**: Simple text output highlighting problematic files
- **Log-Only Mode**: Preview mode that only logs what would happen without copying files
- **Date Reliability Detection**: Identifies files with suspicious timestamps (too old/future)
- **Multi-format Support**: Handles JPG, RAW, CR2, CR3, NEF, ARW, DNG, TIFF, and more

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)

### Build from Source

```bash
git clone <repository-url>
cd photo-organizer
cargo build --release
```

The compiled binary will be available at `target/release/photo_organizer`.

## Usage

### Basic Usage

```bash
photo_organizer --source /path/to/source/photos --target /path/to/organized/photos
```

### Common Options

```bash
# Dry run to see what would happen
photo_organizer --source ./photos --target ./organized --dry-run

# Force reprocessing of all files (ignore previous state)
photo_organizer --source ./photos --target ./organized --force-reprocess

# Custom log file location
photo_organizer --source ./photos --target ./organized --log-file ./my-log.csv

# Include additional file extensions
photo_organizer --source ./photos --target ./organized --extensions jpg jpeg raw cr2 cr3 orf

# Preserve source directory structure for files with unreliable dates
photo_organizer --source ./photos --target ./organized --preserve-structure

# Generate simple text log highlighting problem files
photo_organizer --source ./photos --target ./organized --text-log-file mapping.txt

# Log-only mode - see what would happen without copying files
photo_organizer --source ./photos --target ./organized --log-only --text-log-file preview.txt
```

### Full Command Line Options

```
A tool to organize photos by date using EXIF data

Usage: photo_organizer [OPTIONS] --source <SOURCE> --target <TARGET>

Options:
  -s, --source <SOURCE>          Source directory containing photos to organize
  -t, --target <TARGET>          Target directory for organized photos
      --log-file <LOG_FILE>      CSV log file path [default: photo_organizer.csv]
      --state-file <STATE_FILE>  State file for resumable operations [default: photo_organizer_state.json]
      --dry-run                  Dry run mode - show what would be done without making changes
      --force-reprocess                Force re-processing of all files
      --threads <THREADS>              Number of worker threads [default: 4]
      --extensions <EXTENSIONS>        Supported image extensions [default: jpg jpeg raw cr2 cr3 nef arw dng tiff tif]
      --preserve-structure             Preserve source directory structure for files without determinable dates
      --text-log-file <TEXT_LOG_FILE>  Text log file path for simple source->target mapping
      --log-only                       Log-only mode - don't copy files, just log what would happen
  -h, --help                           Print help
  -V, --version                        Print version
```

## How It Works

### Directory Structure

The tool organizes photos into a year/month/day hierarchy:

```
target_directory/
├── 23/
│   ├── 01/
│   │   ├── 15/
│   │   │   ├── IMG_001.jpg
│   │   │   ├── IMG_002.cr2
│   │   │   └── vacation_photos.tag
│   │   └── 16/
│   └── 12/
└── 24/
```

### EXIF Date Priority

The tool tries to extract dates in this order:

1. **DateTimeOriginal** (when the photo was taken)
2. **DateTime** (when the file was created/modified)
3. **DateTimeDigitized** (when the photo was digitized)
4. **File modification time** (fallback)

### Deduplication Strategy

1. **Content-based comparison**: Files are hashed (SHA-256) to detect true duplicates
2. **Filename conflict resolution**: If different files have the same target name, unique suffixes are added (e.g., `photo_001.jpg`, `photo_002.jpg`)
3. **Existing file preservation**: If a file already exists at the target with identical content, it's skipped

### Context Preservation

The tool detects "interesting" directory names (like `vacation_photos`, `wedding_2023`, etc.) and creates `.tag` files in the target directories containing the original directory name. This helps preserve the context of where photos came from.

### Handling Unreliable Dates

The tool automatically detects files with unreliable timestamps (too old or in the future) and provides special handling:

1. **Date Reliability Check**: Files modified more than 10 years ago or in the future are flagged as unreliable
2. **Source Structure Preservation**: With `--preserve-structure`, unreliable files maintain their original directory structure under an `undated/` folder
3. **Visual Indicators**: Text logs use ⚠️ to highlight files with date issues vs ✓ for reliable dates

**Example Output Structure:**
```
organized_photos/
├── 25/01/15/           # Reliable dates organized by yy/mm/dd
│   ├── photo1.jpg
│   └── photo2.jpg  
└── undated/            # Unreliable dates preserve source structure
    └── old_photos/
        ├── ancient.jpg
        └── old_photos.tag
```

## Output Files

### CSV Log File

Every operation is logged to a CSV file with the following columns:

- `source_path`: Original file location
- `target_path`: Where the file was copied
- `timestamp`: Date/time the photo was taken (or file modified)
- `directory_context`: Original directory name
- `file_size`: Size in bytes
- `file_hash`: SHA-256 hash of file content
- `exif_date_available`: Whether EXIF date was found
- `operation`: What happened (PROCESSED, ERROR, etc.)

### State File

A JSON file tracking processing state for resumable operations:

```json
{
  "processed_files": ["hash1", "hash2", ...],
  "last_scan_path": "/path/to/source",
  "total_files_found": 1500,
  "processed_count": 1450,
  "error_count": 2,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Tag Files

For directories with interesting names, `.tag` files are created:

```
vacation_photos.tag
```

Content: `Photos from directory: vacation_photos`

### Text Log File

When using `--text-log-file`, a simple text file is created showing source-to-target mappings:

```
✓  recent_photo.jpg -> organized_photos/25/10/11/recent_photo.jpg [FILE_TIME] (PROCESSED)
⚠️  old_photo.jpg -> organized_photos/undated/old_photos/old_photo.jpg [UNRELIABLE_DATE] (PRESERVED_STRUCTURE)

=== SUMMARY ===
Total files: 2
Processed: 2
Errors: 0
Files with unreliable dates: 1

Legend:
✓  = Successfully processed with reliable date
⚠️  = Processed but date may be unreliable (preserved source structure)
EXIF = Date from image metadata
FILE_TIME = Date from file timestamp
UNRELIABLE_DATE = File timestamp seems invalid (too old/future)
```

## Error Handling

- **Corrupted EXIF data**: Falls back to file modification time
- **File access errors**: Logged but doesn't stop processing
- **Duplicate filenames**: Automatically generates unique names
- **Interrupted operations**: Can be resumed using state file
- **Network drives**: Handles slow I/O gracefully

## Performance

- **Parallel processing**: Configurable thread count (default: 4)
- **Memory efficient**: Processes files one at a time
- **Progress tracking**: Real-time updates on processing speed
- **Hash caching**: Avoids recomputing hashes for unchanged files

## Supported Formats

### Image Formats
- JPEG (`.jpg`, `.jpeg`)
- TIFF (`.tiff`, `.tif`)

### RAW Formats
- Canon (`.cr2`, `.cr3`)
- Nikon (`.nef`)
- Sony (`.arw`)
- Generic RAW (`.raw`)
- Adobe DNG (`.dng`)

*Note: EXIF support varies by format. Some RAW formats may fall back to file timestamps.*

## Safety Features

- **Dry run mode**: Test operations without making changes
- **Content verification**: Uses checksums to ensure data integrity
- **Non-destructive**: Original files are copied, never moved or deleted
- **Atomic operations**: Individual file operations are atomic
- **Comprehensive logging**: Full audit trail of all operations

## Examples

### Organizing a Large Photo Collection

```bash
# First, do a dry run to see what will happen
photo_organizer --source /Volumes/ExternalDrive/Photos --target ./Organized --dry-run

# If satisfied, run for real
photo_organizer --source /Volumes/ExternalDrive/Photos --target ./Organized
```

### Resuming After Interruption

```bash
# If the process was interrupted, simply run the same command again
# It will automatically resume from where it left off
photo_organizer --source /Volumes/ExternalDrive/Photos --target ./Organized
```

### Processing Only Specific Formats

```bash
# Only process Canon RAW files
photo_organizer --source ./photos --target ./organized --extensions cr2 cr3

# Preview organization without copying files
photo_organizer --source ./photos --target ./organized --log-only --text-log-file preview.txt
```

## Troubleshooting

### Common Issues

1. **"Permission denied" errors**
   - Ensure you have read access to source directory
   - Ensure you have write access to target directory

2. **"No EXIF data found" warnings**
   - Normal for some file formats
   - Tool will use file modification time instead

3. **Slow processing on network drives**
   - Consider copying files locally first
   - Or use `--threads 1` to reduce concurrent I/O

4. **Out of disk space**
   - Check target directory has enough space
   - Tool calculates total size before starting

### Debug Information

Enable debug logging:

```bash
RUST_LOG=debug photo_organizer --source ./photos --target ./organized
```

## Development

### Running Tests

```bash
cargo test
```

### Code Structure

- `src/main.rs` - Entry point and CLI setup
- `src/config.rs` - Configuration and argument parsing
- `src/scanner.rs` - File discovery and metadata extraction
- `src/exif_extractor.rs` - EXIF date parsing
- `src/deduplicator.rs` - Duplicate detection and resolution
- `src/organizer.rs` - Main orchestration logic
- `src/logger.rs` - CSV logging functionality
- `src/progress.rs` - Progress tracking and display
- `src/state.rs` - State persistence for resumable operations
- `src/file_info.rs` - Core data structures

## License

[Specify your license here]

## Contributing

[Add contribution guidelines if needed]

---

**Note**: This tool is designed to be safe and non-destructive, but always test with a small subset of files first and keep backups of important photos.