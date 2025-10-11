use crate::config::Config;
use crate::date_inference::DateInference;
use crate::deduplicator::Deduplicator;
use crate::file_info::FileInfo;
use crate::logger::CsvLogger;
use crate::progress::ProgressTracker;
use crate::scanner::Scanner;
use crate::state::ProcessingState;
use crate::text_logger::TextLogger;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

pub struct PhotoOrganizer {
    config: Config,
    scanner: Scanner,
    state: ProcessingState,
    logger: CsvLogger,
    text_logger: Option<TextLogger>,
}

impl PhotoOrganizer {
    pub fn new(config: Config) -> Result<Self> {
        let scanner = Scanner::new(config.clone());
        let state = ProcessingState::load_from_file(&config.state_file)?;
        let logger = CsvLogger::new(&config.log_file)?;
        
        let text_logger = if let Some(ref text_log_path) = config.text_log_file {
            Some(TextLogger::new(text_log_path)?)
        } else {
            None
        };
        
        Ok(Self {
            config,
            scanner,
            state,
            logger,
            text_logger,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting photo organization process");
        
        // Check if we should resume or start fresh
        let source_path_str = self.config.source.to_string_lossy().to_string();
        let should_resume = self.state.should_resume(&source_path_str, self.config.force_reprocess);
        
        if !should_resume {
            self.state.reset();
        }
        
        // Create target directory if it doesn't exist (unless in log-only mode)
        if !self.config.dry_run && !self.config.log_only {
            fs::create_dir_all(&self.config.target)
                .context("Failed to create target directory")?;
        }
        
        // Scan for files
        let progress = ProgressTracker::new(1); // We'll update this once we know the count
        progress.set_scanning_message(&source_path_str);
        println!("\n🚀 Starting photo organization process...");
        
        let all_files = self.scanner.scan_directory(&self.config.source)?;
        info!("Found {} total image files", all_files.len());
        println!("📊 Found {} total image files", all_files.len());
        
        // Filter out already processed files
        println!("🔄 Filtering already processed files...");
        let (mut files_to_process, already_processed) = self.state.filter_unprocessed_files(all_files);
        println!("ℹ️ Files to process: {}, Already processed: {}", 
                 files_to_process.len(), already_processed);
        
        // Perform directory-based date inference before processing
        println!("📅 Performing date inference from directory structure...");
        let inferred_count = DateInference::infer_dates_from_directories(&mut files_to_process);
        if inferred_count > 0 {
            info!("{} files had dates inferred from directory peers", inferred_count);
            println!("🧐 Inferred dates for {} files from directory peers", inferred_count);
        }
        
        if files_to_process.is_empty() {
            info!("No files to process. All files have been processed already.");
            println!("✅ All files have been processed already. Nothing to do!");
            return Ok(());
        }
        
        // Update state with scan info
        self.state.update_scan_info(&source_path_str, files_to_process.len() + already_processed);
        
        // Create new progress tracker with actual count
        let progress = ProgressTracker::new(files_to_process.len());
        
        // Find duplicates
        progress.set_deduplicating_message();
        println!("🔍 Finding duplicate files...");
        let duplicates = Deduplicator::find_duplicates(&files_to_process);
        info!("Found {} duplicate groups", duplicates.len());
        println!("✅ Found {} duplicate groups", duplicates.len());
        
        // Set target paths for all files
        for file in &mut files_to_process {
            let target_path = if file.date_reliable || !self.config.preserve_structure {
                // Use date-based organization for reliable dates or when not preserving structure
                let date_path = file.get_date_path();
                self.config.target.join(&date_path).join(&file.file_name).with_extension(&file.extension)
            } else {
                // Preserve source structure for unreliable dates
                match file.get_source_preserved_path(&self.config.source) {
                    Ok(preserved_path) => {
                        self.config.target.join(&preserved_path).join(&file.file_name).with_extension(&file.extension)
                    }
                    Err(_) => {
                        // Fallback to undated directory if path calculation fails
                        self.config.target.join("undated").join(&file.file_name).with_extension(&file.extension)
                    }
                }
            };
            file.target_path = Some(target_path);
        }
        
        // Resolve filename conflicts and get deduplication report
        let dedup_report = Deduplicator::resolve_duplicates_in_target(&mut files_to_process, &self.config.target, self.text_logger.as_mut())?;
        
        // Process files
        progress.set_organizing_message();
        let mut processed_count = 0;
        let mut error_count = 0;
        let mut unreliable_dates_count = 0;
        
        for file in &mut files_to_process {
            progress.update_current_file(&file.file_name);
            
            if !file.date_reliable {
                unreliable_dates_count += 1;
            }
            
            match self.process_single_file(file).await {
                Ok(_) => {
                    progress.increment_processed();
                    processed_count += 1;
                    
                    self.state.mark_file_processed(file);
                    
                    let operation = if self.config.log_only {
                        "LOG_ONLY"
                    } else if file.date_reliable {
                        "PROCESSED"
                    } else {
                        "PRESERVED_STRUCTURE"
                    };
                    
                    // Console output showing file processing
                    let date_source = if file.date_inferred {
                        "INFERRED"
                    } else if file.exif_date_available {
                        "EXIF"
                    } else if file.date_reliable {
                        "FILE_TIME"
                    } else {
                        "UNRELIABLE_DATE"
                    };
                    
                    let symbol = if file.date_reliable { "✓" } else { "⚠️" };
                    let target_display = file.target_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "NO_TARGET".to_string());
                    
                    println!("{} {} -> {} [{}] ({})", 
                             symbol,
                             file.source_path.display(), 
                             target_display,
                             date_source,
                             operation);
                    
                    if let Err(e) = self.logger.log_file_operation(file, operation) {
                        warn!("Failed to log file operation: {}", e);
                    }
                    
                    if let Some(ref mut text_logger) = self.text_logger {
                        if let Err(e) = text_logger.log_file_operation(file, operation) {
                            warn!("Failed to log to text file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to process file {}: {}", file.source_path.display(), e);
                    progress.increment_errors();
                    error_count += 1;
                    
                    self.state.mark_file_error();
                    
                    let error_msg = format!("ERROR: {}", e);
                    if let Err(log_err) = self.logger.log_file_operation(file, &error_msg) {
                        warn!("Failed to log error: {}", log_err);
                    }
                    
                    if let Some(ref mut text_logger) = self.text_logger {
                        if let Err(log_err) = text_logger.log_file_operation(file, &error_msg) {
                            warn!("Failed to log error to text file: {}", log_err);
                        }
                    }
                }
            }
            
            // Save state periodically
            if (processed_count + error_count) % 10 == 0 {
                if let Err(e) = self.state.save_to_file(&self.config.state_file) {
                    warn!("Failed to save state: {}", e);
                }
            }
        }
        
        // Final state save
        self.state.save_to_file(&self.config.state_file)
            .context("Failed to save final state")?;
        
        // Log summary
        if let Err(e) = self.logger.log_summary(files_to_process.len(), processed_count, error_count) {
            warn!("Failed to log summary: {}", e);
        }
        
        if let Some(ref mut text_logger) = self.text_logger {
            if let Err(e) = text_logger.log_summary_with_dedup(
                files_to_process.len(), 
                processed_count, 
                error_count, 
                unreliable_dates_count,
                dedup_report.duplicate_files_skipped,
                dedup_report.space_saved_bytes
            ) {
                warn!("Failed to log text summary: {}", e);
            }
        }
        
        progress.finish();
        info!("Photo organization completed: {} processed, {} errors", processed_count, error_count);
        
        // Print final summary with deduplication info
        println!("\n=== FINAL SUMMARY ===");
        println!("Total files found: {}", files_to_process.len() + already_processed);
        println!("Files processed: {}", processed_count);
        println!("Files with errors: {}", error_count);
        println!("Files with unreliable dates: {}", unreliable_dates_count);
        if dedup_report.duplicate_files_skipped > 0 {
            println!("Duplicate files skipped: {}", dedup_report.duplicate_files_skipped);
            println!("Space saved from deduplication: {:.2} MB", 
                     dedup_report.space_saved_bytes as f64 / 1_048_576.0);
        }
        println!("=====================\n");
        
        Ok(())
    }
    
    async fn process_single_file(&self, file: &mut FileInfo) -> Result<()> {
        let target_path = file.target_path.as_ref()
            .context("File has no target path")?;
        
        debug!("Processing file: {} -> {}", 
               file.source_path.display(), target_path.display());
        
        // Create target directory (unless in log-only mode)
        if let Some(parent) = target_path.parent() {
            if !self.config.dry_run && !self.config.log_only {
                fs::create_dir_all(parent)
                    .context("Failed to create target directory")?;
                
                // Create tag file if directory is interesting
                if file.is_interesting_directory() {
                    self.create_tag_file(parent, &file.directory_context)?;
                }
            }
        }
        
        // Copy or link the file (unless in log-only mode)
        if self.config.log_only {
            info!("LOG ONLY: Would copy {} to {}", 
                  file.source_path.display(), target_path.display());
        } else if !self.config.dry_run {
            if !target_path.exists() {
                fs::copy(&file.source_path, target_path)
                    .context("Failed to copy file")?;
                debug!("Copied file to: {}", target_path.display());
            } else {
                debug!("Target file already exists: {}", target_path.display());
            }
        } else {
            info!("DRY RUN: Would copy {} to {}", 
                  file.source_path.display(), target_path.display());
        }
        
        file.processed = true;
        Ok(())
    }
    
    fn create_tag_file(&self, target_dir: &Path, context: &str) -> Result<()> {
        let tag_file_path = target_dir.join(format!("{}.tag", context));
        
        if !tag_file_path.exists() {
            fs::write(&tag_file_path, format!("Photos from directory: {}", context))
                .context("Failed to create tag file")?;
            
            debug!("Created tag file: {}", tag_file_path.display());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    
    fn create_test_config(source: PathBuf, target: PathBuf) -> Config {
        Config {
            source,
            target,
            log_file: PathBuf::from("test.csv"),
            state_file: PathBuf::from("test_state.json"),
            dry_run: true,
            force_reprocess: false,
            threads: 1,
            extensions: vec!["jpg".to_string(), "txt".to_string()],
            preserve_structure: false,
            text_log_file: None,
            log_only: false,
        }
    }
    
    #[tokio::test]
    async fn test_organizer_creation() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        
        let config = create_test_config(
            source_dir.path().to_path_buf(),
            target_dir.path().to_path_buf(),
        );
        
        let organizer = PhotoOrganizer::new(config);
        assert!(organizer.is_ok());
    }
    
    #[test]
    fn test_create_tag_file() {
        let dir = tempdir().unwrap();
        let target_dir = dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let config = create_test_config(
            dir.path().to_path_buf(),
            target_dir.clone(),
        );
        
        let organizer = PhotoOrganizer::new(config).unwrap();
        
        let result = organizer.create_tag_file(&target_dir, "vacation_photos");
        assert!(result.is_ok());
        
        let tag_file = target_dir.join("vacation_photos.tag");
        assert!(tag_file.exists());
        
        let content = fs::read_to_string(tag_file).unwrap();
        assert!(content.contains("vacation_photos"));
    }
    
    #[tokio::test]
    async fn test_process_empty_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        
        let config = create_test_config(
            source_dir.path().to_path_buf(),
            target_dir.path().to_path_buf(),
        );
        
        let mut organizer = PhotoOrganizer::new(config).unwrap();
        let result = organizer.run().await;
        
        assert!(result.is_ok());
    }
}
