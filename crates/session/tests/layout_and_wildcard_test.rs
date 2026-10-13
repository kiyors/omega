use omega_config::{PaneLayoutConfig, WildcardRule, WindowLayoutConfig};
use omega_session::layout::{match_wildcard, provision_session_layout};
use omega_tmux::MockTmuxClient;
use std::path::Path;

#[test]
fn test_match_wildcard_patterns() {
    let rules = vec![
        WildcardRule {
            pattern: "/projects/rust/*".to_owned(),
            startup_command: Some("cargo check".to_owned()),
            icon: Some("🦀".to_owned()),
            windows: Vec::new(),
        },
        WildcardRule {
            pattern: "/work/**".to_owned(),
            startup_command: None,
            icon: Some("💼".to_owned()),
            windows: Vec::new(),
        },
    ];

    // Single-star match
    let match1 = match_wildcard(Path::new("/projects/rust/omega"), &rules);
    assert!(match1.is_some());
    assert_eq!(match1.unwrap().icon.as_deref(), Some("🦀"));

    // Single-star should not cross subdirectories
    let no_match1 = match_wildcard(Path::new("/projects/rust/deep/nested"), &rules);
    assert!(no_match1.is_none());

    // Double-star matches deeply
    let match2 = match_wildcard(Path::new("/work/client/subproject/app"), &rules);
    assert!(match2.is_some());
    assert_eq!(match2.unwrap().icon.as_deref(), Some("💼"));

    // Non-match
    let no_match2 = match_wildcard(Path::new("/other/random"), &rules);
    assert!(no_match2.is_none());
}

#[test]
fn test_provision_session_layout_single_window_panes_and_layout() {
    let mut tmux = MockTmuxClient::new();
    let windows = vec![WindowLayoutConfig {
        name: "server".to_owned(),
        command: None,
        layout: Some("main-vertical".to_owned()),
        panes: vec![
            PaneLayoutConfig {
                command: Some("cargo watch -x run".to_owned()),
                path: None,
            },
            PaneLayoutConfig {
                command: Some("docker compose logs -f".to_owned()),
                path: None,
            },
        ],
    }];

    provision_session_layout(&mut tmux, "backend", Path::new("/projects/api"), &windows)
        .expect("provisioning failed");

    // First window created
    assert_eq!(tmux.created_sessions.len(), 1);
    assert_eq!(tmux.created_sessions[0].name, "backend");
    assert_eq!(tmux.created_sessions[0].window_name, "server");

    // Second pane was split
    assert_eq!(tmux.split_panes.len(), 1);

    // Layout applied
    assert_eq!(tmux.applied_layouts.len(), 1);
    assert_eq!(tmux.applied_layouts[0].1, "main-vertical");

    // Commands sent to panes
    assert_eq!(tmux.sent_keys.len(), 2);
    assert_eq!(tmux.sent_keys[0].1, "cargo watch -x run");
    assert_eq!(tmux.sent_keys[1].1, "docker compose logs -f");
}

#[test]
fn test_provision_session_layout_multiple_windows() {
    let mut tmux = MockTmuxClient::new();
    let windows = vec![
        WindowLayoutConfig {
            name: "editor".to_owned(),
            command: Some("nvim".to_owned()),
            layout: None,
            panes: Vec::new(),
        },
        WindowLayoutConfig {
            name: "terminal".to_owned(),
            command: Some("zsh".to_owned()),
            layout: None,
            panes: Vec::new(),
        },
    ];

    provision_session_layout(
        &mut tmux,
        "my-session",
        Path::new("/projects/test"),
        &windows,
    )
    .expect("provisioning failed");

    assert_eq!(tmux.created_sessions.len(), 1);
    assert_eq!(tmux.created_sessions[0].window_name, "editor");
    assert_eq!(tmux.created_windows.len(), 1);
    assert_eq!(tmux.created_windows[0].1, "terminal");

    // Check commands
    assert!(tmux.sent_keys.iter().any(|(_, cmd)| cmd == "nvim"));
    assert!(tmux.sent_keys.iter().any(|(_, cmd)| cmd == "zsh"));
}
