use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

pub struct ProgressTracker {
    progress_bar: ProgressBar,
    start_time: Instant,
    processed_count: Arc<AtomicUsize>,
    error_count: Arc<AtomicUsize>,
}

impl ProgressTracker {
    pub fn new(total_files: usize) -> Self {
        let progress_bar = ProgressBar::new(total_files as u64);
        
        let style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({per_sec}, ETA: {eta})")
            .expect("Failed to create progress style")
            .progress_chars("#>-");
        
        progress_bar.set_style(style);
        progress_bar.set_message("Organizing photos...");
        
        Self {
            progress_bar,
            start_time: Instant::now(),
            processed_count: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn update_current_file(&self, filename: &str) {
        let truncated = if filename.len() > 50 {
            format!("...{}", &filename[filename.len()-47..])
        } else {
            filename.to_string()
        };
        
        self.progress_bar.set_message(format!("Processing: {}", truncated));
    }
    
    pub fn increment_processed(&self) {
        self.processed_count.fetch_add(1, Ordering::SeqCst);
        self.progress_bar.inc(1);
    }
    
    pub fn increment_errors(&self) {
        self.error_count.fetch_add(1, Ordering::SeqCst);
        self.progress_bar.inc(1);
    }
    
    pub fn set_scanning_message(&self, dir: &str) {
        self.progress_bar.set_message(format!("Scanning directory: {}", dir));
    }
    
    pub fn set_deduplicating_message(&self) {
        self.progress_bar.set_message("Finding and resolving duplicates...");
    }
    
    pub fn set_organizing_message(&self) {
        self.progress_bar.set_message("Organizing files by date...");
    }
    
    pub fn finish(&self) {
        let duration = self.start_time.elapsed();
        let processed = self.processed_count.load(Ordering::SeqCst);
        let errors = self.error_count.load(Ordering::SeqCst);
        
        self.progress_bar.finish_with_message(format!(
            "Completed in {:.2}s: {} files processed, {} errors",
            duration.as_secs_f32(),
            processed,
            errors
        ));
    }
    
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.processed_count.load(Ordering::SeqCst),
            self.error_count.load(Ordering::SeqCst),
        )
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new(100);
        let (processed, errors) = tracker.get_stats();
        assert_eq!(processed, 0);
        assert_eq!(errors, 0);
    }
    
    #[test]
    fn test_progress_tracking() {
        let tracker = ProgressTracker::new(10);
        
        tracker.increment_processed();
        tracker.increment_processed();
        tracker.increment_errors();
        
        let (processed, errors) = tracker.get_stats();
        assert_eq!(processed, 2);
        assert_eq!(errors, 1);
    }
    
    #[test]
    fn test_current_file_update() {
        let tracker = ProgressTracker::new(5);
        
        // Test normal length filename
        tracker.update_current_file("test.jpg");
        
        // Test long filename truncation
        let long_filename = "a".repeat(60) + ".jpg";
        tracker.update_current_file(&long_filename);
        
        // Should not panic or cause issues
    }
    
    #[test]
    fn test_message_updates() {
        let tracker = ProgressTracker::new(5);
        
        tracker.set_scanning_message("/path/to/photos");
        tracker.set_deduplicating_message();
        tracker.set_organizing_message();
        
        // All should complete without issues
    }
}