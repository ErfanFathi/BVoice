use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub input_device: Option<String>,
    #[serde(default = "default_beam_size")]
    pub beam_size: u32,
    #[serde(default = "default_use_vad")]
    pub use_vad: bool,
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
    #[serde(default)]
    pub overlay_position: Option<(i32, i32)>,
}

fn default_beam_size() -> u32 {
    2
}

fn default_use_vad() -> bool {
    false
}

fn default_vad_threshold() -> f32 {
    0.5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "base.en".to_string(),
            input_device: None,
            beam_size: default_beam_size(),
            use_vad: default_use_vad(),
            vad_threshold: default_vad_threshold(),
            overlay_position: None,
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
