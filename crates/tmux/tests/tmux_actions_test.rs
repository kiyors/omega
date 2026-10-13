use omega_tmux::{MockTmuxClient, TmuxActionRunner};
use std::path::Path;

#[test]
fn test_connect_existing_session_inside_tmux() {
    let mut mock = MockTmuxClient::new();
    mock.inside_tmux = true;
    mock.existing_sessions.push("omega".to_string());

    let runner = TmuxActionRunner::new();
    let result = runner.connect(&mut mock, "omega", Path::new("/path/to/omega"), None);

    assert!(result.is_ok());
    assert_eq!(mock.switched_to.as_deref(), Some("omega"));
    assert_eq!(mock.attached_to, None);
    assert_eq!(mock.created_sessions.len(), 0);
}

#[test]
fn test_connect_existing_session_outside_tmux() {
    let mut mock = MockTmuxClient::new();
    mock.inside_tmux = false;
    mock.existing_sessions.push("omega".to_string());

    let runner = TmuxActionRunner::new();
    let result = runner.connect(&mut mock, "omega", Path::new("/path/to/omega"), None);

    assert!(result.is_ok());
    assert_eq!(mock.attached_to.as_deref(), Some("omega"));
    assert_eq!(mock.switched_to, None);
    assert_eq!(mock.created_sessions.len(), 0);
}

#[test]
fn test_connect_creates_session_if_missing() {
    let mut mock = MockTmuxClient::new();
    mock.inside_tmux = true;

    let runner = TmuxActionRunner::new();
    let result = runner.connect(
        &mut mock,
        "new-session",
        Path::new("/path/to/new"),
        Some("cargo check"),
    );

    assert!(result.is_ok());
    assert_eq!(mock.created_sessions.len(), 1);
    assert_eq!(mock.created_sessions[0].name, "new-session");
    assert_eq!(
        mock.created_sessions[0].command.as_deref(),
        Some("cargo check")
    );
    assert_eq!(mock.switched_to.as_deref(), Some("new-session"));
}

#[test]
fn test_kill_session() {
    let mut mock = MockTmuxClient::new();
    mock.existing_sessions.push("doomed".to_string());

    let runner = TmuxActionRunner::new();
    let result = runner.kill(&mut mock, "doomed");

    assert!(result.is_ok());
    assert_eq!(mock.killed_sessions, vec!["doomed"]);
}

#[test]
fn test_switch_last_session() {
    let mut mock = MockTmuxClient::new();
    mock.inside_tmux = true;

    let runner = TmuxActionRunner::new();
    let result = runner.last(&mut mock);

    assert!(result.is_ok());
    assert!(mock.switched_last_called);
}

#[test]
fn test_switch_last_session_outside_tmux() {
    let mut mock = MockTmuxClient::new();
    mock.inside_tmux = false;
    mock.existing_sessions.push("prior-session".to_string());

    let runner = TmuxActionRunner::new();
    let result = runner.last(&mut mock);

    assert!(result.is_ok());
    assert_eq!(mock.attached_to.as_deref(), Some("prior-session"));
}

#[test]
fn test_rename_session() {
    use omega_tmux::TmuxClient;

    let mut mock = MockTmuxClient::new();
    let result = mock.rename_session(Some("old-name"), "new-name");
    assert!(result.is_ok());
    assert_eq!(
        mock.renamed_sessions,
        vec![(Some("old-name".to_string()), "new-name".to_string())]
    );

    let result_current = mock.rename_session(None, "active-renamed");
    assert!(result_current.is_ok());
    assert_eq!(
        mock.renamed_sessions.get(1),
        Some(&(None, "active-renamed".to_string()))
    );
}
