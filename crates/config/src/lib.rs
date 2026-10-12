use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasConfig {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
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
        }
    }
}
