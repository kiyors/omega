use omega::app::App;
use omega::ui::{compute_horizontal_chunks, should_render_preview};
use omega_config::Config;
use omega_session::{SessionCandidate, SessionSource};
use ratatui::layout::Rect;

#[test]
fn test_config_tui_defaults() {
    let cfg = Config::default();
    assert!(cfg.tui.preview);
    assert_eq!(cfg.tui.preview_width, 55);
    assert_eq!(cfg.tui.preview_border, "rounded");
}

#[test]
fn test_config_load_from_path() {
    let tmp = tempfile::tempdir().unwrap();
    let file_path = tmp.path().join("config.json");
    let json_content = r#"{
        "hide_attached": false,
        "dir_length": 2,
        "tui": {
            "preview": false,
            "preview_width": 40,
            "preview_border": "double"
        }
    }"#;
    std::fs::write(&file_path, json_content).unwrap();

    let cfg = Config::load_from_path(&file_path).unwrap();
    assert!(!cfg.hide_attached);
    assert_eq!(cfg.dir_length, 2);
    assert!(!cfg.tui.preview);
    assert_eq!(cfg.tui.preview_width, 40);
    assert_eq!(cfg.tui.preview_border, "double");
}

#[test]
fn test_app_preview_state_and_scrolling() {
    let items = vec![
        SessionCandidate::new("s1", "/p1", SessionSource::Tmux),
        SessionCandidate::new("s2", "/p2", SessionSource::Git),
    ];

    let mut app = App::new(items, None);
    assert!(app.preview_enabled);
    assert_eq!(app.preview_scroll, 0);

    // Scroll down preview
    app.scroll_preview_down();
    assert_eq!(app.preview_scroll, 1);
    app.scroll_preview_down();
    assert_eq!(app.preview_scroll, 2);

    // Scroll up preview
    app.scroll_preview_up();
    assert_eq!(app.preview_scroll, 1);

    // Toggle preview off and on
    app.toggle_preview();
    assert!(!app.preview_enabled);
    app.toggle_preview();
    assert!(app.preview_enabled);

    // Changing selected session resets preview scroll to 0
    app.select_next();
    assert_eq!(app.selected, 1);
    assert_eq!(app.preview_scroll, 0);
}

#[test]
fn test_ui_responsive_collapse_and_split() {
    let app = App::new(vec![], None);

    // 1. Narrow terminal (e.g. 80 columns) -> should collapse preview
    let narrow_area = Rect::new(0, 0, 80, 24);
    assert!(!should_render_preview(&app, narrow_area));
    let chunks_narrow = compute_horizontal_chunks(&app, narrow_area);
    assert_eq!(chunks_narrow.len(), 1); // Only candidate list

    // 2. Wide terminal (e.g. 140 columns) with preview enabled -> should show 2 panes
    let wide_area = Rect::new(0, 0, 140, 24);
    assert!(should_render_preview(&app, wide_area));
    let chunks_wide = compute_horizontal_chunks(&app, wide_area);
    assert_eq!(chunks_wide.len(), 2);

    // Left pane ~ 45%, Right preview pane ~ 55%
    let left_width = chunks_wide[0].width;
    let right_width = chunks_wide[1].width;
    assert!(right_width > left_width);
    assert_eq!(left_width + right_width, 140);
}
