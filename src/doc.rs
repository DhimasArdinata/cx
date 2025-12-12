use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn generate_docs() -> Result<()> {
    println!("{} Generating documentation...", "📚".magenta());

    // 1. Check for Doxygen
    if Command::new("doxygen").arg("--version").output().is_err() {
        println!("{} Doxygen not found. Please install it first.", "x".red());
        return Ok(());
    }

    // 2. Create default Doxyfile if not exists
    if !Path::new("Doxyfile").exists() {
        println!("   Creating default Doxyfile...");
        let project_name = std::env::current_dir()?
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let doxy_content = format!(
            r#"PROJECT_NAME           = "{}"
OUTPUT_DIRECTORY       = docs
INPUT                  = src
RECURSIVE              = YES
GENERATE_HTML          = YES
GENERATE_LATEX         = NO
OPTIMIZE_OUTPUT_FOR_C  = YES
EXTRACT_ALL            = YES
"#,
            project_name
        );
        fs::write("Doxyfile", doxy_content)?;
    }

    // 3. Run Doxygen
    let status = Command::new("doxygen").status()?;

    if status.success() {
        println!(
            "{} Documentation generated in docs/html/index.html",
            "✓".green()
        );
    } else {
        println!("{} Doxygen failed.", "x".red());
    }

    Ok(())
}
