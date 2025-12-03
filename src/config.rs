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
            model: "qwen3:8b".to_string(),
            workspace_path: PathBuf::from("./workspace"),
            ollama_url: "http://localhost:11434/api/chat".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // 1. Determine config path: ~/.config/copilot_rust_llama/config.toml
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("copilot_rust_llama");

        let config_path = config_dir.join("config.toml");

        // 2. If file exists, load it
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config at {:?}", config_path))?;

            let config: Config =
                toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;

            return Ok(config);
        }

        // 3. Fallback: check legacy environment variable or return default
        if let Ok(env_path) = std::env::var("LLM_AGENT_WORKSPACE") {
            let mut config = Config::default();
            config.workspace_path = PathBuf::from(env_path);
            return Ok(config);
        }

        Ok(Config::default())
    }
}
