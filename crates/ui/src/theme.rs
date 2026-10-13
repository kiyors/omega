use omega_config::ThemeConfig;
use ratatui::style::Color;

#[must_use]
pub fn parse_color(input: &str) -> Color {
    let s = input.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("reset") {
        return Color::Reset;
    }

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            )
        {
            return Color::Rgb(r, g, b);
        } else if hex.len() == 3
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..1], 16),
                u8::from_str_radix(&hex[1..2], 16),
                u8::from_str_radix(&hex[2..3], 16),
            )
        {
            return Color::Rgb(r * 17, g * 17, b * 17);
        }
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3
            && let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
            )
        {
            return Color::Rgb(r, g, b);
        }
    }

    if let Some(num_str) = s
        .strip_prefix("color256:")
        .or_else(|| s.strip_prefix("indexed:"))
        && let Ok(idx) = num_str.trim().parse::<u8>()
    {
        return Color::Indexed(idx);
    }

    if s.chars().all(|c| c.is_ascii_digit())
        && let Ok(idx) = s.parse::<u8>()
    {
        return Color::Indexed(idx);
    }

    match s.to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" | "darkgray" | "dark_gray" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Reset,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTheme {
    pub border: Color,
    pub border_focused: Color,
    pub title: Color,
    pub search_prompt: Color,
    pub search_text: Color,
    pub item_selected_fg: Color,
    pub item_selected_bg: Option<Color>,
    pub item_unselected_fg: Color,
    pub item_path: Color,
    pub cursor: Color,
    pub badge_tmux: Color,
    pub badge_git: Color,
    pub badge_alias: Color,
    pub badge_config: Color,
    pub badge_zoxide: Color,
    pub badge_custom: Color,
    pub inline_windows: Color,
    pub group_separator: Color,
    pub preview_border: Color,
    pub preview_title: Color,
    pub preview_scroll_indicator: Color,
    pub footer_key: Color,
    pub footer_desc: Color,
    pub confirm_border: Color,
    pub confirm_button_active_fg: Color,
    pub confirm_button_active_bg: Color,
}

impl ResolvedTheme {
    #[must_use]
    pub fn preset_catppuccin_mocha() -> Self {
        Self {
            border: Color::Rgb(0x89, 0xb4, 0xfa),                   // Blue
            border_focused: Color::Rgb(0xcb, 0xa6, 0xf7),           // Mauve
            title: Color::Rgb(0xcd, 0xd6, 0xf4),                    // Text
            search_prompt: Color::Rgb(0xf3, 0x8b, 0xa8),            // Red
            search_text: Color::Rgb(0xcd, 0xd6, 0xf4),              // Text
            item_selected_fg: Color::Rgb(0xcb, 0xa6, 0xf7),         // Mauve
            item_selected_bg: Some(Color::Rgb(0x31, 0x32, 0x44)),   // Surface0
            item_unselected_fg: Color::Rgb(0xcd, 0xd6, 0xf4),       // Text
            item_path: Color::Rgb(0x6c, 0x70, 0x86),                // Overlay0
            cursor: Color::Rgb(0xa6, 0xe3, 0xa1),                   // Green
            badge_tmux: Color::Rgb(0xa6, 0xe3, 0xa1),               // Green
            badge_git: Color::Rgb(0x89, 0xb4, 0xfa),                // Blue
            badge_alias: Color::Rgb(0xf5, 0xc2, 0xe7),              // Pink
            badge_config: Color::Rgb(0x94, 0xe2, 0xd5),             // Teal
            badge_zoxide: Color::Rgb(0xf9, 0xe2, 0xaf),             // Yellow
            badge_custom: Color::Rgb(0x74, 0xc7, 0xec),             // Sapphire
            inline_windows: Color::Rgb(0x93, 0x99, 0xb2),           // Overlay2
            group_separator: Color::Rgb(0x45, 0x47, 0x5a),          // Surface1
            preview_border: Color::Rgb(0x58, 0x5b, 0x70),           // Surface2
            preview_title: Color::Rgb(0x89, 0xb4, 0xfa),            // Blue
            preview_scroll_indicator: Color::Rgb(0xfa, 0xb3, 0x87), // Peach
            footer_key: Color::Rgb(0xfa, 0xb3, 0x87),               // Peach
            footer_desc: Color::Rgb(0xa6, 0xad, 0xc8),              // Subtext0
            confirm_border: Color::Rgb(0xf3, 0x8b, 0xa8),           // Red
            confirm_button_active_fg: Color::Rgb(0x11, 0x11, 0x1b), // Crust
            confirm_button_active_bg: Color::Rgb(0xa6, 0xe3, 0xa1), // Green
        }
    }

    #[must_use]
    pub fn preset_catppuccin_macchiato() -> Self {
        Self {
            border: Color::Rgb(0x8a, 0xad, 0xf4),
            border_focused: Color::Rgb(0xc6, 0xa0, 0xf6),
            title: Color::Rgb(0xca, 0xd3, 0xf5),
            search_prompt: Color::Rgb(0xed, 0x87, 0x96),
            search_text: Color::Rgb(0xca, 0xd3, 0xf5),
            item_selected_fg: Color::Rgb(0xc6, 0xa0, 0xf6),
            item_selected_bg: Some(Color::Rgb(0x36, 0x3a, 0x4f)),
            item_unselected_fg: Color::Rgb(0xca, 0xd3, 0xf5),
            item_path: Color::Rgb(0x6e, 0x73, 0x8d),
            cursor: Color::Rgb(0xa6, 0xda, 0x95),
            badge_tmux: Color::Rgb(0xa6, 0xda, 0x95),
            badge_git: Color::Rgb(0x8a, 0xad, 0xf4),
            badge_alias: Color::Rgb(0xf5, 0xbd, 0xe6),
            badge_config: Color::Rgb(0x8b, 0xd5, 0xca),
            badge_zoxide: Color::Rgb(0xee, 0xd4, 0x9f),
            badge_custom: Color::Rgb(0x7d, 0xc4, 0xe4),
            inline_windows: Color::Rgb(0x93, 0x9a, 0xb7),
            group_separator: Color::Rgb(0x49, 0x4d, 0x64),
            preview_border: Color::Rgb(0x5b, 0x60, 0x78),
            preview_title: Color::Rgb(0x8a, 0xad, 0xf4),
            preview_scroll_indicator: Color::Rgb(0xf5, 0xa9, 0x7f),
            footer_key: Color::Rgb(0xf5, 0xa9, 0x7f),
            footer_desc: Color::Rgb(0xa5, 0xad, 0xcb),
            confirm_border: Color::Rgb(0xed, 0x87, 0x96),
            confirm_button_active_fg: Color::Rgb(0x18, 0x19, 0x26),
            confirm_button_active_bg: Color::Rgb(0xa6, 0xda, 0x95),
        }
    }

    #[must_use]
    pub fn preset_catppuccin_frappe() -> Self {
        Self {
            border: Color::Rgb(0x8c, 0xaa, 0xee),
            border_focused: Color::Rgb(0xca, 0x9e, 0xe6),
            title: Color::Rgb(0xc6, 0xd0, 0xf5),
            search_prompt: Color::Rgb(0xe7, 0x82, 0x84),
            search_text: Color::Rgb(0xc6, 0xd0, 0xf5),
            item_selected_fg: Color::Rgb(0xca, 0x9e, 0xe6),
            item_selected_bg: Some(Color::Rgb(0x41, 0x45, 0x59)),
            item_unselected_fg: Color::Rgb(0xc6, 0xd0, 0xf5),
            item_path: Color::Rgb(0x73, 0x79, 0x94),
            cursor: Color::Rgb(0xa6, 0xd1, 0x89),
            badge_tmux: Color::Rgb(0xa6, 0xd1, 0x89),
            badge_git: Color::Rgb(0x8c, 0xaa, 0xee),
            badge_alias: Color::Rgb(0xf4, 0xb8, 0xe4),
            badge_config: Color::Rgb(0x81, 0xc8, 0xbe),
            badge_zoxide: Color::Rgb(0xe5, 0xc8, 0x90),
            badge_custom: Color::Rgb(0x85, 0xc1, 0xdc),
            inline_windows: Color::Rgb(0x94, 0x9c, 0xbb),
            group_separator: Color::Rgb(0x51, 0x57, 0x6d),
            preview_border: Color::Rgb(0x62, 0x68, 0x80),
            preview_title: Color::Rgb(0x8c, 0xaa, 0xee),
            preview_scroll_indicator: Color::Rgb(0xef, 0x9f, 0x76),
            footer_key: Color::Rgb(0xef, 0x9f, 0x76),
            footer_desc: Color::Rgb(0xa5, 0xad, 0xce),
            confirm_border: Color::Rgb(0xe7, 0x82, 0x84),
            confirm_button_active_fg: Color::Rgb(0x23, 0x26, 0x34),
            confirm_button_active_bg: Color::Rgb(0xa6, 0xd1, 0x89),
        }
    }

    #[must_use]
    pub fn preset_catppuccin_latte() -> Self {
        Self {
            border: Color::Rgb(0x1e, 0x66, 0xf5),
            border_focused: Color::Rgb(0x88, 0x39, 0xef),
            title: Color::Rgb(0x4c, 0x4f, 0x69),
            search_prompt: Color::Rgb(0xd2, 0x0f, 0x39),
            search_text: Color::Rgb(0x4c, 0x4f, 0x69),
            item_selected_fg: Color::Rgb(0x88, 0x39, 0xef),
            item_selected_bg: Some(Color::Rgb(0xcc, 0xd0, 0xda)),
            item_unselected_fg: Color::Rgb(0x4c, 0x4f, 0x69),
            item_path: Color::Rgb(0x8c, 0x8f, 0xa1),
            cursor: Color::Rgb(0x40, 0xa0, 0x2b),
            badge_tmux: Color::Rgb(0x40, 0xa0, 0x2b),
            badge_git: Color::Rgb(0x1e, 0x66, 0xf5),
            badge_alias: Color::Rgb(0xea, 0x76, 0xcb),
            badge_config: Color::Rgb(0x17, 0x92, 0x99),
            badge_zoxide: Color::Rgb(0xdf, 0x8e, 0x1d),
            badge_custom: Color::Rgb(0x20, 0x9f, 0xb5),
            inline_windows: Color::Rgb(0x7c, 0x7f, 0x93),
            group_separator: Color::Rgb(0xbc, 0xc0, 0xcc),
            preview_border: Color::Rgb(0xac, 0xb0, 0xbe),
            preview_title: Color::Rgb(0x1e, 0x66, 0xf5),
            preview_scroll_indicator: Color::Rgb(0xfe, 0x64, 0x0b),
            footer_key: Color::Rgb(0xfe, 0x64, 0x0b),
            footer_desc: Color::Rgb(0x6c, 0x6f, 0x85),
            confirm_border: Color::Rgb(0xd2, 0x0f, 0x39),
            confirm_button_active_fg: Color::Rgb(0xdc, 0xe0, 0xe8),
            confirm_button_active_bg: Color::Rgb(0x40, 0xa0, 0x2b),
        }
    }

    #[must_use]
    pub fn preset_tokyonight() -> Self {
        Self {
            border: Color::Rgb(0x7a, 0xa2, 0xf7),
            border_focused: Color::Rgb(0xbb, 0x9a, 0xf7),
            title: Color::Rgb(0xc0, 0xca, 0xf5),
            search_prompt: Color::Rgb(0xf7, 0x76, 0x8e),
            search_text: Color::Rgb(0xc0, 0xca, 0xf5),
            item_selected_fg: Color::Rgb(0xbb, 0x9a, 0xf7),
            item_selected_bg: Some(Color::Rgb(0x24, 0x28, 0x3b)),
            item_unselected_fg: Color::Rgb(0xc0, 0xca, 0xf5),
            item_path: Color::Rgb(0x56, 0x5f, 0x89),
            cursor: Color::Rgb(0x9e, 0xce, 0x6a),
            badge_tmux: Color::Rgb(0x9e, 0xce, 0x6a),
            badge_git: Color::Rgb(0x7a, 0xa2, 0xf7),
            badge_alias: Color::Rgb(0xbb, 0x9a, 0xf7),
            badge_config: Color::Rgb(0x7d, 0xcf, 0xff),
            badge_zoxide: Color::Rgb(0xe0, 0xaf, 0x68),
            badge_custom: Color::Rgb(0x2a, 0xc3, 0xde),
            inline_windows: Color::Rgb(0x56, 0x5f, 0x89),
            group_separator: Color::Rgb(0x3b, 0x42, 0x61),
            preview_border: Color::Rgb(0x3b, 0x42, 0x61),
            preview_title: Color::Rgb(0x7a, 0xa2, 0xf7),
            preview_scroll_indicator: Color::Rgb(0xff, 0x9e, 0x64),
            footer_key: Color::Rgb(0xff, 0x9e, 0x64),
            footer_desc: Color::Rgb(0x9a, 0xa5, 0xce),
            confirm_border: Color::Rgb(0xf7, 0x76, 0x8e),
            confirm_button_active_fg: Color::Rgb(0x1a, 0x1b, 0x26),
            confirm_button_active_bg: Color::Rgb(0x9e, 0xce, 0x6a),
        }
    }

    #[must_use]
    pub fn preset_nord() -> Self {
        Self {
            border: Color::Rgb(0x88, 0xc0, 0xd0),
            border_focused: Color::Rgb(0x81, 0xa1, 0xc1),
            title: Color::Rgb(0xec, 0xef, 0xf4),
            search_prompt: Color::Rgb(0xbf, 0x61, 0x6a),
            search_text: Color::Rgb(0xec, 0xef, 0xf4),
            item_selected_fg: Color::Rgb(0x88, 0xc0, 0xd0),
            item_selected_bg: Some(Color::Rgb(0x3b, 0x42, 0x52)),
            item_unselected_fg: Color::Rgb(0xd8, 0xde, 0xe9),
            item_path: Color::Rgb(0x4c, 0x56, 0x6a),
            cursor: Color::Rgb(0xa3, 0xbe, 0x8c),
            badge_tmux: Color::Rgb(0xa3, 0xbe, 0x8c),
            badge_git: Color::Rgb(0x81, 0xa1, 0xc1),
            badge_alias: Color::Rgb(0xb4, 0x8e, 0xad),
            badge_config: Color::Rgb(0x8f, 0xbc, 0xbb),
            badge_zoxide: Color::Rgb(0xeb, 0xcb, 0x8b),
            badge_custom: Color::Rgb(0x5e, 0x81, 0xac),
            inline_windows: Color::Rgb(0x61, 0x6e, 0x88),
            group_separator: Color::Rgb(0x43, 0x4c, 0x5e),
            preview_border: Color::Rgb(0x43, 0x4c, 0x5e),
            preview_title: Color::Rgb(0x88, 0xc0, 0xd0),
            preview_scroll_indicator: Color::Rgb(0xd0, 0x87, 0x70),
            footer_key: Color::Rgb(0xd0, 0x87, 0x70),
            footer_desc: Color::Rgb(0xd8, 0xde, 0xe9),
            confirm_border: Color::Rgb(0xbf, 0x61, 0x6a),
            confirm_button_active_fg: Color::Rgb(0x2e, 0x34, 0x40),
            confirm_button_active_bg: Color::Rgb(0xa3, 0xbe, 0x8c),
        }
    }

    #[must_use]
    pub fn preset_gruvbox() -> Self {
        Self {
            border: Color::Rgb(0x83, 0xa5, 0x98),
            border_focused: Color::Rgb(0xd3, 0x86, 0x9b),
            title: Color::Rgb(0xfb, 0xf1, 0xc7),
            search_prompt: Color::Rgb(0xfb, 0x49, 0x34),
            search_text: Color::Rgb(0xfb, 0xf1, 0xc7),
            item_selected_fg: Color::Rgb(0xfa, 0xbd, 0x2f),
            item_selected_bg: Some(Color::Rgb(0x3c, 0x38, 0x36)),
            item_unselected_fg: Color::Rgb(0xeb, 0xdb, 0xb2),
            item_path: Color::Rgb(0x92, 0x83, 0x74),
            cursor: Color::Rgb(0xb8, 0xbb, 0x26),
            badge_tmux: Color::Rgb(0xb8, 0xbb, 0x26),
            badge_git: Color::Rgb(0x83, 0xa5, 0x98),
            badge_alias: Color::Rgb(0xd3, 0x86, 0x9b),
            badge_config: Color::Rgb(0x8e, 0xc0, 0x7c),
            badge_zoxide: Color::Rgb(0xfa, 0xbd, 0x2f),
            badge_custom: Color::Rgb(0x7c, 0xa6, 0x62),
            inline_windows: Color::Rgb(0xa8, 0x99, 0x84),
            group_separator: Color::Rgb(0x50, 0x49, 0x45),
            preview_border: Color::Rgb(0x50, 0x49, 0x45),
            preview_title: Color::Rgb(0x83, 0xa5, 0x98),
            preview_scroll_indicator: Color::Rgb(0xfe, 0x80, 0x19),
            footer_key: Color::Rgb(0xfe, 0x80, 0x19),
            footer_desc: Color::Rgb(0xeb, 0xdb, 0xb2),
            confirm_border: Color::Rgb(0xfb, 0x49, 0x34),
            confirm_button_active_fg: Color::Rgb(0x1d, 0x20, 0x21),
            confirm_button_active_bg: Color::Rgb(0xb8, 0xbb, 0x26),
        }
    }

    #[must_use]
    pub fn preset_default() -> Self {
        Self {
            border: Color::Cyan,
            border_focused: Color::Magenta,
            title: Color::White,
            search_prompt: Color::Cyan,
            search_text: Color::White,
            item_selected_fg: Color::Yellow,
            item_selected_bg: None,
            item_unselected_fg: Color::White,
            item_path: Color::DarkGray,
            cursor: Color::Green,
            badge_tmux: Color::Green,
            badge_git: Color::Blue,
            badge_alias: Color::Magenta,
            badge_config: Color::Cyan,
            badge_zoxide: Color::Yellow,
            badge_custom: Color::Cyan,
            inline_windows: Color::DarkGray,
            group_separator: Color::DarkGray,
            preview_border: Color::Cyan,
            preview_title: Color::Cyan,
            preview_scroll_indicator: Color::Yellow,
            footer_key: Color::Yellow,
            footer_desc: Color::DarkGray,
            confirm_border: Color::Red,
            confirm_button_active_fg: Color::Black,
            confirm_button_active_bg: Color::Green,
        }
    }

    #[must_use]
    pub fn from_config(config: &ThemeConfig) -> Self {
        let mut theme = match config.name.trim().to_ascii_lowercase().as_str() {
            "catppuccin-mocha" | "mocha" => Self::preset_catppuccin_mocha(),
            "catppuccin-macchiato" | "macchiato" => Self::preset_catppuccin_macchiato(),
            "catppuccin-frappe" | "frappe" => Self::preset_catppuccin_frappe(),
            "catppuccin-latte" | "latte" => Self::preset_catppuccin_latte(),
            "tokyonight" | "tokyo-night" => Self::preset_tokyonight(),
            "nord" => Self::preset_nord(),
            "gruvbox" => Self::preset_gruvbox(),
            "default" => Self::preset_default(),
            _ => Self::preset_catppuccin_mocha(),
        };

        if let Some(c) = &config.border {
            theme.border = parse_color(c);
        }
        if let Some(c) = &config.border_focused {
            theme.border_focused = parse_color(c);
        }
        if let Some(c) = &config.title {
            theme.title = parse_color(c);
        }
        if let Some(c) = &config.search_prompt {
            theme.search_prompt = parse_color(c);
        }
        if let Some(c) = &config.search_text {
            theme.search_text = parse_color(c);
        }
        if let Some(c) = &config.item_selected_fg {
            theme.item_selected_fg = parse_color(c);
        }
        if let Some(c) = &config.item_selected_bg {
            let parsed = parse_color(c);
            theme.item_selected_bg = if parsed == Color::Reset {
                None
            } else {
                Some(parsed)
            };
        }
        if let Some(c) = &config.item_unselected_fg {
            theme.item_unselected_fg = parse_color(c);
        }
        if let Some(c) = &config.item_path {
            theme.item_path = parse_color(c);
        }
        if let Some(c) = &config.cursor {
            theme.cursor = parse_color(c);
        }
        if let Some(c) = &config.badge_tmux {
            theme.badge_tmux = parse_color(c);
        }
        if let Some(c) = &config.badge_git {
            theme.badge_git = parse_color(c);
        }
        if let Some(c) = &config.badge_alias {
            theme.badge_alias = parse_color(c);
        }
        if let Some(c) = &config.badge_config {
            theme.badge_config = parse_color(c);
        }
        if let Some(c) = &config.badge_zoxide {
            theme.badge_zoxide = parse_color(c);
        }
        if let Some(c) = &config.badge_custom {
            theme.badge_custom = parse_color(c);
        }
        if let Some(c) = &config.inline_windows {
            theme.inline_windows = parse_color(c);
        }
        if let Some(c) = &config.group_separator {
            theme.group_separator = parse_color(c);
        }
        if let Some(c) = &config.preview_border {
            theme.preview_border = parse_color(c);
        }
        if let Some(c) = &config.preview_title {
            theme.preview_title = parse_color(c);
        }
        if let Some(c) = &config.preview_scroll_indicator {
            theme.preview_scroll_indicator = parse_color(c);
        }
        if let Some(c) = &config.footer_key {
            theme.footer_key = parse_color(c);
        }
        if let Some(c) = &config.footer_desc {
            theme.footer_desc = parse_color(c);
        }
        if let Some(c) = &config.confirm_border {
            theme.confirm_border = parse_color(c);
        }
        if let Some(c) = &config.confirm_button_active {
            theme.confirm_button_active_bg = parse_color(c);
        }

        theme
    }
}
