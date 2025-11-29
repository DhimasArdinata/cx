use anyhow::{Context, Result};
use colored::*;
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;

pub fn fetch_dependencies(deps: &HashMap<String, String>) -> Result<Vec<String>> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let cache_dir = home_dir.join(".cx").join("cache");
    fs::create_dir_all(&cache_dir)?;

    let mut include_paths = Vec::new();

    if !deps.is_empty() {
        println!("{} Checking {} dependencies...", "📦".blue(), deps.len());
    }

    for (name, url) in deps {
        let lib_path = cache_dir.join(name);

        if !lib_path.exists() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")?
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""]),
            );

            pb.set_message(format!("Downloading {}...", name));
            pb.enable_steady_tick(std::time::Duration::from_millis(100)); // Update tiap 100ms

            match Repository::clone(url, &lib_path) {
                Ok(_) => {
                    pb.finish_with_message(format!("{} Downloaded {}", "✓".green(), name));
                }
                Err(e) => {
                    pb.finish_with_message(format!("{} Failed {}", "x".red(), name));
                    println!("Error details: {}", e);
                    continue;
                }
            }
        } else {
            println!("   {} Using cached: {}", "⚡".green(), name);
        }

        include_paths.push(format!("-I{}", lib_path.display()));
        include_paths.push(format!("-I{}/include", lib_path.display()));
        include_paths.push(format!("-I{}/src", lib_path.display()));
    }

    Ok(include_paths)
}
