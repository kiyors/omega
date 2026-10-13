use omega_config::ThemeConfig;
use omega_ui::theme::{ResolvedTheme, parse_color};
use ratatui::style::Color;

#[test]
fn test_parse_color_hex() {
    assert_eq!(parse_color("#cba6f7"), Color::Rgb(0xcb, 0xa6, 0xf7));
    assert_eq!(parse_color("#89b4fa"), Color::Rgb(0x89, 0xb4, 0xfa));
    assert_eq!(parse_color("#fff"), Color::Rgb(255, 255, 255));
    assert_eq!(parse_color("#000"), Color::Rgb(0, 0, 0));
    assert_eq!(parse_color("#123"), Color::Rgb(0x11, 0x22, 0x33));
}

#[test]
fn test_parse_color_rgb_function() {
    assert_eq!(parse_color("rgb(203, 166, 247)"), Color::Rgb(203, 166, 247));
    assert_eq!(parse_color("rgb(10, 20, 30)"), Color::Rgb(10, 20, 30));
}

#[test]
fn test_parse_color_named_ansi() {
    assert_eq!(parse_color("red"), Color::Red);
    assert_eq!(parse_color("green"), Color::Green);
    assert_eq!(parse_color("blue"), Color::Blue);
    assert_eq!(parse_color("yellow"), Color::Yellow);
    assert_eq!(parse_color("magenta"), Color::Magenta);
    assert_eq!(parse_color("cyan"), Color::Cyan);
    assert_eq!(parse_color("white"), Color::White);
    assert_eq!(parse_color("black"), Color::Black);
    assert_eq!(parse_color("dark_gray"), Color::DarkGray);
    assert_eq!(parse_color("gray"), Color::DarkGray);
    assert_eq!(parse_color("light_blue"), Color::LightBlue);
    assert_eq!(parse_color("none"), Color::Reset);
    assert_eq!(parse_color("reset"), Color::Reset);
    assert_eq!(parse_color(""), Color::Reset);
}

#[test]
fn test_parse_color_indexed() {
    assert_eq!(parse_color("123"), Color::Indexed(123));
    assert_eq!(parse_color("color256:214"), Color::Indexed(214));
}

#[test]
fn test_resolved_theme_catppuccin_mocha_default() {
    let cfg = ThemeConfig::default();
    let theme = ResolvedTheme::from_config(&cfg);
    assert_eq!(theme.border, Color::Rgb(0x89, 0xb4, 0xfa)); // Blue
    assert_eq!(theme.border_focused, Color::Rgb(0xcb, 0xa6, 0xf7)); // Mauve
    assert_eq!(theme.search_prompt, Color::Rgb(0xf3, 0x8b, 0xa8)); // Red
    assert_eq!(theme.badge_tmux, Color::Rgb(0xa6, 0xe3, 0xa1)); // Green
    assert_eq!(theme.badge_git, Color::Rgb(0x89, 0xb4, 0xfa)); // Blue
    assert_eq!(theme.badge_alias, Color::Rgb(0xf5, 0xc2, 0xe7)); // Pink
}

#[test]
fn test_resolved_theme_tokyonight() {
    let mut cfg = ThemeConfig::default();
    cfg.name = "tokyonight".to_owned();
    let theme = ResolvedTheme::from_config(&cfg);
    assert_eq!(theme.border, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.badge_tmux, Color::Rgb(0x9e, 0xce, 0x6a));
}

#[test]
fn test_resolved_theme_with_element_overrides() {
    let mut cfg = ThemeConfig::default();
    cfg.name = "catppuccin-mocha".to_owned();
    cfg.border = Some("#ff0000".to_owned());
    cfg.search_prompt = Some("yellow".to_owned());
    cfg.inline_windows = Some("#123456".to_owned());

    let theme = ResolvedTheme::from_config(&cfg);
    assert_eq!(theme.border, Color::Rgb(0xff, 0x00, 0x00));
    assert_eq!(theme.search_prompt, Color::Yellow);
    assert_eq!(theme.inline_windows, Color::Rgb(0x12, 0x34, 0x56));
    // Un-overridden fields should retain Mocha defaults
    assert_eq!(theme.badge_tmux, Color::Rgb(0xa6, 0xe3, 0xa1));
    assert_eq!(theme.badge_window, Color::Rgb(0x89, 0xdc, 0xeb));
}

#[test]
fn test_resolved_theme_catppuccin_flavors() {
    let mut macchiato_cfg = ThemeConfig::default();
    macchiato_cfg.name = "catppuccin-macchiato".to_owned();
    let macchiato = ResolvedTheme::from_config(&macchiato_cfg);
    assert_eq!(macchiato.border, Color::Rgb(0x8a, 0xad, 0xf4));
    assert_eq!(macchiato.badge_window, Color::Rgb(0x91, 0xd7, 0xe3));

    let mut frappe_cfg = ThemeConfig::default();
    frappe_cfg.name = "catppuccin-frappe".to_owned();
    let frappe = ResolvedTheme::from_config(&frappe_cfg);
    assert_eq!(frappe.border, Color::Rgb(0x8c, 0xaa, 0xee));
    assert_eq!(frappe.badge_window, Color::Rgb(0x99, 0xd1, 0xdb));

    let mut latte_cfg = ThemeConfig::default();
    latte_cfg.name = "catppuccin-latte".to_owned();
    let latte = ResolvedTheme::from_config(&latte_cfg);
    assert_eq!(latte.border, Color::Rgb(0x1e, 0x66, 0xf5));
    assert_eq!(latte.badge_window, Color::Rgb(0x04, 0xa5, 0xe5));
}

#[test]
fn test_resolved_theme_badge_window_override() {
    let mut cfg = ThemeConfig::default();
    cfg.badge_window = Some("magenta".to_owned());
    let theme = ResolvedTheme::from_config(&cfg);
    assert_eq!(theme.badge_window, Color::Magenta);
}
