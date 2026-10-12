use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    Tmux = 0,
    Alias = 1,
    Config = 2,
    Git = 3,
    Zoxide = 4,
}

impl SessionSource {
    pub fn priority(&self) -> u8 {
        match self {
            Self::Tmux => 0,
            Self::Alias => 1,
            Self::Config => 2,
            Self::Git => 3,
            Self::Zoxide => 4,
        }
    }

    pub fn sigil_prefix(&self) -> &'static str {
        match self {
            Self::Tmux => ":t ",
            Self::Alias => "@",
            Self::Config => "",
            Self::Git => ":g ",
            Self::Zoxide => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCandidate {
    pub name: String,
    pub path: PathBuf,
    pub source: SessionSource,
    pub score: Option<f64>,
    pub alias: Option<String>,
    pub icon: Option<String>,
    pub attached: bool,
    pub windows: Option<usize>,
}

impl SessionCandidate {
    pub fn new<P: Into<PathBuf>>(name: impl Into<String>, path: P, source: SessionSource) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            source,
            score: None,
            alias: None,
            icon: None,
            attached: false,
            windows: None,
        }
    }
}
