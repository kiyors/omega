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
