use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{Select, Text};
use std::fs;
use std::path::Path;

mod build;
mod cache;
mod checker;
mod config;
mod deps;
mod doc;
mod lock;
mod registry;
mod templates;
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
}

#[derive(Subcommand)]
enum CacheOp {
    Clean,
    Ls,
    Path,
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
        .unwrap_or_default()
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
    let (toml_content, main_code) = templates::get_template(&name, lang, template);

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

    println!("\n{}", "Toolchain Check:".bold());
    let compilers = vec![
        ("clang++", "LLVM C++"),
        ("g++", "GNU C++"),
        ("gcc", "GNU C"),
        ("cl", "MSVC"),
        ("cmake", "CMake"),
        ("make", "Make"),
    ];

    for (bin, name) in compilers {
        let output = std::process::Command::new(bin).arg("--version").output();
        let (status, version) = match output {
            Ok(out) => {
                let v_str = String::from_utf8_lossy(&out.stdout);
                let first_line = v_str.lines().next().unwrap_or("Detected").trim();
                let short_ver = if first_line.len() > 40 {
                    &first_line[..40]
                } else {
                    first_line
                };
                ("✓".green(), short_ver.to_string())
            }
            Err(_) => ("x".red(), "Not Found".dimmed().to_string()),
        };
        println!("  [{}] {:<10} : ({}) {}", status, bin, name, version);
    }

    Ok(())
}
