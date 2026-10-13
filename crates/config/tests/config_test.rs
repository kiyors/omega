use omega_config::Config;
use tempfile::tempdir;

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert!(cfg.hide_attached);
    assert_eq!(cfg.dir_length, 1);
    assert!(cfg.tui.preview);
    assert_eq!(cfg.tui.preview_width, 55);
    assert_eq!(cfg.tui.preview_border, "rounded");
    assert!(cfg.aliases.is_empty());
    assert!(cfg.wildcards.is_empty());
}

#[test]
fn test_load_from_json_path() {
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("config.json");
    let json_content = r#"{
        "hide_attached": false,
        "dir_length": 3,
        "aliases": [
            {
                "name": "work",
                "path": "/workspace",
                "icon": "W"
            }
        ]
    }"#;
    std::fs::write(&json_path, json_content).unwrap();

    let cfg = Config::load_from_path(&json_path).expect("should load valid json config");
    assert!(!cfg.hide_attached);
    assert_eq!(cfg.dir_length, 3);
    assert_eq!(cfg.aliases.len(), 1);
    assert_eq!(cfg.aliases[0].name, "work");
    assert_eq!(cfg.aliases[0].path, "/workspace");
    assert_eq!(cfg.aliases[0].icon.as_deref(), Some("W"));
}
#[test]
fn test_tui_popup_and_customization_config_defaults_and_override() {
    let cfg = Config::default();
    assert!(cfg.tui.popup);
    assert_eq!(cfg.tui.popup_width, 85);
    assert_eq!(cfg.tui.popup_height, 80);
    assert_eq!(cfg.tui.popup_border, "rounded");
    assert_eq!(cfg.tui.border_color, "cyan");
    assert_eq!(cfg.tui.highlight_color, "yellow");
    assert_eq!(cfg.tui.search_prompt, "> ");
    assert!(cfg.tui.show_badges);
    assert!(cfg.tui.show_icons);
    assert!(cfg.tui.show_path);
    assert!(cfg.tui.show_footer);

    let dir = tempdir().unwrap();
    let json_path = dir.path().join("config.json");
    let json_content = r#"{
        "tui": {
            "popup": false,
            "popup_width": 70,
            "popup_height": 60,
            "popup_border": "double",
            "border_color": "magenta",
            "highlight_color": "green",
            "search_prompt": "find: ",
            "show_badges": false,
            "show_icons": false,
            "show_path": false,
            "show_footer": false
        }
    }"#;
    std::fs::write(&json_path, json_content).unwrap();

    let loaded = Config::load_from_path(&json_path).expect("should load custom tui options");
    assert!(!loaded.tui.popup);
    assert_eq!(loaded.tui.popup_width, 70);
    assert_eq!(loaded.tui.popup_height, 60);
    assert_eq!(loaded.tui.popup_border, "double");
    assert_eq!(loaded.tui.border_color, "magenta");
    assert_eq!(loaded.tui.highlight_color, "green");
    assert_eq!(loaded.tui.search_prompt, "find: ");
    assert!(!loaded.tui.show_badges);
    assert!(!loaded.tui.show_icons);
    assert!(!loaded.tui.show_path);
    assert!(!loaded.tui.show_footer);
}
