use anyhow::{Context, Result};
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/dhimasardinata/caxe/main/registry.json";
const CACHE_FILE: &str = "registry.json";
const CACHE_TTL_SECS: u64 = 86400; // 24 hours

#[derive(Deserialize, Debug, Clone)]
pub struct RegistryEntry {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Registry(HashMap<String, RegistryEntry>);

impl Registry {
    pub fn get(name: &str) -> Option<String> {
        let registry = Self::load().unwrap_or_else(|_| Self::default());
        registry.0.get(name).map(|entry| entry.url.clone())
    }

    #[allow(dead_code)]
    pub fn get_entry(name: &str) -> Option<RegistryEntry> {
        let registry = Self::load().unwrap_or_else(|_| Self::default());
        registry.0.get(name).cloned()
    }

    fn default() -> Self {
        // Fallback hardcoded registry
        let mut m = HashMap::new();
        m.insert(
            "raylib".to_string(),
            RegistryEntry {
                url: "https://github.com/raysan5/raylib.git".to_string(),
                description: Some(
                    "A simple and easy-to-use library to enjoy videogames programming".to_string(),
                ),
            },
        );
        m.insert(
            "json".to_string(),
            RegistryEntry {
                url: "https://github.com/nlohmann/json.git".to_string(),
                description: Some("JSON for Modern C++".to_string()),
            },
        );
        m.insert(
            "fmt".to_string(),
            RegistryEntry {
                url: "https://github.com/fmtlib/fmt.git".to_string(),
                description: Some("A modern formatting library".to_string()),
            },
        );
        Self(m)
    }

    fn load() -> Result<Self> {
        let cache_path = Self::get_cache_path()?;

        // 1. Check Cache Validity
        if let Ok(metadata) = fs::metadata(&cache_path)
            && let Ok(modified) = metadata.modified()
                && let Ok(age) = SystemTime::now().duration_since(modified)
                    && age < Duration::from_secs(CACHE_TTL_SECS)
                        && let Ok(content) = fs::read_to_string(&cache_path)
                            && let Ok(reg) =
                                serde_json::from_str::<HashMap<String, RegistryEntry>>(&content)
                            {
                                return Ok(Self(reg));
                            }

        // 2. Fetch from Remote
        print!("{} Fetching registry... ", "⚡".yellow());
        match ureq::get(REGISTRY_URL).call() {
            Ok(mut response) => {
                let content = response.body_mut().read_to_string()?;
                println!("{}", "✓".green());

                // Save to cache
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&cache_path, &content)?;

                let map: HashMap<String, RegistryEntry> =
                    serde_json::from_str(&content).context("Failed to parse registry JSON")?;
                Ok(Self(map))
            }
            Err(_) => {
                println!("{}", "Failed (Using cached/fallback)".red());
                // Try reading cache even if old
                if cache_path.exists() {
                    let content = fs::read_to_string(&cache_path)?;
                    let map: HashMap<String, RegistryEntry> = serde_json::from_str(&content)?;
                    Ok(Self(map))
                } else {
                    Ok(Self::default())
                }
            }
        }
    }

    fn get_cache_path() -> Result<PathBuf> {
        let home = dirs::home_dir().expect("Could not find home directory");
        Ok(home.join(".cx").join(CACHE_FILE))
    }
}

pub fn resolve_alias(name: &str) -> Option<String> {
    Registry::get(name)
}

pub fn search(query: &str) -> Vec<(String, String)> {
    let registry = Registry::load().unwrap_or_else(|_| Registry::default());
    let query = query.to_lowercase();

    registry
        .0
        .iter()
        .filter(|(k, entry)| {
            k.to_lowercase().contains(&query)
                || entry
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query))
                    .unwrap_or(false)
        })
        .map(|(k, entry)| (k.clone(), entry.url.clone()))
        .collect()
}
