use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::PtlError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Font name or path (e.g. "sans-serif", "/usr/share/fonts/MyFont.ttf")
    pub font: String,
    /// Override font size in pixels (None = auto-size to tape)
    pub fontsize: Option<f32>,
    /// Preferred device as "vid:pid" hex string (e.g. "04f9:2062")
    pub device: Option<String>,
    /// Log level string
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: "sans-serif".to_string(),
            fontsize: None,
            device: None,
            log_level: "warn".to_string(),
        }
    }
}

impl Config {
    /// Load config from the given path, falling back to defaults if the file
    /// doesn't exist. Returns an error only for parse failures.
    pub fn load(path: &Path) -> Result<Self, PtlError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| PtlError::Config(e.to_string()))
    }

    /// Default config file path: ~/.config/ptl/config.toml
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ptl").join("config.toml"))
    }
}
