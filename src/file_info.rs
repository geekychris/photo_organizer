use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileInfo {
    pub source_path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub file_size: u64,
    pub file_hash: String,
    pub date_taken: DateTime<Utc>,
    pub exif_date_available: bool,
    pub date_reliable: bool, // true if from EXIF or file timestamp is reasonable
    pub date_inferred: bool, // true if date was inferred from directory peers
    pub directory_context: String,
    pub target_path: Option<PathBuf>,
    pub processed: bool,
}

impl FileInfo {
    pub fn new(
        source_path: PathBuf,
        file_size: u64,
        file_hash: String,
        date_taken: DateTime<Utc>,
        exif_date_available: bool,
        date_reliable: bool,
    ) -> Self {
        let file_name = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
            
        let directory_context = source_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            source_path,
            file_name,
            extension,
            file_size,
            file_hash,
            date_taken,
            exif_date_available,
            date_reliable,
            date_inferred: false,
            directory_context,
            target_path: None,
            processed: false,
        }
    }
    
    pub fn get_date_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "{:02}/{:02}/{:02}",
            self.date_taken.year() % 100,
            self.date_taken.month(),
            self.date_taken.day()
        ))
    }
    
    pub fn get_source_preserved_path(&self, source_root: &std::path::Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Get relative path from source root
        let relative_path = self.source_path.strip_prefix(source_root)?;
        
        // Remove the filename to get just the directory structure
        if let Some(parent) = relative_path.parent() {
            Ok(PathBuf::from("undated").join(parent))
        } else {
            Ok(PathBuf::from("undated"))
        }
    }
    
    pub fn is_interesting_directory(&self) -> bool {
        let context = self.directory_context.to_lowercase();
        // Check if directory name indicates interesting content
        context.contains("wedding") ||
        context.contains("vacation") ||
        context.contains("birthday") ||
        context.contains("christmas") ||
        context.contains("holiday") ||
        context.contains("trip") ||
        context.contains("event") ||
        context.contains("party") ||
        context.len() > 20 || // Long descriptive names
        context.chars().any(|c| c == '_' || c == '-') // Names with separators
    }
    
    pub fn infer_date_from_peers(&mut self, peer_files: &[FileInfo]) -> bool {
        // Only infer if current file doesn't have a reliable date
        if self.date_reliable {
            return false;
        }
        
        // Find files in the same directory with EXIF dates
        let same_dir_files_with_exif: Vec<_> = peer_files.iter()
            .filter(|f| {
                f.exif_date_available && 
                f.source_path.parent() == self.source_path.parent()
            })
            .collect();
        
        if same_dir_files_with_exif.is_empty() {
            return false; // No EXIF dates available in directory
        }
        
        // Use the most common date from EXIF files, or the first if all different
        if let Some(reference_file) = same_dir_files_with_exif.first() {
            self.date_taken = reference_file.date_taken;
            self.date_reliable = true;
            self.date_inferred = true;
            return true;
        }
        
        false
    }
}