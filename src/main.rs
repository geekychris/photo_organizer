mod config;
mod date_inference;
mod file_info;
mod exif_extractor;
mod scanner;
mod deduplicator;
mod organizer;
mod logger;
mod progress;
mod state;
mod text_logger;

#[cfg(test)]
mod integration_tests;

use anyhow::Result;
use clap::Parser;
use config::Config;
use env_logger;
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let config = Config::parse();
    info!("Starting photo organizer with config: {:?}", config);
    
    let mut organizer = organizer::PhotoOrganizer::new(config)?;
    organizer.run().await?;
    
    info!("Photo organization completed successfully");
    Ok(())
}
