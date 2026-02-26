use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::themes::DEFAULT_THEME;

pub const DEFAULT_MODEL: &str = "MiniMax-M2.5";

pub const VALID_MODELS: &[&str] = &["MiniMax-M2.5", "MiniMax-M2.5-highspeed"];

pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("MiniMax-M2.5", "Latest, ~60 tps"),
    ("MiniMax-M2.5-highspeed", "Latest fast, ~100 tps"),
    ("MiniMax-M2.1", "Previous gen, ~60 tps"),
    ("MiniMax-M2.1-highspeed", "Previous gen fast, ~100 tps"),
];

pub const MODEL_IDS: &[&str] = &[
    "MiniMax-M2.5",
    "MiniMax-M2.5-highspeed",
    "MiniMax-M2.1",
    "MiniMax-M2.1-highspeed",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_theme() -> String {
    DEFAULT_THEME.to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: DEFAULT_MODEL.to_string(),
            theme: DEFAULT_THEME.to_string(),
            mcp_servers: HashMap::new(),
        }
    }
}

/// Returns the path to ~/.minmax-code/
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minmax-code")
}

/// Returns the path to ~/.minmax-code/config.json
pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    let dir = config_dir();
    let file = config_file();

    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }

    if !file.exists() {
        let config = AppConfig::default();
        let _ = save_config(&config);
        return config;
    }

    match fs::read_to_string(&file) {
        Ok(raw) => match serde_json::from_str::<AppConfig>(&raw) {
            Ok(mut config) => {
                // Migrate: if saved model is not valid, reset to default
                if !VALID_MODELS.contains(&config.model.as_str()) {
                    config.model = DEFAULT_MODEL.to_string();
                    let _ = save_config(&config);
                }
                config
            }
            Err(_) => AppConfig::default(),
        },
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_file(), json)?;
    Ok(())
}

pub fn update_config(partial: serde_json::Value) -> Result<AppConfig> {
    let mut config = load_config();

    if let Some(key) = partial.get("apiKey").and_then(|v| v.as_str()) {
        config.api_key = key.to_string();
    }
    if let Some(model) = partial.get("model").and_then(|v| v.as_str()) {
        config.model = model.to_string();
    }
    if let Some(theme) = partial.get("theme").and_then(|v| v.as_str()) {
        config.theme = theme.to_string();
    }

    save_config(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_config<F: FnOnce(PathBuf)>(f: F) {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        f(config_path);
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = AppConfig::default();
        assert_eq!(config.api_key, "");
        assert_eq!(config.model, "MiniMax-M2.5");
        assert_eq!(config.theme, "tokyo-night");
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn config_serialization_round_trip() {
        let config = AppConfig {
            api_key: "test-key-123".to_string(),
            model: "MiniMax-M2.5-highspeed".to_string(),
            theme: "gruvbox".to_string(),
            mcp_servers: HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.api_key, "test-key-123");
        assert_eq!(deserialized.model, "MiniMax-M2.5-highspeed");
        assert_eq!(deserialized.theme, "gruvbox");
    }

    #[test]
    fn config_deserializes_with_missing_fields() {
        let json = r#"{"apiKey": "abc"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, "abc");
        assert_eq!(config.model, "MiniMax-M2.5");
        assert_eq!(config.theme, "tokyo-night");
    }

    #[test]
    fn save_and_load_config_file() {
        with_temp_config(|path| {
            let config = AppConfig {
                api_key: "my-key".to_string(),
                ..AppConfig::default()
            };
            let json = serde_json::to_string_pretty(&config).unwrap();
            fs::write(&path, &json).unwrap();

            let raw = fs::read_to_string(&path).unwrap();
            let loaded: AppConfig = serde_json::from_str(&raw).unwrap();
            assert_eq!(loaded.api_key, "my-key");
        });
    }
}
