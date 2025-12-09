use anyhow::{Context, Result};
use colored::*;
use std::fs;

pub fn print_path() -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let cache_dir = home.join(".cx").join("cache");
    println!("{}", cache_dir.display());
    Ok(())
}

pub fn list() -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let cache_dir = home.join(".cx").join("cache");
    
    if !cache_dir.exists() {
         println!("{} Cache is empty.", "ℹ".blue());
         return Ok(());
    }

    let entries = fs::read_dir(&cache_dir)?;
    let mut count = 0;
    println!("{}", "Cached Libraries:".blue().bold());
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name();
                    println!("  - {}", name.to_string_lossy());
                    count += 1;
                }
            }
        }
    }
    
    if count == 0 {
         println!("  (empty)");
    }
    
    Ok(())
}

pub fn clean() -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let cache_dir = home.join(".cx").join("cache");
    
    if cache_dir.exists() {
        println!("{} Cleaning cache...", "🧹".yellow());
        fs::remove_dir_all(&cache_dir)?;
        fs::create_dir_all(&cache_dir)?;
        println!("{} Cache cleaned.", "✓".green());
    } else {
        println!("{} Cache already empty.", "✓".green());
    }
    Ok(())
}
