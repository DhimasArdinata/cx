use crate::config::CxConfig;
use crate::toolchain::{self, CompilerType, Toolchain, ToolchainError};
use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

// --- Helper: Load Config Once ---
pub fn load_config() -> Result<CxConfig> {
    if !Path::new("cx.toml").exists() {
        return Err(anyhow::anyhow!("cx.toml not found"));
    }
    let config_str = fs::read_to_string("cx.toml")?;
    toml::from_str(&config_str).context("Failed to parse cx.toml")
}

// --- Helper: Check if a command exists (for fallback only) ---
fn is_command_available(cmd: &str) -> bool {
    let mut command = Command::new(cmd);
    if cmd == "cl" || cmd == "cl.exe" {
        return command.arg("/?").output().is_ok();
    }
    command.arg("--version").output().is_ok()
}

// --- Helper: Get Toolchain (uses vswhere on Windows) ---
pub fn get_toolchain(config: &CxConfig, _has_cpp: bool) -> Result<Toolchain, ToolchainError> {
    // 1. Check if user specified a compiler in config
    let preferred = if let Some(build) = &config.build {
        if let Some(compiler) = &build.compiler {
            match compiler.to_lowercase().as_str() {
                "msvc" | "cl" | "cl.exe" => Some(CompilerType::MSVC),
                "clang-cl" | "clangcl" => Some(CompilerType::ClangCL),
                "clang" | "clang++" => Some(CompilerType::Clang),
                "gcc" | "g++" => Some(CompilerType::GCC),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    // 2. Try to detect toolchain using proper discovery
    match toolchain::get_or_detect_toolchain(preferred, false) {
        Ok(tc) => {
            println!(
                "   {} Detected toolchain: {} ({})",
                "🔧".cyan(),
                tc.cxx_path.display(),
                tc.version
            );
            Ok(tc)
        }
        Err(e) => {
            // On Windows, show clear error message only for MSVC-related issues
            #[cfg(windows)]
            {
                let msg = format!("{}", e);
                // Don't show VS Install help for intentional non-MSVC compiler preferences
                if !msg.contains("Clang/GCC") {
                    println!("{} {}", "x".red(), e);
                    println!();
                    println!("{}:", "To fix this".bold());
                    println!("  1. Install Visual Studio Build Tools from:");
                    println!("     https://visualstudio.microsoft.com/visual-cpp-build-tools/");
                    println!("  2. Select 'Desktop development with C++' workload");
                    println!();
                }
            }
            Err(e)
        }
    }
}

// --- Helper: Legacy get_compiler for backward compatibility ---
pub fn get_compiler(config: &CxConfig, has_cpp: bool) -> String {
    // Try new toolchain detection first
    if let Ok(tc) = get_toolchain(config, has_cpp) {
        return tc.cxx_path.to_string_lossy().to_string();
    }

    // Fallback to old PATH-based detection (backward compatibility)
    println!(
        "   {} Falling back to PATH-based compiler detection",
        "⚠".yellow()
    );

    // Check Config
    if let Some(build) = &config.build {
        if let Some(compiler) = &build.compiler {
            return compiler.clone();
        }
    }

    // Check Env Vars
    if has_cpp {
        if let Ok(env_cxx) = std::env::var("CXX") {
            return env_cxx;
        }
    } else if let Ok(env_cc) = std::env::var("CC") {
        return env_cc;
    }

    // Auto-Detect from PATH
    if has_cpp {
        if is_command_available("clang++") {
            return "clang++".to_string();
        }
        if is_command_available("g++") {
            return "g++".to_string();
        }
        if cfg!(target_os = "windows") && is_command_available("cl") {
            return "cl".to_string();
        }
        "clang++".to_string()
    } else {
        if is_command_available("clang") {
            return "clang".to_string();
        }
        if is_command_available("gcc") {
            return "gcc".to_string();
        }
        if cfg!(target_os = "windows") && is_command_available("cl") {
            return "cl".to_string();
        }
        "clang".to_string()
    }
}

// --- Helper: Run Script (Cross Platform) ---
pub fn run_script(script: &str, project_dir: &Path) -> Result<()> {
    // Check if script file exists with .rhai extension
    if script.ends_with(".rhai") {
        let script_path = project_dir.join(script);
        if script_path.exists() {
            println!("   {} Running Rhai script: '{}'...", "📜".magenta(), script);
            let engine = rhai::Engine::new();
            engine
                .run_file(script_path)
                .map_err(|e| anyhow::anyhow!("Rhai script failed: {}", e))?;
            return Ok(());
        }
    }

    println!("   {} Running script: '{}'...", "📜".magenta(), script);
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", script])
            .current_dir(project_dir)
            .status()?
    } else {
        Command::new("sh")
            .args(&["-c", script])
            .current_dir(project_dir)
            .status()?
    };

    if !status.success() {
        return Err(anyhow::anyhow!("Script failed"));
    }
    Ok(())
}
