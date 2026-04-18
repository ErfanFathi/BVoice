use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub arm_threshold_ms: u64,
    pub input_device: Option<String>,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_beam_size")]
    pub beam_size: u32,
}

fn default_hotkey() -> String {
    "AltGr".to_string()
}

fn default_beam_size() -> u32 {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "base.en".to_string(),
            arm_threshold_ms: 1000,
            input_device: None,
            hotkey: default_hotkey(),
            beam_size: default_beam_size(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("bvoice");
    p.push("config.toml");
    p
}

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(c: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let s = toml::to_string_pretty(c)?;
    std::fs::write(&path, s)?;
    Ok(())
}
