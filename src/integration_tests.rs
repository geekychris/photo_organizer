use crate::config::Config;
use crate::file_info::FileInfo;
use crate::organizer::PhotoOrganizer;
use chrono::{Duration, Utc};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;
use tempfile::tempdir;
use filetime;

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_config(source: PathBuf, target: PathBuf, preserve_structure: bool, text_log: Option<PathBuf>, log_only: bool) -> Config {
        Config {
            source,
            target,
            log_file: PathBuf::from("integration_test.csv"),
            state_file: PathBuf::from("integration_test_state.json"),
            dry_run: false,
            force_reprocess: true, // Always reprocess in tests
            threads: 1,
            extensions: vec!["jpg".to_string(), "txt".to_string()],
            preserve_structure,
            text_log_file: text_log,
            log_only,
        }
    }
    
    #[test]
    fn test_source_preserved_path() {
        let temp_dir = tempdir().unwrap();
        let source_root = temp_dir.path().join("source");
        let file_path = source_root.join("photos").join("vacation").join("beach.jpg");
        
        let file_info = FileInfo::new(
            file_path.clone(),
            1000,
            "hash123".to_string(),
            Utc::now(),
            false,
            false, // unreliable date
        );
        
        let preserved_path = file_info.get_source_preserved_path(&source_root).unwrap();
        assert_eq!(preserved_path, PathBuf::from("undated/photos/vacation"));
    }
    
    #[test]
    fn test_date_reliability_very_old_file() {
        use crate::exif_extractor::ExifExtractor;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("old_file.txt");
        
        // Create file with content
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "old file content").unwrap();
        drop(file); // Close file
        
        // Set modification time to 20 years ago
        let twenty_years_ago = SystemTime::now() - std::time::Duration::from_secs(20 * 365 * 24 * 60 * 60);
        filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(twenty_years_ago)).unwrap();
        
        let (date, exif_available, date_reliable) = ExifExtractor::extract_date_taken(&file_path).unwrap();
        
        assert!(!exif_available); // Should not have EXIF
        assert!(!date_reliable); // Should be unreliable due to old timestamp
    }
    
    #[tokio::test]
    async fn test_preserve_structure_mode() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let log_file = temp_dir.path().join("test.txt");
        
        // Create source directory structure
        fs::create_dir_all(source_dir.join("old_photos")).unwrap();
        
        // Create an old file (simulate unreliable date)
        let old_file = source_dir.join("old_photos").join("ancient.jpg");
        let mut file = File::create(&old_file).unwrap();
        writeln!(file, "very old photo").unwrap();
        drop(file);
        
        // Set very old modification time
        let ancient_time = SystemTime::now() - std::time::Duration::from_secs(20 * 365 * 24 * 60 * 60);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(ancient_time)).unwrap();
        
        let config = create_test_config(
            source_dir.clone(),
            target_dir.clone(),
            true,  // preserve structure
            Some(log_file.clone()),
            false, // not log-only
        );
        
        let mut organizer = PhotoOrganizer::new(config).unwrap();
        let result = organizer.run().await;
        assert!(result.is_ok());
        
        // Check that file was placed in undated directory preserving structure
        let expected_target = target_dir.join("undated").join("old_photos").join("ancient.jpg");
        assert!(expected_target.exists());
        
        // Check that text log was created and contains warning
        assert!(log_file.exists());
        let log_content = fs::read_to_string(&log_file).unwrap();
        assert!(log_content.contains("⚠️"));
        assert!(log_content.contains("UNRELIABLE_DATE"));
        assert!(log_content.contains("undated/old_photos"));
    }
    
    #[tokio::test] 
    async fn test_log_only_mode() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let log_file = temp_dir.path().join("log_only.txt");
        
        // Create source file
        fs::create_dir_all(&source_dir).unwrap();
        let test_file = source_dir.join("test.jpg");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test photo").unwrap();
        
        let config = create_test_config(
            source_dir,
            target_dir.clone(),
            false,
            Some(log_file.clone()),
            true,  // log-only mode
        );
        
        let mut organizer = PhotoOrganizer::new(config).unwrap();
        let result = organizer.run().await;
        assert!(result.is_ok());
        
        // Target directory should not be created in log-only mode
        assert!(!target_dir.exists());
        
        // But log file should exist and contain the mapping
        assert!(log_file.exists());
        let log_content = fs::read_to_string(&log_file).unwrap();
        assert!(log_content.contains("LOG_ONLY"));
        assert!(log_content.contains("test.jpg"));
    }
    
    #[tokio::test]
    async fn test_mixed_reliable_unreliable_dates() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let log_file = temp_dir.path().join("mixed.txt");
        
        fs::create_dir_all(source_dir.join("recent_photos")).unwrap();
        fs::create_dir_all(source_dir.join("old_photos")).unwrap();
        
        // Create a recent file (reliable date)
        let recent_file = source_dir.join("recent_photos").join("recent.jpg");
        let mut file = File::create(&recent_file).unwrap();
        writeln!(file, "recent photo").unwrap();
        
        // Create an old file (unreliable date)
        let old_file = source_dir.join("old_photos").join("old.jpg");
        let mut file = File::create(&old_file).unwrap();
        writeln!(file, "old photo").unwrap();
        drop(file);
        
        // Set very old modification time
        let ancient_time = SystemTime::now() - std::time::Duration::from_secs(15 * 365 * 24 * 60 * 60);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(ancient_time)).unwrap();
        
        let config = create_test_config(
            source_dir,
            target_dir.clone(),
            true, // preserve structure
            Some(log_file.clone()),
            false,
        );
        
        let mut organizer = PhotoOrganizer::new(config).unwrap();
        let result = organizer.run().await;
        assert!(result.is_ok());
        
        // Recent file should be organized by date
        let recent_files: Vec<_> = walkdir::WalkDir::new(&target_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap_or("") == "recent.jpg")
            .collect();
        assert_eq!(recent_files.len(), 1);
        
        // Old file should be in undated directory preserving structure
        let old_target = target_dir.join("undated").join("old_photos").join("old.jpg");
        assert!(old_target.exists());
        
        // Text log should show both types
        let log_content = fs::read_to_string(&log_file).unwrap();
        assert!(log_content.contains("✓")); // reliable file
        assert!(log_content.contains("⚠️")); // unreliable file
        assert!(log_content.contains("Files with unreliable dates: 1"));
    }
    
    #[tokio::test]
    async fn test_date_inference_from_directory_peers() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let log_file = temp_dir.path().join("inference.txt");
        
        // Create directory structure with mixed file types
        fs::create_dir_all(source_dir.join("vacation_photos")).unwrap();
        fs::create_dir_all(source_dir.join("no_exif_photos")).unwrap();
        
        // Create files - we'll simulate the behavior by having one file with recent timestamp
        // (which will be reliable) and one with old timestamp (unreliable)
        let recent_file = source_dir.join("vacation_photos").join("IMG_001.jpg");
        let old_file = source_dir.join("vacation_photos").join("document.txt");
        let isolated_file = source_dir.join("no_exif_photos").join("scan.jpg");
        
        // Create the recent file - this will have a reliable date (recent mod time)
        let mut file = File::create(&recent_file).unwrap();
        writeln!(file, "recent photo").unwrap();
        drop(file);
        
        // Create old files
        let mut file = File::create(&old_file).unwrap();
        writeln!(file, "old document").unwrap();
        drop(file);
        
        let mut file = File::create(&isolated_file).unwrap();
        writeln!(file, "isolated photo").unwrap();
        drop(file);
        
        // Set old timestamps on files we want to be unreliable
        let ancient_time = SystemTime::now() - std::time::Duration::from_secs(15 * 365 * 24 * 60 * 60);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(ancient_time)).unwrap();
        filetime::set_file_mtime(&isolated_file, filetime::FileTime::from_system_time(ancient_time)).unwrap();
        
        let config = create_test_config(
            source_dir,
            target_dir.clone(),
            true, // preserve structure
            Some(log_file.clone()),
            false,
        );
        
        let mut organizer = PhotoOrganizer::new(config).unwrap();
        let result = organizer.run().await;
        assert!(result.is_ok());
        
        // Both the recent file and old file in vacation_photos should be organized by date
        // The old file should get its date inferred from the recent file
        let vacation_files: Vec<_> = walkdir::WalkDir::new(&target_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_str().unwrap_or("");
                name.contains("IMG_001") || name.contains("document")
            })
            .filter(|e| !e.path().to_string_lossy().contains("undated"))
            .collect();
        
        // Both files should be found in dated directories (not undated)
        assert_eq!(vacation_files.len(), 2, "Both vacation_photos files should be in dated directories");
        
        // The isolated scan should be in undated (no peers with reliable dates)
        let scan_target = target_dir.join("undated").join("no_exif_photos").join("scan.jpg");
        assert!(scan_target.exists(), "Isolated file should be in undated directory");
        
        // Check text log shows inferred date
        let log_content = fs::read_to_string(&log_file).unwrap();
        assert!(log_content.contains("[INFERRED]"), "Log should show inferred dates");
    }
}
