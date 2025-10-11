use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "photo-organizer")]
#[command(about = "A tool to organize photos by date using EXIF data")]
#[command(version = "1.0")]
pub struct Config {
    #[arg(short, long, help = "Source directory containing photos to organize")]
    pub source: PathBuf,
    
    #[arg(short, long, help = "Target directory for organized photos")]
    pub target: PathBuf,
    
    #[arg(long, help = "CSV log file path", default_value = "photo_organizer.csv")]
    pub log_file: PathBuf,
    
    #[arg(long, help = "State file for resumable operations", default_value = "photo_organizer_state.json")]
    pub state_file: PathBuf,
    
    #[arg(long, help = "Dry run mode - show what would be done without making changes")]
    pub dry_run: bool,
    
    #[arg(long, help = "Force re-processing of all files")]
    pub force_reprocess: bool,
    
    #[arg(long, help = "Number of worker threads", default_value = "4")]
    pub threads: usize,
    
    #[arg(long, help = "Supported image extensions", default_values_t = vec!["jpg".to_string(), "jpeg".to_string(), "raw".to_string(), "cr2".to_string(), "cr3".to_string(), "nef".to_string(), "arw".to_string(), "dng".to_string(), "tiff".to_string(), "tif".to_string()])]
    pub extensions: Vec<String>,
    
    #[arg(long, help = "Preserve source directory structure for files without determinable dates")]
    pub preserve_structure: bool,
    
    #[arg(long, help = "Text log file path for simple source->target mapping")]
    pub text_log_file: Option<PathBuf>,
    
    #[arg(long, help = "Log-only mode - don't copy files, just log what would happen")]
    pub log_only: bool,
}

impl Config {
    pub fn is_supported_extension(&self, extension: &str) -> bool {
        self.extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension))
    }
}