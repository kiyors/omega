use omega_config::Config;
use omega_daemon::{
    AUTOSAVE_INTERVAL, DaemonClient, DaemonServer, autosave_due, fetch_candidates_or_fallback,
};
use omega_session::{SessionCandidate, SessionSource};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[cfg(unix)]
fn spawn_server(
    sock_path: &std::path::Path,
    candidates: Vec<SessionCandidate>,
) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
    let server = DaemonServer::new_with_candidates(sock_path.to_path_buf(), candidates);
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    let handle = std::thread::spawn(move || {
        let _ = server.listen_until(r_clone);
    });

    // Wait for the socket to accept connections.
    let mut client_opt = None;
    for _ in 0..100 {
        if let Some(c) = DaemonClient::connect(Some(sock_path)) {
            client_opt = Some(c);
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(client_opt.is_some(), "daemon client connected");
    (handle, running)
}

#[test]
fn test_autosave_due_pure_logic() {
    let now = Instant::now();
    let recent = now - Duration::from_secs(60);
    let stale = now - AUTOSAVE_INTERVAL - Duration::from_secs(1);

    assert!(!autosave_due(recent, now), "recent save is not yet due");
    assert!(autosave_due(stale, now), "expired save is due");
    assert_eq!(
        AUTOSAVE_INTERVAL,
        Duration::from_secs(15 * 60),
        "periodic auto-save defaults to 15 minutes"
    );
}

#[test]
fn test_fallback_when_daemon_not_running() {
    let tmp = tempdir().expect("tempdir");
    let missing_sock = tmp.path().join("does-not-exist.sock");

    assert!(DaemonClient::connect(Some(&missing_sock)).is_none());

    // Direct scan fallback must not error even without any sessions.
    let candidates = fetch_candidates_or_fallback(Some(&missing_sock), &Config::default());
    assert!(candidates.iter().all(|c| !c.name.is_empty()));
}

#[test]
fn test_stale_socket_file_falls_back_cleanly() {
    let tmp = tempdir().expect("tempdir");
    // Socket file exists but nothing is listening (daemon killed uncleanly).
    let stale_sock = tmp.path().join("stale.sock");
    std::fs::write(&stale_sock, b"not-a-real-socket").expect("write stale socket");

    assert!(
        DaemonClient::connect(Some(&stale_sock)).is_none(),
        "connect to a stale socket must fail cleanly"
    );
    let candidates = fetch_candidates_or_fallback(Some(&stale_sock), &Config::default());
    assert!(candidates.iter().all(|c| !c.name.is_empty()));
}

#[test]
#[cfg(unix)]
fn test_daemon_ping_and_get_candidates_latency() {
    let tmp = tempdir().expect("tempdir");
    let sock_path = tmp.path().join("omega-test.sock");

    let candidates = vec![
        SessionCandidate::new("session-1", "/path/1", SessionSource::Tmux),
        SessionCandidate::new("session-2", "/path/2", SessionSource::Git),
    ];

    let (server_handle, running) = spawn_server(&sock_path, candidates.clone());
    let mut client = DaemonClient::connect(Some(&sock_path)).expect("client");

    let status = client.ping().expect("ping");
    assert!(status.running);
    assert_eq!(status.candidate_count, 2);

    // Hot cache: responses over the Unix socket are sub-millisecond in practice.
    let start = Instant::now();
    let fetched = client.get_candidates().expect("get candidates");
    assert!(start.elapsed() < Duration::from_millis(5));
    assert_eq!(fetched, candidates);

    running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();
}

#[test]
#[cfg(unix)]
fn test_fetch_candidates_prefers_daemon_when_available() {
    let tmp = tempdir().expect("tempdir");
    let sock_path = tmp.path().join("omega-priority.sock");

    let candidates = vec![SessionCandidate::new(
        "daemon-only",
        "/daemon/path",
        SessionSource::Tmux,
    )];

    let (server_handle, running) = spawn_server(&sock_path, candidates.clone());

    // The daemon path must win over the direct synchronous scan.
    let fetched = fetch_candidates_or_fallback(Some(&sock_path), &Config::default());
    assert_eq!(fetched, candidates);

    running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();
}

#[test]
#[cfg(unix)]
fn test_daemon_graceful_stop_unlinks_socket() {
    let tmp = tempdir().expect("tempdir");
    let sock_path = tmp.path().join("omega-stop.sock");

    let (server_handle, _running) = spawn_server(&sock_path, Vec::new());
    let mut client = DaemonClient::connect(Some(&sock_path)).expect("client");

    assert!(sock_path.exists(), "socket exists while running");
    client.stop().expect("send stop");
    let _ = server_handle.join();

    assert!(
        !sock_path.exists(),
        "socket must be unlinked on graceful shutdown"
    );
}
