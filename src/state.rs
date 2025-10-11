use crate::file_info::FileInfo;
use anyhow::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingState {
    pub processed_files: HashSet<String>, // Store file hashes of processed files
    pub last_scan_path: Option<String>,
    pub total_files_found: usize,
    pub processed_count: usize,
    pub error_count: usize,
    pub timestamp: String,
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self {
            processed_files: HashSet::new(),
            last_scan_path: None,
            total_files_found: 0,
            processed_count: 0,
            error_count: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl ProcessingState {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            info!("No existing state file found, starting fresh");
            return Ok(Self::default());
        }
        
        info!("Loading existing state from: {}", path.display());
        
        let contents = fs::read_to_string(path)
            .context("Failed to read state file")?;
        
        let state: ProcessingState = serde_json::from_str(&contents)
            .context("Failed to parse state file")?;
        
        info!("Loaded state: {} previously processed files", state.processed_files.len());
        
        Ok(state)
    }
    
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        debug!("Saving state to: {}", path.display());
        
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize state")?;
        
        fs::write(path, contents)
            .context("Failed to write state file")?;
        
        debug!("State saved successfully");
        Ok(())
    }
    
    pub fn is_file_processed(&self, file_info: &FileInfo) -> bool {
        self.processed_files.contains(&file_info.file_hash)
    }
    
    pub fn mark_file_processed(&mut self, file_info: &FileInfo) {
        self.processed_files.insert(file_info.file_hash.clone());
        self.processed_count += 1;
        self.update_timestamp();
    }
    
    pub fn mark_file_error(&mut self) {
        self.error_count += 1;
        self.update_timestamp();
    }
    
    pub fn filter_unprocessed_files(&self, files: Vec<FileInfo>) -> (Vec<FileInfo>, usize) {
        if self.processed_files.is_empty() {
            info!("No previously processed files, processing all {} files", files.len());
            return (files, 0);
        }
        
        let (unprocessed, already_processed): (Vec<_>, Vec<_>) = files
            .into_iter()
            .partition(|file| !self.is_file_processed(file));
        
        info!("Found {} unprocessed files out of {} total files", 
              unprocessed.len(), unprocessed.len() + already_processed.len());
        
        if !already_processed.is_empty() {
            info!("Skipping {} already processed files", already_processed.len());
        }
        
        (unprocessed, already_processed.len())
    }
    
    pub fn update_scan_info(&mut self, scan_path: &str, total_files: usize) {
        self.last_scan_path = Some(scan_path.to_string());
        self.total_files_found = total_files;
        self.update_timestamp();
    }
    
    fn update_timestamp(&mut self) {
        self.timestamp = chrono::Utc::now().to_rfc3339();
    }
    
    pub fn should_resume(&self, current_scan_path: &str, force_reprocess: bool) -> bool {
        if force_reprocess {
            info!("Force reprocess requested, ignoring existing state");
            return false;
        }
        
        match &self.last_scan_path {
            Some(last_path) if last_path == current_scan_path => {
                if !self.processed_files.is_empty() {
                    info!("Resuming previous scan of: {}", current_scan_path);
                    true
                } else {
                    false
                }
            }
            Some(last_path) => {
                warn!("Scan path changed from '{}' to '{}', starting fresh", 
                      last_path, current_scan_path);
                false
            }
            None => {
                info!("No previous scan path, starting fresh");
                false
            }
        }
    }
    
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::tempdir;
    
    fn create_test_file_info(hash: &str) -> FileInfo {
        FileInfo::new(
            PathBuf::from("/test/path.jpg"),
            1000,
            hash.to_string(),
            Utc::now(),
            false,
            true,
        )
    }
    
    #[test]
    fn test_state_creation() {
        let state = ProcessingState::default();
        assert!(state.processed_files.is_empty());
        assert_eq!(state.processed_count, 0);
        assert_eq!(state.error_count, 0);
    }
    
    #[test]
    fn test_file_processing() {
        let mut state = ProcessingState::default();
        let file = create_test_file_info("hash1");
        
        assert!(!state.is_file_processed(&file));
        
        state.mark_file_processed(&file);
        
        assert!(state.is_file_processed(&file));
        assert_eq!(state.processed_count, 1);
    }
    
    #[test]
    fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        
        // Create and save state
        let mut state = ProcessingState::default();
        let file = create_test_file_info("hash1");
        state.mark_file_processed(&file);
        state.update_scan_info("/test/path", 100);
        
        state.save_to_file(&state_path).unwrap();
        assert!(state_path.exists());
        
        // Load state
        let loaded_state = ProcessingState::load_from_file(&state_path).unwrap();
        assert_eq!(loaded_state.processed_files.len(), 1);
        assert_eq!(loaded_state.total_files_found, 100);
        assert_eq!(loaded_state.last_scan_path, Some("/test/path".to_string()));
    }
    
    #[test]
    fn test_filter_unprocessed_files() {
        let mut state = ProcessingState::default();
        let file1 = create_test_file_info("hash1");
        let file2 = create_test_file_info("hash2");
        let file3 = create_test_file_info("hash3");
        
        // Mark file1 as processed
        state.mark_file_processed(&file1);
        
        let files = vec![file1.clone(), file2.clone(), file3.clone()];
        let (unprocessed, processed_count) = state.filter_unprocessed_files(files);
        
        assert_eq!(unprocessed.len(), 2);
        assert_eq!(processed_count, 1);
        
        // Check that the right files are unprocessed
        let unprocessed_hashes: HashSet<String> = unprocessed
            .iter()
            .map(|f| f.file_hash.clone())
            .collect();
        
        assert!(unprocessed_hashes.contains("hash2"));
        assert!(unprocessed_hashes.contains("hash3"));
        assert!(!unprocessed_hashes.contains("hash1"));
    }
    
    #[test]
    fn test_should_resume() {
        let mut state = ProcessingState::default();
        
        // Should not resume with empty state
        assert!(!state.should_resume("/test/path", false));
        
        // Add some processed files and scan path
        let file = create_test_file_info("hash1");
        state.mark_file_processed(&file);
        state.update_scan_info("/test/path", 10);
        
        // Should resume with same path
        assert!(state.should_resume("/test/path", false));
        
        // Should not resume with different path
        assert!(!state.should_resume("/different/path", false));
        
        // Should not resume when force reprocess is true
        assert!(!state.should_resume("/test/path", true));
    }
    
    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("nonexistent.json");
        
        let state = ProcessingState::load_from_file(&state_path).unwrap();
        assert!(state.processed_files.is_empty());
    }
}
