use omega_daemon::SnapshotManager;
use omega_tmux::{
    MockTmuxClient, TmuxClient, TmuxPaneState, TmuxSessionState, TmuxWindowState,
    parse_session_state,
};
use std::path::Path;

#[test]
fn test_parse_session_state_full_tree() {
    let sessions = "work\t1\nscratch\t0\n";
    let windows = "work\t0\teditor\tbb62,80x23,0,0,0\t1\n\
                   work\t1\tserver\t5c4b,80x23,0,0[80x11,0,0,1,80x11,0,12,2]\t0\n\
                   scratch\t2\tlogs\taa10,80x23,0,0,3\t1\n";
    let panes = "work\t0\t0\t/Users/dev/work\tnvim\t1\n\
                 work\t1\t0\t/Users/dev/work\tzsh\t1\n\
                 work\t1\t1\t/Users/dev/work/backend\tcargo\t0\n\
                 scratch\t2\t0\t/var/log\tless\t1\n";

    let state = parse_session_state(sessions, windows, panes);

    assert_eq!(state.len(), 2);

    let work = &state[0];
    assert_eq!(work.name, "work");
    assert!(work.attached);
    assert_eq!(work.windows.len(), 2);

    let editor = &work.windows[0];
    assert_eq!(editor.index, 0);
    assert_eq!(editor.name, "editor");
    assert_eq!(editor.layout, "bb62,80x23,0,0,0");
    assert!(editor.active);
    assert_eq!(editor.panes.len(), 1);
    assert_eq!(editor.panes[0].index, 0);
    assert_eq!(editor.panes[0].path, "/Users/dev/work");
    assert_eq!(editor.panes[0].command, "nvim");
    assert!(editor.panes[0].active);

    let server = &work.windows[1];
    assert_eq!(server.index, 1);
    assert_eq!(server.name, "server");
    assert_eq!(server.panes.len(), 2);
    assert_eq!(server.panes[0].command, "zsh");
    assert_eq!(server.panes[1].index, 1);
    assert_eq!(server.panes[1].path, "/Users/dev/work/backend");
    assert_eq!(server.panes[1].command, "cargo");

    let scratch = &state[1];
    assert_eq!(scratch.name, "scratch");
    assert!(!scratch.attached);
    assert_eq!(scratch.windows.len(), 1);
    assert_eq!(scratch.windows[0].index, 2);
}

#[test]
fn test_parse_session_state_skips_malformed_lines() {
    let sessions = "good\t1\n\nbad-no-attached\n";
    let windows = "good\t0\tmain\tlayout-a\t1\n\
                   good\tnotanindex\tbad\tlayout-b\t0\n\
                   good\t1\n\
                   ghost\t9\tnope\tlayout-c\t1\n";
    let panes = "good\t0\t0\t/path/ok\tzsh\t1\n\
                 good\t0\tx\t/path/bad\tbad\t0\n\
                 ghost\t9\t0\t/path/ghost\tzsh\t1\n";

    let state = parse_session_state(sessions, windows, panes);

    assert_eq!(state.len(), 1);
    assert_eq!(state[0].name, "good");
    assert_eq!(state[0].windows.len(), 1);
    assert_eq!(state[0].windows[0].name, "main");
    assert_eq!(state[0].windows[0].panes.len(), 1);
    assert_eq!(state[0].windows[0].panes[0].path, "/path/ok");
}

#[test]
fn test_parse_session_state_empty_input() {
    assert!(parse_session_state("", "", "").is_empty());

    // Session listed but no windows or panes reported yet.
    let state = parse_session_state("solo\t0\n", "", "");
    assert_eq!(state.len(), 1);
    assert!(state[0].windows.is_empty());
}

#[test]
fn test_parse_session_state_window_without_panes_is_preserved() {
    let state = parse_session_state("s\t0\n", "s\t0\tempty\tlayout-x\t1\n", "");
    assert_eq!(state.len(), 1);
    assert_eq!(state[0].windows.len(), 1);
    assert!(state[0].windows[0].panes.is_empty());
}

#[test]
fn test_mock_capture_state_returns_fixture() {
    let mut mock = MockTmuxClient::new();
    mock.snapshot_state.push(TmuxSessionState {
        name: "fixture".to_string(),
        attached: true,
        windows: vec![TmuxWindowState {
            index: 1,
            name: "main".to_string(),
            layout: "layout-fixture".to_string(),
            active: true,
            panes: vec![TmuxPaneState {
                index: 1,
                path: "/fixture".to_string(),
                command: "zsh".to_string(),
                active: true,
            }],
        }],
    });

    let captured = mock.capture_state().expect("capture_state");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].name, "fixture");
    assert_eq!(captured[0].windows[0].layout, "layout-fixture");
}

#[test]
fn test_capture_maps_state_tree_into_snapshot() {
    let mut mock = MockTmuxClient::new();
    mock.snapshot_state = vec![TmuxSessionState {
        name: "proj".to_string(),
        attached: true,
        windows: vec![
            TmuxWindowState {
                index: 1,
                name: "code".to_string(),
                layout: "bb62,159x48,0,0,1".to_string(),
                active: true,
                panes: vec![TmuxPaneState {
                    index: 1,
                    path: "/repo/proj".to_string(),
                    command: "nvim".to_string(),
                    active: true,
                }],
            },
            TmuxWindowState {
                index: 2,
                name: "server".to_string(),
                layout: "5c4b,80x23,0,0,2".to_string(),
                active: false,
                panes: vec![
                    TmuxPaneState {
                        index: 1,
                        path: "/repo/proj".to_string(),
                        command: "zsh".to_string(),
                        active: true,
                    },
                    TmuxPaneState {
                        index: 2,
                        path: "/repo/proj/api".to_string(),
                        command: "cargo".to_string(),
                        active: false,
                    },
                ],
            },
        ],
    }];

    let snapshot = SnapshotManager::new().capture(&mock).expect("capture");

    assert_eq!(snapshot.version, 1);
    assert!(!snapshot.created_at.is_empty());
    assert_eq!(snapshot.sessions.len(), 1);

    let session = &snapshot.sessions[0];
    assert_eq!(session.name, "proj");
    assert_eq!(session.path, Path::new("/repo/proj"));
    assert_eq!(session.windows.len(), 2);
    assert_eq!(session.windows[0].name, "code");
    assert_eq!(session.windows[0].layout, "bb62,159x48,0,0,1");
    assert_eq!(session.windows[1].panes.len(), 2);
    assert_eq!(
        session.windows[1].panes[1].path,
        Path::new("/repo/proj/api")
    );
    assert_eq!(session.windows[1].panes[1].command, "cargo");
}
