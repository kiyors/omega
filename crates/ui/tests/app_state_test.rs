use omega_config::TuiConfig;
use omega_session::{SessionCandidate, SessionSource};
use omega_ui::app::App;

#[test]
fn test_app_initialization_and_filtering() {
    let items = vec![
        SessionCandidate::new("alpha", "/path/alpha", SessionSource::Tmux),
        SessionCandidate::new("beta", "/path/beta", SessionSource::Git),
        SessionCandidate::new("gamma", "/path/gamma", SessionSource::Alias),
    ];

    let mut app = App::new(items, None);
    assert_eq!(app.filtered.len(), 3);
    assert_eq!(app.selected, 0);
    assert!(app.popup_enabled);
    assert_eq!(app.popup_width, 85);
    assert_eq!(app.popup_height, 80);

    // Apply custom TUI config
    let mut custom_tui = TuiConfig::default();
    custom_tui.popup = false;
    custom_tui.popup_width = 75;
    custom_tui.search_prompt = "search> ".to_string();
    app.apply_tui_config(&custom_tui);

    assert!(!app.popup_enabled);
    assert_eq!(app.popup_width, 75);
    assert_eq!(app.search_prompt, "search> ");

    // Test query filtering
    app.query = "bet".to_string();
    app.refilter();
    assert_eq!(app.filtered.len(), 1);
    assert_eq!(app.selected_item().unwrap().name, "beta");

    // Test navigation bounds
    app.select_next();
    assert_eq!(app.selected, 0); // Only 1 item, so stays at 0
    app.select_prev();
    assert_eq!(app.selected, 0);
}

#[test]
fn test_app_preview_toggling_and_scrolling() {
    let items = vec![SessionCandidate::new("one", "/p1", SessionSource::Tmux)];
    let mut app = App::new(items, None);

    assert!(app.preview_enabled);
    app.toggle_preview();
    assert!(!app.preview_enabled);
    app.toggle_preview();
    assert!(app.preview_enabled);

    assert_eq!(app.preview_scroll, 0);
    app.scroll_preview_down();
    assert_eq!(app.preview_scroll, 1);
    app.scroll_preview_up();
    assert_eq!(app.preview_scroll, 0);
    app.scroll_preview_up(); // Does not underflow
    assert_eq!(app.preview_scroll, 0);
}
