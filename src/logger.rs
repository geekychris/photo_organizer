use crate::file_info::FileInfo;
use anyhow::{Context, Result};
use csv::Writer;
use log::info;
use serde::Serialize;
use std::fs::OpenOptions;
use std::path::Path;

#[derive(Serialize)]
struct LogEntry {
    source_path: String,
    target_path: String,
    timestamp: String,
    directory_context: String,
    file_size: u64,
    file_hash: String,
    exif_date_available: bool,
    operation: String,
}

pub struct CsvLogger {
    writer: Writer<std::fs::File>,
}

impl CsvLogger {
    pub fn new(log_path: &Path) -> Result<Self> {
        let file_exists = log_path.exists();
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open log file")?;
        
        let mut writer = Writer::from_writer(file);
        
        // Write headers if this is a new file
        if !file_exists {
            writer.write_record(&[
                "source_path",
                "target_path",
                "timestamp",
                "directory_context",
                "file_size",
                "file_hash",
                "exif_date_available",
                "operation"
            ]).context("Failed to write CSV headers")?;
        }
        
        Ok(Self { writer })
    }
    
    pub fn log_file_operation(
        &mut self,
        file_info: &FileInfo,
        operation: &str,
    ) -> Result<()> {
        let entry = LogEntry {
            source_path: file_info.source_path.to_string_lossy().to_string(),
            target_path: file_info.target_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            timestamp: file_info.date_taken.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            directory_context: file_info.directory_context.clone(),
            file_size: file_info.file_size,
            file_hash: file_info.file_hash.clone(),
            exif_date_available: file_info.exif_date_available,
            operation: operation.to_string(),
        };
        
        self.writer.serialize(entry)
            .context("Failed to write log entry")?;
        
        self.writer.flush()
            .context("Failed to flush log file")?;
        
        Ok(())
    }
    
    pub fn log_summary(&mut self, total_files: usize, processed_files: usize, errors: usize) -> Result<()> {
        info!("Logging summary: {} total, {} processed, {} errors", 
              total_files, processed_files, errors);
        
        let summary_entry = LogEntry {
            source_path: "SUMMARY".to_string(),
            target_path: format!("Processed: {}, Errors: {}", processed_files, errors),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            directory_context: format!("Total files: {}", total_files),
            file_size: 0,
            file_hash: "N/A".to_string(),
            exif_date_available: false,
            operation: "SUMMARY".to_string(),
        };
        
        self.writer.serialize(summary_entry)
            .context("Failed to write summary entry")?;
        
        self.writer.flush()
            .context("Failed to flush summary")?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::tempdir;
    
    fn create_test_file_info() -> FileInfo {
        FileInfo::new(
            PathBuf::from("/source/test.jpg"),
            1024,
            "abcd1234".to_string(),
            Utc::now(),
            true,
            true,
        )
    }
    
    #[test]
    fn test_csv_logger_creation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.csv");
        
        let logger = CsvLogger::new(&log_path);
        assert!(logger.is_ok());
        assert!(log_path.exists());
    }
    
    #[test]
    fn test_log_file_operation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.csv");
        
        let mut logger = CsvLogger::new(&log_path).unwrap();
        let mut file_info = create_test_file_info();
        file_info.target_path = Some(PathBuf::from("/target/23/01/15/test.jpg"));
        
        let result = logger.log_file_operation(&file_info, "COPIED");
        assert!(result.is_ok());
        
        // Verify file has content
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("source_path"));
        assert!(content.contains("/source/test.jpg"));
        assert!(content.contains("COPIED"));
    }
    
    #[test]
    fn test_log_summary() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.csv");
        
        let mut logger = CsvLogger::new(&log_path).unwrap();
        let result = logger.log_summary(100, 95, 5);
        assert!(result.is_ok());
        
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("SUMMARY"));
        assert!(content.contains("Processed: 95"));
    }
}