use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct CxConfig {
    pub package: PackageConfig,
    pub dependencies: Option<HashMap<String, Dependency>>,
    pub build: Option<BuildConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Complex {
        git: String,
        branch: Option<String>,
        build: Option<String>,
        output: Option<String>,
    },
}

impl Dependency {
    pub fn get_url(&self) -> String {
        match self {
            Dependency::Simple(url) => url.clone(),
            Dependency::Complex { git, .. } => git.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct PackageConfig {
    pub name: String,
    #[allow(dead_code)]
    pub version: String,
    #[serde(default = "default_edition")]
    pub edition: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct BuildConfig {
    pub cflags: Option<Vec<String>>,
    pub libs: Option<Vec<String>>,
}

fn default_edition() -> String {
    "c++20".to_string()
}
