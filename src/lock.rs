use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LockFile {
    #[serde(rename = "package")]
    pub packages: BTreeMap<String, PackageLock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageLock {
    pub git: String,
    pub rev: String,
}

impl LockFile {
    pub fn load() -> Result<Self> {
        if Path::new("cx.lock").exists() {
            let content = fs::read_to_string("cx.lock")?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write("cx.lock", content)?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&PackageLock> {
        self.packages.get(name)
    }

    pub fn insert(&mut self, name: String, git: String, rev: String) {
        self.packages.insert(name, PackageLock { git, rev });
    }
}
