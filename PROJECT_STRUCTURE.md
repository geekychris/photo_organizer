# Photo Organizer - Project Structure

## 📁 Directory Structure

```
photo-organizer/                    # Main project root
├── src/                           # Rust source code
│   ├── main.rs                   # Main entry point
│   ├── config.rs                 # Configuration handling
│   ├── scanner.rs                # File scanning
│   ├── exif_extractor.rs         # EXIF data extraction
│   ├── file_info.rs              # File information structure
│   ├── organizer.rs              # Main organization logic
│   ├── deduplicator.rs           # Deduplication functionality
│   ├── date_inference.rs         # Date inference from directories
│   ├── logger.rs                 # CSV logging
│   ├── text_logger.rs            # Enhanced text logging with deduplication
│   ├── progress.rs               # Progress reporting
│   └── state.rs                  # State management for resumable operations
├── target/                       # Rust build artifacts
├── logs/                         # Production logs from runs
│   ├── full_photo_organization_log.txt  # Complete text log (1.8MB)
│   ├── photo_organizer.csv             # Structured CSV log (10MB)  
│   ├── photo_organizer_state.json      # State for resumable operations
│   └── test-logs/                      # Various test run logs
├── docs/                         # Documentation
│   ├── DEMO.md                   # Demo and usage guide
│   ├── ENHANCED_FEATURES.md      # Feature documentation
│   └── instructions.txt          # Setup instructions
├── test-data/                    # Test files and utilities
│   ├── set_old_time.py          # Script for testing old file dates
│   └── test_photos/             # Sample photos for testing
├── Cargo.toml                    # Rust project configuration
├── Cargo.lock                    # Rust dependency lock file
├── README.md                     # Main project documentation
└── test_date_inference.sh        # Test script for date inference
```

## 🎯 Main Project Components

### **Core Application**
- **`src/`** - All Rust source code for the photo organizer
- **`target/`** - Compiled binaries and build artifacts
- **`Cargo.toml`** - Project dependencies and metadata

### **Production Data** 
- **`logs/`** - Contains logs from your 15,250 photo organization run
  - Text log with deduplication tracking
  - CSV format for analysis
  - State file for resumable operations

### **Documentation**
- **`docs/`** - All project documentation
- **`README.md`** - Main project documentation

### **Testing & Development**
- **`test-data/`** - Sample data and utilities for testing
- **`test_date_inference.sh`** - Automated test script

## 🚀 Quick Start

```bash
cd photo-organizer
cargo build --release
./target/release/photo_organizer --help
```

## 📊 Your Recent Run Results

- **15,250 photos** successfully processed
- **Full logs available** in `logs/` directory
- **Zero errors** during processing
- **Enhanced deduplication logging** now implemented

---

*This structure consolidates all project artifacts into a single, organized directory.*