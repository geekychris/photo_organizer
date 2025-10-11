use crate::config::Config;
use crate::exif_extractor::ExifExtractor;
use crate::file_info::FileInfo;
use anyhow::{Context, Result};
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Scanner {
    config: Config,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    pub fn scan_directory(&self, path: &Path) -> Result<Vec<FileInfo>> {
        info!("Starting scan of directory: {}", path.display());
        println!("🔍 Scanning directory: {}", path.display());
        let mut files = Vec::new();
        let mut processed_count = 0;
        let mut skipped_count = 0;
        
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            
            if entry.file_type().is_file() {
                if let Some(extension) = file_path.extension() {
                    let ext_str = extension.to_string_lossy().to_lowercase();
                    
                    if self.config.is_supported_extension(&ext_str) {
                        processed_count += 1;
                        
                        // Show progress every 100 files
                        if processed_count % 100 == 0 {
                            println!("   📁 Processed {} image files so far...", processed_count);
                        }
                        
                        debug!("Processing file: {}", file_path.display());
                        
                        match self.process_file(file_path) {
                            Ok(file_info) => files.push(file_info),
                            Err(e) => {
                                warn!("Failed to process file {}: {}", file_path.display(), e);
                                continue;
                            }
                        }
                    } else {
                        skipped_count += 1;
                    }
                }
            }
        }
        
        info!("Scan completed. Found {} image files", files.len());
        println!("✅ Scan completed! Found {} image files (processed: {}, skipped: {})", 
                 files.len(), processed_count, skipped_count);
        Ok(files)
    }
    
    fn process_file(&self, path: &Path) -> Result<FileInfo> {
        let metadata = std::fs::metadata(path)
            .context("Failed to get file metadata")?;
        
        let file_size = metadata.len();
        let file_hash = self.calculate_file_hash(path)?;
        
        let (date_taken, exif_available, date_reliable) = ExifExtractor::extract_date_taken(path)
            .context("Failed to extract date from file")?;
        
        Ok(FileInfo::new(
            path.to_path_buf(),
            file_size,
            file_hash,
            date_taken,
            exif_available,
            date_reliable,
        ))
    }
    
    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let mut file = File::open(path)
            .context("Failed to open file for hashing")?;
        
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .context("Failed to read file for hashing")?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    
    fn create_test_config() -> Config {
        Config {
            source: PathBuf::from("/tmp"),
            target: PathBuf::from("/tmp/target"),
            log_file: PathBuf::from("test.csv"),
            state_file: PathBuf::from("test_state.json"),
            dry_run: false,
            force_reprocess: false,
            threads: 1,
            extensions: vec!["jpg".to_string(), "jpeg".to_string(), "txt".to_string()],
            preserve_structure: false,
            text_log_file: None,
            log_only: false,
        }
    }
    
    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let config = create_test_config();
        let scanner = Scanner::new(config);
        
        let results = scanner.scan_directory(dir.path()).unwrap();
        assert!(results.is_empty());
    }
    
    #[test]
    fn test_scan_with_supported_files() {
        let dir = tempdir().unwrap();
        
        // Create test files
        let jpg_path = dir.path().join("test.jpg");
        let txt_path = dir.path().join("test.txt");
        let unsupported_path = dir.path().join("test.xyz");
        
        let mut jpg_file = File::create(&jpg_path).unwrap();
        writeln!(jpg_file, "fake jpg content").unwrap();
        
        let mut txt_file = File::create(&txt_path).unwrap();
        writeln!(txt_file, "text content").unwrap();
        
        let mut unsupported_file = File::create(&unsupported_path).unwrap();
        writeln!(unsupported_file, "unsupported content").unwrap();
        
        let config = create_test_config();
        let scanner = Scanner::new(config);
        
        let results = scanner.scan_directory(dir.path()).unwrap();
        assert_eq!(results.len(), 2); // Only jpg and txt should be found
        
        let extensions: Vec<String> = results.iter()
            .map(|f| f.extension.clone())
            .collect();
        
        assert!(extensions.contains(&"jpg".to_string()));
        assert!(extensions.contains(&"txt".to_string()));
        assert!(!extensions.contains(&"xyz".to_string()));
    }
    
    #[test]
    fn test_calculate_file_hash() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test content for hashing").unwrap();
        
        let config = create_test_config();
        let scanner = Scanner::new(config);
        
        let hash1 = scanner.calculate_file_hash(&file_path).unwrap();
        let hash2 = scanner.calculate_file_hash(&file_path).unwrap();
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
    }
}