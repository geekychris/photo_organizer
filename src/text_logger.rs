use crate::file_info::FileInfo;
use anyhow::{Context, Result};
use log::info;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct TextLogger {
    file: std::fs::File,
}

impl TextLogger {
    pub fn new(log_path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .context("Failed to open text log file")?;
        
        Ok(Self { file })
    }
    
    pub fn log_file_operation(
        &mut self,
        file_info: &FileInfo,
        operation: &str,
    ) -> Result<()> {
        let source = file_info.source_path.to_string_lossy();
        let target = file_info.target_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "N/A".to_string());
        
        let date_status = if file_info.exif_date_available {
            "EXIF"
        } else if file_info.date_inferred {
            "INFERRED"
        } else if file_info.date_reliable {
            "FILE_TIME"
        } else {
            "UNRELIABLE_DATE"
        };
        
        let line = if !file_info.date_reliable {
            format!("⚠️  {} -> {} [{}] ({})\n", source, target, date_status, operation)
        } else {
            format!("✓  {} -> {} [{}] ({})\n", source, target, date_status, operation)
        };
        
        self.file.write_all(line.as_bytes())
            .context("Failed to write to text log file")?;
        
        Ok(())
    }
    
    pub fn log_deduplication_entry(
        &mut self,
        source_path: &str,
        target_path: &str,
        file_size: u64,
    ) -> Result<()> {
        let line = format!("🔄 {} -> {} [DUPLICATE] (SKIPPED - {:.2} MB saved)\n", 
                          source_path, target_path, file_size as f64 / 1_048_576.0);
        
        self.file.write_all(line.as_bytes())
            .context("Failed to write deduplication entry to text log file")?;
        
        Ok(())
    }
    
    pub fn log_summary(&mut self, total_files: usize, processed_files: usize, errors: usize, unreliable_dates: usize) -> Result<()> {
        self.log_summary_with_dedup(total_files, processed_files, errors, unreliable_dates, 0, 0)
    }
    
    pub fn log_summary_with_dedup(&mut self, total_files: usize, processed_files: usize, errors: usize, unreliable_dates: usize, duplicates_skipped: usize, space_saved_bytes: u64) -> Result<()> {
        let mut summary = format!("\n=== SUMMARY ===\n\
            Total files: {}\n\
            Processed: {}\n\
            Errors: {}\n\
            Files with unreliable dates: {}\n",
            total_files, processed_files, errors, unreliable_dates
        );
        
        if duplicates_skipped > 0 {
            summary.push_str(&format!("\
            Duplicate files skipped: {}\n\
            Space saved from deduplication: {:.2} MB\n",
                duplicates_skipped, space_saved_bytes as f64 / 1_048_576.0));
        }
        
        summary.push_str("\
            \n\
            Legend:\n\
            ✓  = Successfully processed with reliable date\n\
            ⚠️  = Processed but date may be unreliable (preserved source structure)\n\
            🔄 = Skipped duplicate file\n\
            EXIF = Date from image metadata\n\
            INFERRED = Date inferred from directory peers with EXIF data\n\
            FILE_TIME = Date from file timestamp\n\
            UNRELIABLE_DATE = File timestamp seems invalid (too old/future)\n");
        
        self.file.write_all(summary.as_bytes())
            .context("Failed to write summary to text log file")?;
        
        self.file.flush()
            .context("Failed to flush text log file")?;
        
        info!("Text log summary written");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::tempdir;
    
    fn create_test_file_info(reliable: bool, exif: bool) -> FileInfo {
        let mut file = FileInfo::new(
            PathBuf::from("/source/test.jpg"),
            1024,
            "abcd1234".to_string(),
            Utc::now(),
            exif,
            reliable,
        );
        file.date_inferred = false; // Set explicitly for tests
        file
    }
    
    #[test]
    fn test_text_logger_creation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.txt");
        
        let logger = TextLogger::new(&log_path);
        assert!(logger.is_ok());
        assert!(log_path.exists());
    }
    
    #[test]
    fn test_log_reliable_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.txt");
        
        let mut logger = TextLogger::new(&log_path).unwrap();
        let mut file_info = create_test_file_info(true, true);
        file_info.target_path = Some(PathBuf::from("/target/25/01/15/test.jpg"));
        
        let result = logger.log_file_operation(&file_info, "COPIED");
        assert!(result.is_ok());
        
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("✓"));
        assert!(content.contains("[EXIF]"));
        assert!(content.contains("COPIED"));
    }
    
    #[test]
    fn test_log_unreliable_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.txt");
        
        let mut logger = TextLogger::new(&log_path).unwrap();
        let mut file_info = create_test_file_info(false, false);
        file_info.target_path = Some(PathBuf::from("/target/undated/old_photos/test.jpg"));
        
        let result = logger.log_file_operation(&file_info, "PRESERVED_STRUCTURE");
        assert!(result.is_ok());
        
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("⚠️"));
        assert!(content.contains("[UNRELIABLE_DATE]"));
        assert!(content.contains("PRESERVED_STRUCTURE"));
    }
    
    #[test]
    fn test_summary_logging() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.txt");
        
        let mut logger = TextLogger::new(&log_path).unwrap();
        let result = logger.log_summary(100, 95, 5, 10);
        assert!(result.is_ok());
        
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("SUMMARY"));
        assert!(content.contains("Total files: 100"));
        assert!(content.contains("Files with unreliable dates: 10"));
        assert!(content.contains("Legend:"));
    }
}