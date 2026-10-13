use omega_daemon::{
    PaneSnapshot, SessionSnapshotRecord, Snapshot, SnapshotManager, WindowSnapshot,
};
use omega_tmux::MockTmuxClient;
use std::path::{Path, PathBuf};

fn window(index: usize, name: &str, layout: &str, panes: Vec<PaneSnapshot>) -> WindowSnapshot {
    WindowSnapshot {
        index,
        name: name.to_string(),
        layout: layout.to_string(),
        active: true,
        panes,
    }
}

fn pane(index: usize, path: &str, command: &str) -> PaneSnapshot {
    PaneSnapshot {
        index,
        path: PathBuf::from(path),
        command: command.to_string(),
        active: index == 1,
    }
}

fn two_pane_session() -> SessionSnapshotRecord {
    SessionSnapshotRecord {
        name: "work".to_string(),
        path: PathBuf::from("/repo/work"),
        windows: vec![window(
            1,
            "editor",
            "bb62,80x24,0,0,1[80x12,0,0,2,80x11,0,13,3]",
            vec![
                pane(1, "/repo/work", "nvim"),
                pane(2, "/repo/work/crates", "zsh"),
            ],
        )],
    }
}

#[test]
fn test_restore_non_destructive_skips_running_sessions() {
    let mut mock = MockTmuxClient::new();
    mock.existing_sessions = vec!["work".to_string()];

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![
            two_pane_session(),
            SessionSnapshotRecord {
                name: "notes".to_string(),
                path: PathBuf::from("/repo/notes"),
                windows: vec![window(
                    1,
                    "main",
                    "aa10,80x24,0,0,4",
                    vec![pane(1, "/repo/notes", "zsh")],
                )],
            },
        ],
    };

    let summary = SnapshotManager::new()
        .restore(&mut mock, &snapshot, None, false)
        .expect("restore");

    assert_eq!(summary.skipped, vec!["work"]);
    assert_eq!(summary.restored, vec!["notes"]);
    assert!(mock.created_sessions.iter().any(|c| c.name == "notes"));
    assert!(!mock.created_sessions.iter().any(|c| c.name == "work"));
}

#[test]
fn test_restore_overwrite_recreates_running_sessions() {
    let mut mock = MockTmuxClient::new();
    mock.existing_sessions = vec!["work".to_string()];

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![two_pane_session()],
    };

    let summary = SnapshotManager::new()
        .restore(&mut mock, &snapshot, None, true)
        .expect("restore with overwrite");

    assert!(summary.skipped.is_empty());
    assert_eq!(summary.restored, vec!["work"]);
    assert!(mock.killed_sessions.contains(&"work".to_string()));
    assert!(mock.created_sessions.iter().any(|c| c.name == "work"));
}

#[test]
fn test_selective_restore_single_session() {
    let mut mock = MockTmuxClient::new();

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![
            SessionSnapshotRecord {
                name: "alpha".to_string(),
                path: PathBuf::from("/repo/alpha"),
                windows: vec![],
            },
            two_pane_session(),
            SessionSnapshotRecord {
                name: "gamma".to_string(),
                path: PathBuf::from("/repo/gamma"),
                windows: vec![],
            },
        ],
    };

    let summary = SnapshotManager::new()
        .restore(&mut mock, &snapshot, Some("work"), false)
        .expect("selective restore");

    assert_eq!(summary.restored, vec!["work"]);
    assert_eq!(mock.created_sessions.len(), 1);
    assert_eq!(mock.created_sessions[0].name, "work");
}

#[test]
fn test_restore_rebuilds_windows_splits_layouts_and_commands() {
    let mut mock = MockTmuxClient::new();

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![SessionSnapshotRecord {
            name: "work".to_string(),
            path: PathBuf::from("/repo/work"),
            windows: vec![
                window(
                    1,
                    "editor",
                    "bb62,80x24,0,0,1[80x12,0,0,2,80x11,0,13,3]",
                    vec![
                        pane(1, "/repo/work", "nvim"),
                        pane(2, "/repo/work/crates", "zsh"),
                    ],
                ),
                window(
                    2,
                    "server",
                    "5c4b,80x24,0,0,5",
                    vec![
                        pane(1, "/repo/work", "cargo watch -x run"),
                        pane(2, "/repo/work/api", "docker compose logs -f"),
                    ],
                ),
            ],
        }],
    };

    SnapshotManager::new()
        .restore(&mut mock, &snapshot, None, false)
        .expect("restore");

    // Session created at the first pane path with the recorded window name.
    assert_eq!(mock.created_sessions.len(), 1);
    let created = &mock.created_sessions[0];
    assert_eq!(created.name, "work");
    assert_eq!(created.window_name, "editor");
    assert_eq!(created.path, Path::new("/repo/work"));

    // Second window created at its first pane path.
    assert_eq!(mock.created_windows.len(), 1);
    assert_eq!(mock.created_windows[0].0, "work");
    assert_eq!(mock.created_windows[0].1, "server");
    assert_eq!(mock.created_windows[0].2, PathBuf::from("/repo/work"));

    // One extra pane split per multi-pane window.
    assert_eq!(mock.split_panes.len(), 2);
    assert_eq!(mock.split_panes[0].1, PathBuf::from("/repo/work/crates"));
    assert_eq!(mock.split_panes[1].1, PathBuf::from("/repo/work/api"));

    // Native layout strings applied per window.
    assert_eq!(mock.applied_layouts.len(), 2);
    assert_eq!(
        mock.applied_layouts[0].1,
        "bb62,80x24,0,0,1[80x12,0,0,2,80x11,0,13,3]"
    );
    assert_eq!(mock.applied_layouts[1].1, "5c4b,80x24,0,0,5");

    // Pane commands replayed; shells skipped (zsh must not be re-spawned).
    assert_eq!(mock.sent_keys.len(), 3);
    let commands: Vec<&str> = mock.sent_keys.iter().map(|(_, c)| c.as_str()).collect();
    assert!(commands.contains(&"nvim"));
    assert!(commands.contains(&"cargo watch -x run"));
    assert!(commands.contains(&"docker compose logs -f"));
    assert!(!commands.contains(&"zsh"));
}

#[test]
fn test_selective_restore_unknown_session_errors() {
    let mut mock = MockTmuxClient::new();

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![SessionSnapshotRecord {
            name: "known".to_string(),
            path: PathBuf::from("/repo/known"),
            windows: vec![],
        }],
    };

    let err = SnapshotManager::new()
        .restore(&mut mock, &snapshot, Some("ghost"), false)
        .expect_err("unknown selective target must error");
    assert!(
        err.to_string().contains("ghost"),
        "error names the missing session: {err}"
    );
    assert!(
        mock.created_sessions.is_empty(),
        "no session may be created for an unknown target"
    );
}

#[test]
fn test_restore_session_without_windows_creates_bare_session() {
    let mut mock = MockTmuxClient::new();

    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![SessionSnapshotRecord {
            name: "bare".to_string(),
            path: PathBuf::from("/repo/bare"),
            windows: vec![],
        }],
    };

    let summary = SnapshotManager::new()
        .restore(&mut mock, &snapshot, None, false)
        .expect("restore bare");

    assert_eq!(summary.restored, vec!["bare"]);
    assert_eq!(mock.created_sessions.len(), 1);
    assert_eq!(mock.created_sessions[0].path, PathBuf::from("/repo/bare"));
}

#[test]
fn test_restore_preserves_active_window_and_pane_focus() {
    let mut mock = MockTmuxClient::new();
    let snapshot = Snapshot {
        version: 1,
        created_at: "2026-10-13T07:00:00Z".to_string(),
        sessions: vec![two_pane_session()],
    };

    SnapshotManager::new()
        .restore(&mut mock, &snapshot, None, false)
        .expect("restore focus");

    assert!(!mock.selected_windows.is_empty());
    assert_eq!(mock.selected_windows[0], "work:1");
    assert!(!mock.selected_panes.is_empty());
    assert_eq!(mock.selected_panes[0], "work:1.1");
}
