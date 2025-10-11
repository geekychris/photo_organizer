use crate::file_info::FileInfo;
use log::{debug, info};
use std::collections::HashMap;

pub struct DateInference;

impl DateInference {
    /// Performs directory-based date inference on files without reliable dates.
    /// Groups files by directory and infers dates from peers with reliable dates (preferring EXIF dates).
    pub fn infer_dates_from_directories(files: &mut [FileInfo]) -> usize {
        info!("Starting directory-based date inference");
        
        // First pass: collect directory information
        let mut directory_info: HashMap<Option<std::path::PathBuf>, (Vec<usize>, Option<chrono::DateTime<chrono::Utc>>)> = HashMap::new();
        
        for (idx, file) in files.iter().enumerate() {
            let parent_dir = file.source_path.parent().map(|p| p.to_path_buf());
            let entry = directory_info.entry(parent_dir).or_insert_with(|| (Vec::new(), None));
            entry.0.push(idx);
            
            // If this file has a reliable date and we haven't set a reference date yet
            // Prefer EXIF dates over file timestamp dates for reference
            if file.date_reliable && !file.date_inferred {
                if entry.1.is_none() || (file.exif_date_available && entry.1.is_some()) {
                    entry.1 = Some(file.date_taken);
                }
            }
        }
        
        let mut inferred_count = 0;
        
        // Second pass: apply inferred dates
        for (dir_path, (file_indices, reference_date_opt)) in directory_info {
            let dir_name = dir_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("root");
            
            debug!("Processing directory: {}", dir_name);
            
            if let Some(reference_date) = reference_date_opt {
                debug!("Using reference date {:?} for directory: {}", reference_date, dir_name);
                
                // Apply inferred date to files without reliable dates
                for &idx in &file_indices {
                    let file = &mut files[idx];
                    
                    if !file.date_reliable {
                        debug!("Inferring date for file: {}", file.source_path.display());
                        file.date_taken = reference_date;
                        file.date_reliable = true;
                        file.date_inferred = true;
                        inferred_count += 1;
                    }
                }
            } else {
                debug!("No reliable dates found in directory: {}", dir_name);
            }
        }
        
        info!("Date inference completed: {} files had dates inferred", inferred_count);
        inferred_count
    }
    
    /// Gets a representative date from a collection of EXIF dates.
    /// Currently uses the earliest date, but could be enhanced to use median or most common.
    fn get_representative_date(dates: &[chrono::DateTime<chrono::Utc>]) -> chrono::DateTime<chrono::Utc> {
        if dates.is_empty() {
            return chrono::Utc::now(); // Fallback, though this shouldn't happen
        }
        
        // Use the earliest date as the representative date
        // This assumes that if photos were taken over several days,
        // we want to group them with the earliest date
        *dates.iter().min().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::path::PathBuf;
    
    fn create_test_file_info(
        path: &str, 
        has_exif: bool, 
        reliable: bool,
        date: chrono::DateTime<chrono::Utc>
    ) -> FileInfo {
        FileInfo::new(
            PathBuf::from(path),
            1000,
            "hash123".to_string(),
            date,
            has_exif,
            reliable,
        )
    }
    
    #[test]
    fn test_infer_dates_from_directory() {
        let base_date = Utc.with_ymd_and_hms(2023, 6, 15, 10, 0, 0).unwrap();
        let unreliable_date = Utc.with_ymd_and_hms(2010, 1, 1, 0, 0, 0).unwrap(); // Very old
        
        let mut files = vec![
            // Directory 1: vacation_2023 - has one file with EXIF, one without
            create_test_file_info("/photos/vacation_2023/IMG_001.jpg", true, true, base_date),
            create_test_file_info("/photos/vacation_2023/IMG_002.jpg", false, false, unreliable_date),
            
            // Directory 2: work_photos - no EXIF data
            create_test_file_info("/photos/work_photos/scan1.jpg", false, false, unreliable_date),
            create_test_file_info("/photos/work_photos/scan2.jpg", false, false, unreliable_date),
            
            // Directory 3: mixed - multiple EXIF dates
            create_test_file_info("/photos/mixed/photo1.jpg", true, true, base_date),
            create_test_file_info("/photos/mixed/photo2.jpg", true, true, base_date),
            create_test_file_info("/photos/mixed/video.mp4", false, false, unreliable_date),
        ];
        
        let inferred_count = DateInference::infer_dates_from_directories(&mut files);
        
        // Should infer dates for 2 files: IMG_002.jpg and video.mp4
        assert_eq!(inferred_count, 2);
        
        // Check that IMG_002.jpg got the inferred date
        assert!(files[1].date_inferred);
        assert!(files[1].date_reliable);
        assert_eq!(files[1].date_taken, base_date);
        
        // Check that work_photos files still don't have reliable dates (no EXIF in directory)
        assert!(!files[2].date_reliable);
        assert!(!files[3].date_reliable);
        
        // Check that video.mp4 got the inferred date
        assert!(files[6].date_inferred);
        assert!(files[6].date_reliable);
        assert_eq!(files[6].date_taken, base_date);
    }
    
    #[test]
    fn test_get_representative_date() {
        let date1 = Utc.with_ymd_and_hms(2023, 6, 15, 10, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2023, 6, 16, 10, 0, 0).unwrap();
        let date3 = Utc.with_ymd_and_hms(2023, 6, 14, 10, 0, 0).unwrap();
        
        let dates = vec![date1, date2, date3];
        let representative = DateInference::get_representative_date(&dates);
        
        // Should return the earliest date
        assert_eq!(representative, date3);
    }
    
    #[test]
    fn test_no_inference_when_reliable() {
        let base_date = Utc.with_ymd_and_hms(2023, 6, 15, 10, 0, 0).unwrap();
        
        let mut files = vec![
            create_test_file_info("/photos/test/photo1.jpg", true, true, base_date),
            create_test_file_info("/photos/test/photo2.jpg", false, true, base_date), // Already reliable
        ];
        
        let inferred_count = DateInference::infer_dates_from_directories(&mut files);
        
        // Should not infer any dates since photo2 is already reliable
        assert_eq!(inferred_count, 0);
        assert!(!files[1].date_inferred);
    }
}