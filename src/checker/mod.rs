use crate::build::load_config;
use crate::deps;
use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::process::Command;
use walkdir::WalkDir;

pub fn format_code() -> Result<()> {
    if Command::new("clang-format")
        .arg("--version")
        .output()
        .is_err()
    {
        println!(
            "{} clang-format not found. Please install it first.",
            "x".red()
        );
        return Ok(());
    }

    println!("{} Formatting source code...", "🎨".magenta());

    let mut files = Vec::new();
    for entry in WalkDir::new("src").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_path_buf();
        if let Some(ext) = path.extension() {
            let s = ext.to_string_lossy();
            if ["cpp", "hpp", "c", "h", "cc", "cxx"].contains(&s.as_ref()) {
                files.push(path);
            }
        }
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut count = 0;
    for path in files {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        pb.set_message(format!("Formatting {}", name));

        let status = Command::new("clang-format")
            .arg("-i")
            .arg("-style=file")
            .arg(&path)
            .status();

        if let Ok(s) = status {
            if s.success() {
                count += 1;
            }
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    println!("{} Formatted {} files.", "✓".green(), count);
    Ok(())
}

pub fn check_code() -> Result<()> {
    if Command::new("clang-tidy")
        .arg("--version")
        .output()
        .is_err()
    {
        println!(
            "{} clang-tidy not found. Please install it first.",
            "x".red()
        );
        return Ok(());
    }

    println!("{} Checking code with clang-tidy...", "🔍".magenta());

    let config = load_config()?;

    // Fetch dependencies for include paths
    let mut include_flags = Vec::new();
    if let Some(deps) = &config.dependencies {
        if !deps.is_empty() {
            if let Ok((paths, cflags, _)) = deps::fetch_dependencies(deps) {
                for p in paths {
                    include_flags.push(format!("-I{}", p.display()));
                }
                include_flags.extend(cflags);
            }
        }
    }

    let mut files = Vec::new();
    for entry in WalkDir::new("src").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_path_buf();
        if let Some(ext) = path.extension() {
            let s = ext.to_string_lossy();
            if ["cpp", "hpp", "c", "h", "cc", "cxx"].contains(&s.as_ref()) {
                files.push(path);
            }
        }
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let warnings: usize = files
        .par_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            pb.set_message(format!("Checking {}", name));

            let mut cmd = Command::new("clang-tidy");
            cmd.arg(path);
            cmd.arg("--");
            cmd.arg(format!("-std={}", config.package.edition));

            if let Some(build_cfg) = &config.build {
                if let Some(flags) = &build_cfg.cflags {
                    cmd.args(flags);
                }
            }
            cmd.args(&include_flags);

            // Execute clang-tidy
            let output = cmd.output().ok(); // Handle potential execution failure gracefully

            if let Some(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let has_issues = stdout.contains("warning:")
                    || stdout.contains("error:")
                    || !out.status.success();

                if has_issues {
                    pb.suspend(|| {
                        println!("{} Issues in {}", "!".yellow(), name);
                        if !stdout.is_empty() {
                            println!("{}", stdout.trim());
                        }
                        if !stderr.is_empty() {
                            println!("{}", stderr.trim());
                        }
                        println!("{}", "-".repeat(40).dimmed());
                    });
                    pb.inc(1);
                    return 1;
                }
            }

            pb.inc(1);
            0
        })
        .sum();

    pb.finish_and_clear();

    if warnings == 0 {
        println!(
            "{} Checked {} files. No issues found.",
            "✓".green(),
            files.len()
        );
    } else {
        println!(
            "{} Checked {} files. Found issues in {} files.",
            "!".yellow(),
            files.len(),
            warnings
        );
    }

    Ok(())
}
