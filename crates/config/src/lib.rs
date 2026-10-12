use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasConfig {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuiConfig {
    #[serde(default = "default_preview")]
    pub preview: bool,

    #[serde(default = "default_preview_width")]
    pub preview_width: u16,

    #[serde(default = "default_preview_border")]
    pub preview_border: String,
}

fn default_preview() -> bool {
    true
}

fn default_preview_width() -> u16 {
    55
}

fn default_preview_border() -> String {
    "rounded".to_string()
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            preview: default_preview(),
            preview_width: default_preview_width(),
            preview_border: default_preview_border(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_hide_attached")]
    pub hide_attached: bool,

    #[serde(default = "default_dir_length")]
    pub dir_length: usize,

    #[serde(default)]
    pub roots: Vec<PathBuf>,

    #[serde(default)]
    pub aliases: Vec<AliasConfig>,

    #[serde(default)]
    pub tui: TuiConfig,
}

fn default_hide_attached() -> bool {
    true
}

fn default_dir_length() -> usize {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hide_attached: default_hide_attached(),
            dir_length: default_dir_length(),
            roots: Vec::new(),
            aliases: Vec::new(),
            tui: TuiConfig::default(),
        }
    }
}

impl Config {
    /// Loads configuration from standard paths (`$XDG_CONFIG_HOME/omega/config.json`,
    /// `~/.config/omega/config.json`) or falls back to default.
    pub fn load() -> Self {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(config_home).join("omega/config.json");
            if path.exists()
                && let Ok(cfg) = Self::load_from_path(&path)
            {
                return cfg;
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join(".config/omega/config.json");
            if path.exists()
                && let Ok(cfg) = Self::load_from_path(&path)
            {
                return cfg;
            }
        }

        Self::default()
    }

    /// Loads configuration from an explicit file path.
    pub fn load_from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
