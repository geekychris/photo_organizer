use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use exif::{In, Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct ExifExtractor;

impl ExifExtractor {
    pub fn extract_date_taken(path: &Path) -> Result<(DateTime<Utc>, bool, bool)> {
        // Try to extract EXIF data first
        match Self::extract_from_exif(path) {
            Ok(date) => Ok((date, true, true)), // EXIF date is always reliable
            Err(_) => {
                // Fallback to file modification time
                let metadata = std::fs::metadata(path)
                    .context("Failed to get file metadata")?;
                
                let modified = metadata.modified()
                    .context("Failed to get file modification time")?;
                
                let datetime: DateTime<Utc> = modified.into();
                
                // Check if file timestamp seems reasonable (not too old or future)
                let now = Utc::now();
                let years_ago_10 = now - Duration::days(365 * 10);
                let future_limit = now + Duration::days(30);
                
                let reliable = datetime > years_ago_10 && datetime < future_limit;
                
                Ok((datetime, false, reliable))
            }
        }
    }
    
    fn extract_from_exif(path: &Path) -> Result<DateTime<Utc>> {
        let file = File::open(path)
            .context("Failed to open file for EXIF reading")?;
        
        let mut buf_reader = BufReader::new(file);
        let exif_reader = Reader::new();
        let exif = exif_reader.read_from_container(&mut buf_reader)
            .context("Failed to read EXIF data")?;
        
        // Try different EXIF date fields in order of preference
        let date_fields = [
            Tag::DateTimeOriginal,
            Tag::DateTime,
            Tag::DateTimeDigitized,
        ];
        
        for &tag in &date_fields {
            if let Some(field) = exif.get_field(tag, In::PRIMARY) {
                if let Some(date_str) = Self::field_to_string(field) {
                    if let Ok(datetime) = Self::parse_exif_date(&date_str) {
                        return Ok(datetime);
                    }
                }
            }
        }
        
        anyhow::bail!("No valid EXIF date found")
    }
    
    fn field_to_string(field: &exif::Field) -> Option<String> {
        match &field.value {
            exif::Value::Ascii(vec) => {
                if let Some(bytes) = vec.first() {
                    String::from_utf8(bytes.clone()).ok()
                } else {
                    None
                }
            },
            _ => Some(field.display_value().to_string()),
        }
    }
    
    fn parse_exif_date(date_str: &str) -> Result<DateTime<Utc>> {
        // EXIF date format: "YYYY:MM:DD HH:MM:SS"
        let cleaned = date_str.trim().replace(':', "-");
        
        // Try different date formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y:%m:%d %H:%M:%S",
        ];
        
        let date_only_formats = [
            "%Y-%m-%d",
            "%Y:%m:%d",
        ];
        
        // Try full datetime formats first
        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&cleaned, format) {
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
            
            // Try with original string
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
        }
        
        // Try date-only formats (assume midnight UTC)
        for format in &date_only_formats {
            if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(&cleaned, format) {
                let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
            
            // Try with original string
            if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, format) {
                let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
        }
        
        anyhow::bail!("Could not parse EXIF date: {}", date_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    
    #[test]
    fn test_parse_exif_date() {
        let test_cases = vec![
            ("2023:01:15 14:30:45", true),
            ("2023-01-15 14:30:45", true),
            ("2023:01:15", true),
            ("invalid_date", false),
        ];
        
        for (input, should_succeed) in test_cases {
            let result = ExifExtractor::parse_exif_date(input);
            assert_eq!(result.is_ok(), should_succeed, "Failed for input: {}", input);
        }
    }
    
    #[test]
    fn test_fallback_to_file_time() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test content").unwrap();
        
        let result = ExifExtractor::extract_date_taken(&file_path).unwrap();
        // Should fallback to file time since it's not an image
        assert!(!result.1); // exif_date_available should be false
        assert!(result.2); // date should be reliable for recent file
    }
}