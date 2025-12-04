use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub workspace_path: PathBuf,
    pub ollama_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "qwen2.5-coder:latest".to_string(),
            workspace_path: PathBuf::from("./workspace"),
            ollama_url: "http://localhost:11434/api/chat".to_string(),
        }
    }
}

impl Config {
    // Helper to get the consistent config path
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("agerus");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        // 1. If file exists, load it
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config at {:?}", config_path))?;

            let config: Config =
                toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;

            return Ok(config);
        }

        // 2. Legacy Fallback (optional, keep if you want backward compat)
        if let Ok(env_path) = std::env::var("LLM_AGENT_WORKSPACE") {
            let mut config = Config::default();
            config.workspace_path = PathBuf::from(env_path);
            return Ok(config);
        }

        Ok(Config::default())
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }
}
