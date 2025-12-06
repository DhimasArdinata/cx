use crate::config::Dependency;
use anyhow::{Context, Result};
use colored::*;
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn fetch_dependencies(
    deps: &HashMap<String, Dependency>,
) -> Result<(Vec<String>, Vec<String>)> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let cache_dir = home_dir.join(".cx").join("cache");
    fs::create_dir_all(&cache_dir)?;

    let mut include_flags = Vec::new();
    let mut link_flags = Vec::new();

    if !deps.is_empty() {
        println!("{} Checking {} dependencies...", "📦".blue(), deps.len());
    }

    for (name, dep_data) in deps {
        let lib_path = cache_dir.join(name);
        let url = dep_data.get_url();

        if !lib_path.exists() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}")?);
            pb.set_message(format!("Downloading {}...", name));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            match Repository::clone(&url, &lib_path) {
                Ok(_) => pb.finish_with_message(format!("{} Downloaded {}", "✓".green(), name)),
                Err(e) => {
                    pb.finish_with_message(format!("{} Failed {}", "x".red(), name));
                    println!("Error: {}", e);
                    continue;
                }
            }
        } else {
            println!("   {} Using cached: {}", "⚡".green(), name);
        }

        if let Dependency::Complex { build, output, .. } = dep_data {
            if let Some(cmd_str) = build {
                let output_file = output.as_deref().unwrap_or("");
                let should_build = if !output_file.is_empty() {
                    !lib_path.join(output_file).exists()
                } else {
                    true
                };

                if should_build {
                    println!(
                        "   {} Building {} (Script: '{}')...",
                        "🔨".yellow(),
                        name,
                        cmd_str
                    );
                    let status = if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(&["/C", cmd_str])
                            .current_dir(&lib_path)
                            .status()?
                    } else {
                        Command::new("sh")
                            .args(&["-c", cmd_str])
                            .current_dir(&lib_path)
                            .status()?
                    };

                    if !status.success() {
                        println!("{} Build script failed for {}", "x".red(), name);
                        continue;
                    }
                }
            }

            if let Some(out_file) = output {
                let full_lib_path = lib_path.join(out_file);
                if full_lib_path.exists() {
                    link_flags.push(full_lib_path.to_string_lossy().to_string());
                } else {
                    println!(
                        "{} Warning: Output file not found: {}",
                        "!".yellow(),
                        full_lib_path.display()
                    );
                }
            }
        }

        include_flags.push(format!("-I{}", lib_path.display()));
        include_flags.push(format!("-I{}/include", lib_path.display()));
        include_flags.push(format!("-I{}/src", lib_path.display()));
    }

    Ok((include_flags, link_flags))
}

pub fn add_dependency(lib_input: &str) -> Result<()> {
    if !Path::new("cx.toml").exists() {
        println!("{} Error: cx.toml not found.", "x".red());
        return Ok(());
    }

    let (name, url) = if lib_input.contains("http") || lib_input.contains("git@") {
        let name = lib_input
            .split('/')
            .last()
            .unwrap_or("unknown")
            .replace(".git", "");
        (name, lib_input.to_string())
    } else {
        let parts: Vec<&str> = lib_input.split('/').collect();
        if parts.len() != 2 {
            println!("{} Invalid format. Use 'user/repo' or full URL.", "x".red());
            return Ok(());
        }
        let name = parts[1].to_string();
        let url = format!("https://github.com/{}.git", lib_input);
        (name, url)
    };

    println!("{} Adding dependency: {}...", "📦".blue(), name.bold());

    let config_str = fs::read_to_string("cx.toml")?;
    let mut config: crate::config::CxConfig = toml::from_str(&config_str)?;

    if config.dependencies.is_none() {
        config.dependencies = Some(HashMap::new());
    }

    if let Some(deps) = &mut config.dependencies {
        if deps.contains_key(&name) {
            println!("{} Dependency '{}' already exists.", "!".yellow(), name);
            return Ok(());
        }
        deps.insert(name.clone(), crate::config::Dependency::Simple(url));
    }

    let new_toml = toml::to_string_pretty(&config)?;
    fs::write("cx.toml", new_toml)?;

    println!("{} Added {} to cx.toml", "✓".green(), name);

    if let Some(deps) = &config.dependencies {
        let _ = fetch_dependencies(deps)?;
    }

    Ok(())
}

pub fn remove_dependency(name: &str) -> Result<()> {
    if !Path::new("cx.toml").exists() {
        println!("{} Error: cx.toml not found.", "x".red());
        return Ok(());
    }

    let config_str = fs::read_to_string("cx.toml")?;
    let mut config: crate::config::CxConfig = toml::from_str(&config_str)?;

    let mut found = false;
    if let Some(deps) = &mut config.dependencies {
        if deps.remove(name).is_some() {
            found = true;
        }
    }

    if found {
        let new_toml = toml::to_string_pretty(&config)?;
        fs::write("cx.toml", new_toml)?;
        println!("{} Removed dependency: {}", "🗑️".red(), name.bold());
    } else {
        println!(
            "{} Dependency '{}' not found in cx.toml",
            "!".yellow(),
            name
        );
    }

    Ok(())
}
