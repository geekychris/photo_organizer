use crate::file_info::FileInfo;
use crate::text_logger::TextLogger;
use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DuplicateGroup {
    pub files: Vec<FileInfo>,
    pub hash: String,
}

#[derive(Debug)]
pub struct DeduplicationReport {
    pub total_files_processed: usize,
    pub duplicate_files_skipped: usize,
    pub space_saved_bytes: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
}

pub struct Deduplicator;

impl Deduplicator {
    pub fn find_duplicates(files: &[FileInfo]) -> Vec<DuplicateGroup> {
        let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();
        
        for file in files {
            hash_groups
                .entry(file.file_hash.clone())
                .or_insert_with(Vec::new)
                .push(file.clone());
        }
        
        let duplicates: Vec<DuplicateGroup> = hash_groups
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|(hash, files)| DuplicateGroup { hash, files })
            .collect();
        
        info!("Found {} groups with duplicate content", duplicates.len());
        
        for group in &duplicates {
            debug!("Duplicate group with hash {}: {} files", 
                   group.hash, group.files.len());
        }
        
        duplicates
    }
    
    pub fn resolve_duplicates_in_target(
        files: &mut [FileInfo], 
        _target_dir: &Path,
        mut text_logger: Option<&mut TextLogger>
    ) -> Result<DeduplicationReport> {
        info!("Resolving potential filename conflicts in target directory");
        
        let mut duplicate_files_skipped = 0;
        let mut space_saved_bytes = 0u64;
        let mut dedup_details: Vec<(String, String, u64)> = Vec::new(); // (source, existing_target, size)
        
        // Group files by their intended target path (without considering duplicates)
        let mut target_path_groups: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        
        for (idx, file) in files.iter().enumerate() {
            if let Some(ref target_path) = file.target_path {
                target_path_groups
                    .entry(target_path.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
        
        // Resolve conflicts for groups with multiple files targeting same path
        for (target_path, file_indices) in target_path_groups {
            if file_indices.len() > 1 {
                Self::resolve_path_conflict_internal(
                    &mut files[..], 
                    &file_indices, 
                    &target_path, 
                    &mut dedup_details,
                    &mut duplicate_files_skipped,
                    &mut space_saved_bytes
                )?;
            }
        }
        
        // Log deduplication details to text file if logger is available
        if let Some(ref mut logger) = text_logger {
            for (source, target, size) in &dedup_details {
                if let Err(e) = logger.log_deduplication_entry(source, target, *size) {
                    warn!("Failed to log deduplication entry: {}", e);
                }
            }
        }
        
        // Print deduplication report to console
        if duplicate_files_skipped > 0 {
            println!("\n=== DEDUPLICATION REPORT ===");
            println!("Files skipped (duplicates): {}", duplicate_files_skipped);
            println!("Space saved: {:.2} MB ({} bytes)", 
                     space_saved_bytes as f64 / 1_048_576.0, 
                     space_saved_bytes);
            println!("\nDuplicate files detected:");
            for (source, target, size) in &dedup_details {
                println!("  SKIP {} → {} (duplicate, {:.2} MB)", 
                         source, target, *size as f64 / 1_048_576.0);
            }
            println!("============================\n");
        }
        
        Ok(DeduplicationReport {
            total_files_processed: files.len(),
            duplicate_files_skipped,
            space_saved_bytes,
            duplicate_groups: Vec::new(), // Could be populated if needed
        })
    }
    
    fn resolve_path_conflict_internal(
        files: &mut [FileInfo],
        indices: &[usize],
        base_target_path: &Path,
        dedup_details: &mut Vec<(String, String, u64)>,
        duplicates_skipped: &mut usize,
        space_saved: &mut u64,
    ) -> Result<()> {
        info!("Resolving path conflict for {} files targeting: {}", 
              indices.len(), base_target_path.display());
        
        // Check if files exist on disk and compare content
        let mut new_file_indices: Vec<usize> = Vec::new();
        
        for &idx in indices {
            let (source_path, target_path_opt, file_size) = {
                let file = &files[idx];
                (file.source_path.clone(), file.target_path.clone(), file.file_size)
            };
            
            if let Some(ref target_path) = target_path_opt {
                if target_path.exists() {
                    // File exists, check if it's the same content
                    match Self::compare_file_content(&source_path, target_path) {
                        Ok(true) => {
                            debug!("File {} already exists with same content", 
                                   target_path.display());
                            // Mark as processed, no need to copy - this is a duplicate!
                            files[idx].processed = true;
                            *duplicates_skipped += 1;
                            *space_saved += file_size;
                            dedup_details.push((
                                source_path.display().to_string(),
                                target_path.display().to_string(),
                                file_size,
                            ));
                            
                            // Note: text_logger is immutable reference, can't log here
                            // We'll handle logging outside this method
                        }
                        Ok(false) => {
                            debug!("File {} exists with different content", 
                                   target_path.display());
                            new_file_indices.push(idx);
                        }
                        Err(e) => {
                            warn!("Failed to compare file content for {}: {}", 
                                  target_path.display(), e);
                            new_file_indices.push(idx);
                        }
                    }
                } else {
                    new_file_indices.push(idx);
                }
            }
        }
        
        // Generate unique filenames for conflicting files
        for (counter, &idx) in new_file_indices.iter().enumerate() {
            if counter > 0 || Self::path_would_conflict(base_target_path, &files[new_file_indices[0]]) {
                let new_path = Self::generate_unique_filename(base_target_path, counter)?;
                files[idx].target_path = Some(new_path);
                debug!("Generated unique filename for file {}: {}", 
                       files[idx].source_path.display(), 
                       files[idx].target_path.as_ref().unwrap().display());
            }
        }
        
        Ok(())
    }
    
    fn path_would_conflict(target_path: &Path, file: &FileInfo) -> bool {
        if target_path.exists() {
            match Self::compare_file_content(&file.source_path, target_path) {
                Ok(same_content) => !same_content,
                Err(_) => true, // Assume conflict if we can't compare
            }
        } else {
            false
        }
    }
    
    fn compare_file_content(source: &Path, target: &Path) -> Result<bool> {
        use std::fs::File;
        use std::io::Read;
        
        let source_metadata = std::fs::metadata(source)
            .context("Failed to get source file metadata")?;
        let target_metadata = std::fs::metadata(target)
            .context("Failed to get target file metadata")?;
        
        // Quick size check first
        if source_metadata.len() != target_metadata.len() {
            return Ok(false);
        }
        
        // If sizes match, compare content
        let mut source_file = File::open(source)
            .context("Failed to open source file")?;
        let mut target_file = File::open(target)
            .context("Failed to open target file")?;
        
        let mut source_buffer = [0; 8192];
        let mut target_buffer = [0; 8192];
        
        loop {
            let source_bytes = source_file.read(&mut source_buffer)
                .context("Failed to read from source file")?;
            let target_bytes = target_file.read(&mut target_buffer)
                .context("Failed to read from target file")?;
            
            if source_bytes != target_bytes {
                return Ok(false);
            }
            
            if source_bytes == 0 {
                break; // End of files
            }
            
            if source_buffer[..source_bytes] != target_buffer[..target_bytes] {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    fn generate_unique_filename(base_path: &Path, counter: usize) -> Result<PathBuf> {
        let parent = base_path.parent()
            .context("Failed to get parent directory")?;
        
        let stem = base_path.file_stem()
            .and_then(|s| s.to_str())
            .context("Failed to get file stem")?;
        
        let extension = base_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let unique_name = if extension.is_empty() {
            format!("{}_{:03}", stem, counter + 1)
        } else {
            format!("{}_{:03}.{}", stem, counter + 1, extension)
        };
        
        Ok(parent.join(unique_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    
    fn create_test_file_info(path: &str, hash: &str) -> FileInfo {
        FileInfo::new(
            PathBuf::from(path),
            1000,
            hash.to_string(),
            Utc::now(),
            false,
            true,
        )
    }
    
    #[test]
    fn test_find_duplicates() {
        let files = vec![
            create_test_file_info("/path/to/file1.jpg", "hash1"),
            create_test_file_info("/path/to/file2.jpg", "hash2"),
            create_test_file_info("/path/to/file3.jpg", "hash1"), // duplicate
            create_test_file_info("/path/to/file4.jpg", "hash3"),
            create_test_file_info("/path/to/file5.jpg", "hash2"), // duplicate
        ];
        
        let duplicates = Deduplicator::find_duplicates(&files);
        assert_eq!(duplicates.len(), 2);
        
        // Check that we have groups for hash1 and hash2
        let hash1_group = duplicates.iter().find(|g| g.hash == "hash1").unwrap();
        let hash2_group = duplicates.iter().find(|g| g.hash == "hash2").unwrap();
        
        assert_eq!(hash1_group.files.len(), 2);
        assert_eq!(hash2_group.files.len(), 2);
    }
    
    #[test]
    fn test_generate_unique_filename() {
        let base_path = PathBuf::from("/tmp/photo.jpg");
        
        let unique1 = Deduplicator::generate_unique_filename(&base_path, 0).unwrap();
        let unique2 = Deduplicator::generate_unique_filename(&base_path, 1).unwrap();
        
        assert_eq!(unique1, PathBuf::from("/tmp/photo_001.jpg"));
        assert_eq!(unique2, PathBuf::from("/tmp/photo_002.jpg"));
    }
    
    #[test]
    fn test_compare_file_content() {
        let dir = tempdir().unwrap();
        
        let file1_path = dir.path().join("file1.txt");
        let file2_path = dir.path().join("file2.txt");
        let file3_path = dir.path().join("file3.txt");
        
        // Create identical files
        let mut file1 = File::create(&file1_path).unwrap();
        let mut file2 = File::create(&file2_path).unwrap();
        let content = "identical content";
        
        writeln!(file1, "{}", content).unwrap();
        writeln!(file2, "{}", content).unwrap();
        
        // Create different file
        let mut file3 = File::create(&file3_path).unwrap();
        writeln!(file3, "different content").unwrap();
        
        // Test comparisons
        assert!(Deduplicator::compare_file_content(&file1_path, &file2_path).unwrap());
        assert!(!Deduplicator::compare_file_content(&file1_path, &file3_path).unwrap());
    }
}