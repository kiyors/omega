pub mod engine;

pub use engine::LuaEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitContext {
    pub repo: String,
    pub branch: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventPayload {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCandidate {
    pub source_name: String,
    pub name: String,
    pub path: PathBuf,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PaneLayoutConfig {
    pub command: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WindowLayoutConfig {
    pub name: String,
    pub command: Option<String>,
    pub layout: Option<String>,
    #[serde(default)]
    pub panes: Vec<PaneLayoutConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AliasConfig {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub windows: Vec<WindowLayoutConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WildcardRule {
    pub pattern: String,
    pub startup_command: Option<String>,
    pub icon: Option<String>,
    #[serde(default)]
    pub windows: Vec<WindowLayoutConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameSubstitution {
    pub find: String,
    pub replace: String,
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
    "rounded".to_owned()
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
    pub wildcards: Vec<WildcardRule>,

    #[serde(default)]
    pub name_substitutions: Vec<NameSubstitution>,

    #[serde(default)]
    pub icons: HashMap<String, String>,

    #[serde(default)]
    pub keymaps: HashMap<String, String>,

    #[serde(default)]
    pub sort_order: Vec<serde_json::Value>,

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
            wildcards: Vec::new(),
            name_substitutions: Vec::new(),
            icons: HashMap::new(),
            keymaps: HashMap::new(),
            sort_order: Vec::new(),
            tui: TuiConfig::default(),
        }
    }
}

impl Config {
    /// Discovers configuration directory (`$XDG_CONFIG_HOME/omega` or `~/.config/omega`).
    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(config_home).join("omega");
            if path.exists() {
                return Some(path);
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join(".config/omega");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Loads configuration alongside an optional active `LuaEngine` instance.
    pub fn load_with_engine() -> (Self, Option<LuaEngine>) {
        if let Some(dir) = Self::config_dir() {
            let has_lua = dir.join("init.lua").exists() || dir.join("plugins").is_dir();
            if has_lua && let Ok(engine) = LuaEngine::from_config_dir(&dir) {
                let cfg = engine.config();
                return (cfg, Some(engine));
            }

            let config_json = dir.join("config.json");
            if config_json.exists()
                && let Ok(cfg) = Self::load_from_path(&config_json)
            {
                return (cfg, None);
            }
        }

        (Self::default(), None)
    }

    /// Loads configuration from standard paths (`init.lua` takes precedence over `config.json`).
    pub fn load() -> Self {
        Self::load_with_engine().0
    }

    /// Loads configuration from an explicit file path (`.lua` or `.json`).
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        if path.extension().and_then(|s| s.to_str()) == Some("lua") {
            let dir = path.parent().unwrap_or(path);
            let engine = LuaEngine::from_config_dir(dir)
                .map_err(|e| std::io::Error::other(format!("{e}")))?;
            Ok(engine.config())
        } else {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }
}
