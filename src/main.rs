use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{Select, Text};
use std::fs;
use std::path::{Path, PathBuf};

mod build;
mod cache;
mod checker;
mod config;
mod deps;
mod doc;
mod lock;
mod registry;
mod templates;
mod toolchain;
mod upgrade;

#[derive(Parser)]
#[command(name = "cx")]
#[command(about = "The modern C/C++ project manager", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        name: Option<String>,
        #[arg(long, default_value = "cpp")]
        lang: String,
        #[arg(long, default_value = "console")]
        template: String,
    },
    Build {
        #[arg(long)]
        release: bool,
    },
    Run {
        #[arg(long)]
        release: bool,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Add {
        lib: String,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        rev: Option<String>,
    },
    Remove {
        lib: String,
    },
    Watch,
    Clean,
    Test,
    Info,
    Fmt,
    Doc,
    Check,
    Update,
    Upgrade,
    Search {
        query: String,
    },
    Init,
    Cache {
        #[command(subcommand)]
        op: CacheOp,
    },
    /// Manage toolchain selection
    Toolchain {
        #[command(subcommand)]
        op: Option<ToolchainOp>,
    },
}

#[derive(Subcommand)]
enum CacheOp {
    Clean,
    Ls,
    Path,
}

#[derive(Subcommand)]
enum ToolchainOp {
    /// List all available toolchains
    List,
    /// Interactively select a toolchain
    Select,
    /// Clear cached toolchain selection
    Clear,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::New {
            name,
            lang,
            template,
        } => create_project(name, lang, template),

        Commands::Search { query } => {
            let results = registry::search(query);
            if results.is_empty() {
                println!("{} No results found for '{}'", "x".red(), query);
            } else {
                println!("{} Found {} results:", "🔍".blue(), results.len());
                for (name, url) in results {
                    println!("  {} - {}", name.bold().green(), url);
                }
            }
            Ok(())
        }

        Commands::Build { release } => {
            let config = build::load_config()?;
            build::build_project(&config, *release).map(|_| ())
        }

        Commands::Run { release, args } => build::build_and_run(*release, args),

        Commands::Watch => build::watch(),
        Commands::Clean => build::clean(),
        Commands::Test => build::run_tests(),
        Commands::Add {
            lib,
            tag,
            branch,
            rev,
        } => deps::add_dependency(lib, tag.clone(), branch.clone(), rev.clone()),
        Commands::Remove { lib } => deps::remove_dependency(lib),
        Commands::Info => print_info(),
        Commands::Fmt => checker::format_code(),
        Commands::Doc => doc::generate_docs(),
        Commands::Check => checker::check_code(),
        Commands::Update => deps::update_dependencies(),
        Commands::Upgrade => upgrade::check_and_upgrade(),
        Commands::Init => init_project(),
        Commands::Cache { op } => match op {
            CacheOp::Clean => cache::clean(),
            CacheOp::Ls => cache::list(),
            CacheOp::Path => cache::print_path(),
        },
        Commands::Toolchain { op } => handle_toolchain_command(op),
    }
}

fn init_project() -> Result<()> {
    // 1. Check existing
    if Path::new("cx.toml").exists() {
        println!(
            "{} Error: Project already initialized (cx.toml exists).",
            "x".red()
        );
        return Ok(());
    }

    // 2. Interactive Inputs
    let current_dir = std::env::current_dir()?;
    let dir_name = current_dir
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("unknown"))
        .to_string_lossy();

    let name = Text::new("Project name?")
        .with_default(&dir_name)
        .prompt()?;

    let lang = Select::new("Language?", vec!["cpp", "c"]).prompt()?;
    let template = Select::new(
        "Template?",
        vec!["console", "web", "raylib", "sdl2", "opengl"],
    )
    .prompt()?;

    let (toml_content, main_code) = templates::get_template(&name, lang, template);

    fs::write("cx.toml", toml_content)?;

    // Create src if generic template
    if !Path::new("src").exists() {
        fs::create_dir("src")?;
        let ext = if lang == "c" { "c" } else { "cpp" };
        fs::write(Path::new("src").join(format!("main.{}", ext)), main_code)?;
    } else {
        println!(
            "{} 'src' directory exists, skipping main file creation.",
            "!".yellow()
        );
    }

    // Write .gitignore if not exists
    if !Path::new(".gitignore").exists() {
        fs::write(".gitignore", "/build\n/compile_commands.json\n")?;
    }

    println!(
        "{} Initialized caxe project in current directory.",
        "✓".green()
    );
    Ok(())
}

fn create_project(name_opt: &Option<String>, lang_cli: &str, templ_cli: &str) -> Result<()> {
    // 1. Interactive Inputs
    let name = match name_opt {
        Some(n) => n.clone(),
        None => Text::new("What is your project name?")
            .with_default("my-app")
            .prompt()?,
    };

    let template = if name_opt.is_none() {
        let options = vec!["console", "web", "raylib", "sdl2", "opengl"];
        Select::new("Select a template:", options).prompt()?
    } else {
        templ_cli
    };

    let lang = if name_opt.is_none() {
        let options = vec!["cpp", "c"];
        Select::new("Select language:", options).prompt()?
    } else {
        lang_cli
    };

    // 2. Setup Directory
    let path = Path::new(&name);
    if path.exists() {
        println!("{} Error: Directory '{}' already exists", "x".red(), name);
        return Ok(());
    }

    fs::create_dir_all(path.join("src")).context("Failed to create src")?;

    // 3. Get Template Content (Refactored)
    // Use only the final directory name as project name (not the full path)
    let project_name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    let (toml_content, main_code) = templates::get_template(&project_name, lang, template);

    // 4. Write Files
    let ext = if lang == "c" { "c" } else { "cpp" };
    fs::write(path.join("cx.toml"), toml_content)?;
    fs::write(path.join("src").join(format!("main.{}", ext)), main_code)?;
    fs::write(path.join(".gitignore"), "/build\n/compile_commands.json\n")?;

    // 5. VS Code Intellisense Support
    let vscode_dir = path.join(".vscode");
    fs::create_dir_all(&vscode_dir).context("Failed to create .vscode dir")?;

    let vscode_json = r#"{
    "configurations": [
        {
            "name": "cx-config",
            "includePath": ["${workspaceFolder}/**"],
            "compileCommands": "${workspaceFolder}/compile_commands.json",
            "cStandard": "c17",
            "cppStandard": "c++17"
        }
    ],
    "version": 4
}"#;
    fs::write(vscode_dir.join("c_cpp_properties.json"), vscode_json)?;

    // 6. Success Message
    println!(
        "{} Created new project: {} (template: {})",
        "✓".green(),
        name.bold(),
        template.cyan()
    );
    println!("  cd {}\n  cx run", name);
    Ok(())
}

fn print_info() -> Result<()> {
    println!("{} v{}", "caxe".bold().cyan(), env!("CARGO_PKG_VERSION"));
    println!("The Modern C/C++ Project Manager 🪓");
    println!("------------------------------------");

    // System Info
    println!(
        "{}: {} {}",
        "System".bold(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Cache Info
    let home = dirs::home_dir().unwrap_or_default();
    println!(
        "{}: {}",
        "Cache".bold(),
        home.join(".cx").join("cache").display()
    );

    println!("\n{}", "Available Toolchains:".bold());

    #[cfg(windows)]
    {
        use toolchain::CompilerType;
        use toolchain::windows::discover_all_toolchains;

        let toolchains = discover_all_toolchains();
        if toolchains.is_empty() {
            println!("  {} No toolchains found!", "x".red());
            println!("  Install Visual Studio Build Tools or LLVM to get started.");
        } else {
            // Check project's cx.toml for compiler preference
            let project_compiler = if Path::new("cx.toml").exists() {
                if let Ok(config) = build::load_config() {
                    config.build.and_then(|b| b.compiler)
                } else {
                    None
                }
            } else {
                None
            };

            // Determine which compiler type is configured
            let configured_type = match project_compiler.as_deref() {
                Some("msvc") | Some("cl") | Some("cl.exe") => Some(CompilerType::MSVC),
                Some("clang-cl") | Some("clangcl") => Some(CompilerType::ClangCL),
                Some("clang") | Some("clang++") => Some(CompilerType::Clang),
                Some("gcc") | Some("g++") => Some(CompilerType::GCC),
                _ => None, // No preference = use default (first)
            };

            // Find which one is in use
            let in_use_idx = match &configured_type {
                Some(ct) => toolchains.iter().position(|tc| tc.compiler_type == *ct),
                None => Some(0), // Default is first
            };

            for (i, tc) in toolchains.iter().enumerate() {
                let is_in_use = in_use_idx == Some(i);
                let status = "✓".green();
                let short_ver = if tc.version.len() > 40 {
                    format!("{}...", &tc.version[..40])
                } else {
                    tc.version.clone()
                };
                let marker = if is_in_use {
                    " ← in use".green().bold()
                } else {
                    "".normal()
                };
                println!(
                    "  [{}] {} {} {} - {}{}",
                    status,
                    format!("[{}]", i + 1).dimmed(),
                    tc.display_name.cyan(),
                    format!("({})", short_ver).dimmed(),
                    tc.source.yellow(),
                    marker
                );
            }

            // Show current ABI and config source
            println!();
            println!("{}", "Current Configuration:".bold());

            let active_tc = in_use_idx.and_then(|i| toolchains.get(i));
            if let Some(tc) = active_tc {
                println!(
                    "  {}: {} ({})",
                    "Compiler".bold(),
                    tc.display_name.cyan(),
                    tc.source
                );
                let abi = if tc.path.to_string_lossy().contains("x64")
                    || tc.path.to_string_lossy().contains("Hostx64")
                {
                    "x86_64 (64-bit)"
                } else if tc.path.to_string_lossy().contains("x86")
                    || tc.path.to_string_lossy().contains("Hostx86")
                {
                    "x86 (32-bit)"
                } else {
                    "x86_64 (64-bit)"
                };
                println!("  {}: {}", "Target ABI".bold(), abi.cyan());
            }
            println!(
                "  {}: Set {} in cx.toml to override",
                "Override".bold(),
                "compiler = \"...\"".yellow()
            );
        }

        // Build tools check (cmake, make, etc.)
        println!("\n{}", "Build Tools:".bold());
        let tools = vec![("cmake", "CMake"), ("make", "Make"), ("ninja", "Ninja")];
        for (bin, name) in tools {
            let output = std::process::Command::new(bin).arg("--version").output();
            let (status, version) = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let first_line = stdout.lines().next().unwrap_or("Detected").trim();
                    let short = if first_line.len() > 40 {
                        &first_line[..40]
                    } else {
                        first_line
                    };
                    ("✓".green(), short.to_string())
                }
                _ => ("x".red(), "Not Found".dimmed().to_string()),
            };
            println!("  [{}] {:<10} : {}", status, name, version);
        }
    }

    #[cfg(not(windows))]
    {
        // Unix fallback - check PATH
        let compilers = vec![("clang++", "LLVM C++"), ("g++", "GNU C++")];

        for (bin, name) in compilers {
            let output = std::process::Command::new(bin).arg("--version").output();
            let (status, version) = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let first_line = stdout.lines().next().unwrap_or("Detected").trim();
                    ("✓".green(), first_line.to_string())
                }
                _ => ("x".red(), "Not Found".dimmed().to_string()),
            };
            println!("  [{}] {:<10} : ({}) {}", status, bin, name, version);
        }
    }

    Ok(())
}

fn handle_toolchain_command(op: &Option<ToolchainOp>) -> Result<()> {
    #[cfg(windows)]
    {
        use toolchain::windows::discover_all_toolchains;

        match op {
            Some(ToolchainOp::List) => {
                // Just redirect to cx info
                println!(
                    "{}",
                    "Use 'cx info' to see all available toolchains".yellow()
                );
                println!("Or run 'cx toolchain' to select one interactively.");
            }

            None | Some(ToolchainOp::Select) => {
                // Interactive selection (default behavior)
                let toolchains = discover_all_toolchains();
                if toolchains.is_empty() {
                    println!("{} No toolchains found!", "x".red());
                    println!("  Install Visual Studio Build Tools or LLVM to get started.");
                    return Ok(());
                }

                // Format options for display
                let options: Vec<String> = toolchains.iter().map(|tc| tc.to_string()).collect();

                let selection = Select::new("Select a toolchain:", options).prompt()?;

                // Find the selected toolchain
                let selected = toolchains.iter().find(|tc| tc.to_string() == selection);

                if let Some(tc) = selected {
                    // Cache the selection
                    let cache_path = dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".cx")
                        .join("toolchain-selection.toml");

                    let content = format!(
                        "# User-selected toolchain\ncompiler_type = {:?}\npath = {:?}\nversion = {:?}\nsource = {:?}\n",
                        format!("{:?}", tc.compiler_type),
                        tc.path.display(),
                        tc.version,
                        tc.source
                    );

                    if let Err(e) = std::fs::create_dir_all(cache_path.parent().unwrap()) {
                        println!("{} Failed to create cache dir: {}", "x".red(), e);
                    } else if let Err(e) = std::fs::write(&cache_path, content) {
                        println!("{} Failed to save selection: {}", "x".red(), e);
                    } else {
                        println!();
                        println!(
                            "{} Selected: {} ({})",
                            "✓".green(),
                            tc.display_name.cyan(),
                            tc.source.yellow()
                        );
                        println!("  Saved to: {}", cache_path.display().to_string().dimmed());
                    }
                }
            }

            Some(ToolchainOp::Clear) => {
                // Clear cached selection
                let cache_path = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cx")
                    .join("toolchain-selection.toml");

                if cache_path.exists() {
                    if let Err(e) = std::fs::remove_file(&cache_path) {
                        println!("{} Failed to clear selection: {}", "x".red(), e);
                    } else {
                        println!("{} Cleared toolchain selection", "✓".green());
                    }
                } else {
                    println!("{} No toolchain selection cached", "ℹ".blue());
                }

                // Also clear the toolchain cache
                toolchain::clear_toolchain_cache();
                println!("{} Cleared toolchain cache", "✓".green());
            }
        }
    }

    #[cfg(not(windows))]
    {
        println!(
            "{} Toolchain selection is currently Windows-only",
            "ℹ".blue()
        );
        println!("  On Unix, the default clang++ or g++ from PATH is used.");
    }

    Ok(())
}
