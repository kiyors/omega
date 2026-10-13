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
    Custom = 5,
}

impl SessionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tmux => "tmux",
            Self::Alias => "alias",
            Self::Config => "config",
            Self::Git => "git",
            Self::Zoxide => "zoxide",
            Self::Custom => "custom",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            Self::Tmux => 0,
            Self::Alias => 1,
            Self::Config => 2,
            Self::Git => 3,
            Self::Zoxide => 4,
            Self::Custom => 5,
        }
    }

    pub fn sigil_prefix(&self) -> &'static str {
        match self {
            Self::Tmux => ":t ",
            Self::Alias => "@",
            Self::Config => "",
            Self::Git => ":g ",
            Self::Zoxide => "",
            Self::Custom => "",
        }
    }

    /// Matches and strips a leading interactive sigil prefix:
    /// - `@<query>` for aliases
    /// - `:t` or `:t <query>` for tmux
    /// - `:g` or `:g <query>` for git
    pub fn from_sigil(query: &str) -> Option<(Self, &str)> {
        if query == ":t" {
            Some((Self::Tmux, ""))
        } else if let Some(rest) = query.strip_prefix(":t ") {
            Some((Self::Tmux, rest.trim_start()))
        } else if query == ":g" {
            Some((Self::Git, ""))
        } else if let Some(rest) = query.strip_prefix(":g ") {
            Some((Self::Git, rest.trim_start()))
        } else if let Some(rest) = query.strip_prefix('@') {
            Some((Self::Alias, rest.trim_start()))
        } else {
            None
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
    pub custom_source: Option<String>,
    pub icon: Option<String>,
    pub attached: bool,
    pub windows: Option<usize>,
    pub window_names: Vec<String>,
}

impl SessionCandidate {
    pub fn new<P: Into<PathBuf>>(name: impl Into<String>, path: P, source: SessionSource) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            source,
            score: None,
            alias: None,
            custom_source: None,
            icon: None,
            attached: false,
            windows: None,
            window_names: Vec::new(),
        }
    }

    pub fn from_custom(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        source_name: impl Into<String>,
        icon: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            source: SessionSource::Custom,
            score: None,
            alias: None,
            custom_source: Some(source_name.into()),
            icon,
            attached: false,
            windows: None,
            window_names: Vec::new(),
        }
    }

    /// Inherits missing metadata (alias, custom_source, icon, score, window_names) from another candidate.
    pub fn inherit_metadata(&mut self, other: &Self) {
        if self.alias.is_none() {
            self.alias = other.alias.clone();
        }
        if self.custom_source.is_none() {
            self.custom_source = other.custom_source.clone();
        }
        if self.icon.is_none() {
            self.icon = other.icon.clone();
        }
        if self.score.is_none() {
            self.score = other.score;
        }
        if self.window_names.is_empty() {
            self.window_names = other.window_names.clone();
        }
    }
}
