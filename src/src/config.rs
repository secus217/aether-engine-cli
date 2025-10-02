use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub api_endpoint: String,
    pub auth_token: Option<String>,
    pub default_runtime: String,
    pub build_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_endpoint: "https://aetherngine.com".to_string(),
            auth_token: None,
            default_runtime: "node:20".to_string(),
            build_timeout: 300, // 5 minutes
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Create default config
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| crate::AetherError::config("HOME environment variable not set"))?;
        Ok(PathBuf::from(home).join(".aether").join("config.json"))
    }

    pub fn set_auth_token(&mut self, token: String) -> Result<()> {
        self.auth_token = Some(token);
        self.save()
    }

    pub fn clear_auth_token(&mut self) -> Result<()> {
        self.auth_token = None;
        self.save()
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }
}
