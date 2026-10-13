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

    #[serde(default = "default_popup")]
    pub popup: bool,

    #[serde(default = "default_popup_width")]
    pub popup_width: u16,

    #[serde(default = "default_popup_height")]
    pub popup_height: u16,

    #[serde(default = "default_popup_border")]
    pub popup_border: String,

    #[serde(default = "default_border_color")]
    pub border_color: String,

    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,

    #[serde(default = "default_search_prompt")]
    pub search_prompt: String,

    #[serde(default = "default_search_title")]
    pub search_title: String,

    #[serde(default = "default_true")]
    pub show_badges: bool,

    #[serde(default = "default_true")]
    pub show_icons: bool,

    #[serde(default = "default_true")]
    pub show_path: bool,

    #[serde(default = "default_true")]
    pub show_footer: bool,

    #[serde(default)]
    pub theme: ThemeConfig,

    #[serde(default)]
    pub show_windows: bool,

    #[serde(default)]
    pub group_separator: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ThemeConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_focused: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_selected_fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_selected_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_unselected_fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_tmux: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_git: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_zoxide: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_custom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_windows: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_separator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_scroll_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_button_active: Option<String>,
}

impl<'de> Deserialize<'de> for ThemeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = ThemeConfigRaw::deserialize(deserializer)?;
        Ok(raw.into())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ThemeConfigRaw {
    Name(String),
    Detailed(Box<ThemeConfigDetailed>),
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ThemeConfigDetailed {
    #[serde(default = "default_preset_name", alias = "preset")]
    pub name: String,
    pub border: Option<String>,
    pub border_focused: Option<String>,
    pub title: Option<String>,
    pub search_prompt: Option<String>,
    pub search_text: Option<String>,
    pub item_selected_fg: Option<String>,
    pub item_selected_bg: Option<String>,
    pub item_unselected_fg: Option<String>,
    pub item_path: Option<String>,
    pub cursor: Option<String>,
    pub badge_tmux: Option<String>,
    pub badge_git: Option<String>,
    pub badge_alias: Option<String>,
    pub badge_config: Option<String>,
    pub badge_zoxide: Option<String>,
    pub badge_custom: Option<String>,
    pub inline_windows: Option<String>,
    pub group_separator: Option<String>,
    pub preview_border: Option<String>,
    pub preview_title: Option<String>,
    pub preview_scroll_indicator: Option<String>,
    pub footer_key: Option<String>,
    pub footer_desc: Option<String>,
    pub confirm_border: Option<String>,
    pub confirm_button_active: Option<String>,
}

fn default_preset_name() -> String {
    "catppuccin-mocha".to_owned()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_preset_name(),
            border: None,
            border_focused: None,
            title: None,
            search_prompt: None,
            search_text: None,
            item_selected_fg: None,
            item_selected_bg: None,
            item_unselected_fg: None,
            item_path: None,
            cursor: None,
            badge_tmux: None,
            badge_git: None,
            badge_alias: None,
            badge_config: None,
            badge_zoxide: None,
            badge_custom: None,
            inline_windows: None,
            group_separator: None,
            preview_border: None,
            preview_title: None,
            preview_scroll_indicator: None,
            footer_key: None,
            footer_desc: None,
            confirm_border: None,
            confirm_button_active: None,
        }
    }
}

impl From<ThemeConfigRaw> for ThemeConfig {
    fn from(raw: ThemeConfigRaw) -> Self {
        match raw {
            ThemeConfigRaw::Name(name) => Self {
                name,
                ..Default::default()
            },
            ThemeConfigRaw::Detailed(d) => Self {
                name: d.name,
                border: d.border,
                border_focused: d.border_focused,
                title: d.title,
                search_prompt: d.search_prompt,
                search_text: d.search_text,
                item_selected_fg: d.item_selected_fg,
                item_selected_bg: d.item_selected_bg,
                item_unselected_fg: d.item_unselected_fg,
                item_path: d.item_path,
                cursor: d.cursor,
                badge_tmux: d.badge_tmux,
                badge_git: d.badge_git,
                badge_alias: d.badge_alias,
                badge_config: d.badge_config,
                badge_zoxide: d.badge_zoxide,
                badge_custom: d.badge_custom,
                inline_windows: d.inline_windows,
                group_separator: d.group_separator,
                preview_border: d.preview_border,
                preview_title: d.preview_title,
                preview_scroll_indicator: d.preview_scroll_indicator,
                footer_key: d.footer_key,
                footer_desc: d.footer_desc,
                confirm_border: d.confirm_border,
                confirm_button_active: d.confirm_button_active,
            },
        }
    }
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

fn default_popup() -> bool {
    true
}

fn default_popup_width() -> u16 {
    85
}

fn default_popup_height() -> u16 {
    80
}

fn default_popup_border() -> String {
    "rounded".to_owned()
}

fn default_border_color() -> String {
    "cyan".to_owned()
}

fn default_highlight_color() -> String {
    "yellow".to_owned()
}

fn default_search_prompt() -> String {
    "> ".to_owned()
}

fn default_search_title() -> String {
    " Search / Switch Session (Prefixes: :t tmux, :g git, @ alias) ".to_owned()
}

fn default_true() -> bool {
    true
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            preview: default_preview(),
            preview_width: default_preview_width(),
            preview_border: default_preview_border(),
            popup: default_popup(),
            popup_width: default_popup_width(),
            popup_height: default_popup_height(),
            popup_border: default_popup_border(),
            border_color: default_border_color(),
            highlight_color: default_highlight_color(),
            search_prompt: default_search_prompt(),
            search_title: default_search_title(),
            show_badges: default_true(),
            show_icons: default_true(),
            show_path: default_true(),
            show_footer: default_true(),
            theme: ThemeConfig::default(),
            show_windows: false,
            group_separator: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SortTier {
    Single(String),
    Group(Vec<String>),
}

impl SortTier {
    pub fn matches(&self, source: &str, alias: Option<&str>, custom_source: Option<&str>) -> bool {
        match self {
            Self::Single(s) => {
                source == s || alias == Some(s.as_str()) || custom_source == Some(s.as_str())
            }
            Self::Group(group) => group.iter().any(|s| {
                source == s || alias == Some(s.as_str()) || custom_source == Some(s.as_str())
            }),
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
    pub sort_order: Vec<SortTier>,

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

    /// Loads configuration from standard paths (`config.json`).
    pub fn load() -> Self {
        if let Some(dir) = Self::config_dir() {
            let config_json = dir.join("config.json");
            if config_json.exists()
                && let Ok(cfg) = Self::load_from_path(&config_json)
            {
                return cfg;
            }
        }

        Self::default()
    }

    /// Loads configuration from an explicit JSON file path.
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
