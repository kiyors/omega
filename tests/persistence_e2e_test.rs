//! End-to-end Phase 3 verification against a real, isolated tmux server.
//!
//! Each test runs inside a hermetic environment:
//! - `TMUX_TMPDIR` points at a temp dir, isolating the tmux server socket from
//!   the developer's real server.
//! - A `tmux` shim earlier on `PATH` injects `-f /dev/null` into every tmux
//!   invocation, including the ones omega and the daemon spawn internally, so
//!   user tmux configuration (session-rename hooks, base-index overrides)
//!   cannot influence the isolated server.
//! - `TMUX` is removed so the `tmux` client resolves the isolated server.
//! - `XDG_DATA_HOME` / `XDG_RUNTIME_DIR` point at temp dirs, isolating
//!   snapshots, the daemon socket, and the daemon pid file.
//!
//! Tests are skipped gracefully when tmux is unavailable (e.g. Windows CI).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const OMEGA_BIN: &str = env!("CARGO_BIN_EXE_omega");

struct HermeticEnv {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    shim_dir: PathBuf,
    data_dir: PathBuf,
    runtime_dir: PathBuf,
    project: PathBuf,
}

fn tmux_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Sets up the hermetic environment; returns None when the test must skip.
fn setup() -> Option<HermeticEnv> {
    if !cfg!(unix) || !tmux_available() {
        eprintln!("skipping e2e: requires unix + tmux");
        return None;
    }

    let real_tmux = Command::new("sh")
        .args(["-c", "command -v tmux"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|p| !p.is_empty())?;

    let tmp = tempfile::tempdir().ok()?;
    let root = tmp.path().to_path_buf();
    let project = root.join("project");
    fs::create_dir_all(project.join("src")).ok()?;
    let data_dir = root.join("data");
    fs::create_dir_all(&data_dir).ok()?;
    let runtime_dir = root.join("runtime");
    fs::create_dir_all(&runtime_dir).ok()?;

    // Shim that forces an empty tmux config for every tmux invocation.
    let shim_dir = root.join("bin");
    fs::create_dir_all(&shim_dir).ok()?;
    let shim = shim_dir.join("tmux");
    fs::write(
        &shim,
        format!("#!/bin/sh\nexec '{real_tmux}' -f /dev/null \"$@\"\n"),
    )
    .ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&shim, fs::Permissions::from_mode(0o755));
    }

    Some(HermeticEnv {
        _tmp: tmp,
        root,
        shim_dir,
        data_dir,
        runtime_dir,
        project,
    })
}

impl HermeticEnv {
    fn tmux(&self, args: &[&str]) -> Output {
        Command::new("tmux")
            .args(args)
            .env_remove("TMUX")
            .env("TMUX_TMPDIR", &self.root)
            .env("PATH", self.shim_path())
            .output()
            .expect("spawn tmux")
    }

    fn omega(&self, args: &[&str]) -> Output {
        Command::new(OMEGA_BIN)
            .args(args)
            .env_remove("TMUX")
            .env("TMUX_TMPDIR", &self.root)
            .env("XDG_DATA_HOME", &self.data_dir)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("PATH", self.shim_path())
            .output()
            .expect("spawn omega")
    }

    fn shim_path(&self) -> String {
        let base = std::env::var("PATH").unwrap_or_default();
        format!("{}:{base}", self.shim_dir.display())
    }

    /// Runs a tmux command, retrying briefly to absorb tmux server-start
    /// latency (freshly spawned servers need a moment to accept queries).
    fn tmux_ok(&self, args: &[&str]) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let out = self.tmux(args);
            if out.status.success() {
                return String::from_utf8_lossy(&out.stdout).into_owned();
            }
            let stderr = String::from_utf8_lossy(&out.stderr);
            // A prior attempt actually landed.
            if stderr.contains("duplicate session") {
                return String::new();
            }
            if Instant::now() >= deadline {
                panic!("tmux {args:?} failed: {stderr}");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn omega_ok(&self, args: &[&str]) -> String {
        let out = self.omega(args);
        assert!(
            out.status.success(),
            "omega {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Kills every session on the isolated tmux server.
    fn kill_all_sessions(&self) {
        let _ = self.tmux(&["kill-server"]);
        for _ in 0..100 {
            if !self.tmux(&["list-sessions"]).status.success() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn snapshot_dir(&self) -> PathBuf {
        self.data_dir.join("omega/snapshots")
    }
}

#[test]
#[cfg(unix)]
fn test_save_kill_all_selective_and_full_restore_roundtrip() {
    let Some(env) = setup() else { return };

    // Two sessions; `alpha` has a two-pane second window with distinct paths.
    env.tmux_ok(&[
        "new-session",
        "-ds",
        "alpha",
        "-c",
        &env.project.display().to_string(),
    ]);
    env.tmux_ok(&[
        "new-window",
        "-t",
        "alpha",
        "-n",
        "logs",
        "-c",
        &env.project.display().to_string(),
    ]);
    env.tmux_ok(&[
        "split-window",
        "-t",
        "alpha:logs",
        "-c",
        &env.project.join("src").display().to_string(),
    ]);
    env.tmux_ok(&[
        "new-session",
        "-ds",
        "beta",
        "-c",
        &env.root.display().to_string(),
    ]);

    // Geometry captured before the wipe.
    let alpha_windows = env.tmux_ok(&["list-windows", "-t", "alpha", "-F", "#{window_name}"]);
    let logs_panes = env.tmux_ok(&[
        "list-panes",
        "-s",
        "-t",
        "alpha:logs",
        "-F",
        "#{pane_current_path}",
    ]);
    let size = env.tmux_ok(&[
        "display-message",
        "-p",
        "-t",
        "alpha",
        "#{window_width}x#{window_height}",
    ]);

    // --- save ---
    env.omega_ok(&["save", "--name", "e2e"]);
    let snap = env.snapshot_dir().join("e2e.json");
    assert!(
        snap.exists(),
        "named snapshot written to {}",
        snap.display()
    );
    let snap_json = fs::read_to_string(&snap).expect("read snapshot");
    assert!(
        snap_json.contains("\"alpha\""),
        "snapshot contains alpha; got: {snap_json}"
    );
    assert!(snap_json.contains("\"beta\""), "snapshot contains beta");
    assert!(
        snap_json.contains("\"layout\""),
        "snapshot preserves native layout strings"
    );

    // --- kill everything ---
    env.kill_all_sessions();

    // --- selective restore: only alpha ---
    env.omega_ok(&["restore", "alpha", "--name", "e2e"]);
    let sessions = env.tmux_ok(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("alpha"), "alpha restored: {sessions}");
    assert!(
        !sessions.contains("beta"),
        "beta must not be restored selectively: {sessions}"
    );

    // Alpha geometry matches the captured state.
    let restored_windows = env.tmux_ok(&["list-windows", "-t", "alpha", "-F", "#{window_name}"]);
    assert_eq!(restored_windows, alpha_windows, "window hierarchy restored");
    let restored_logs_panes = env.tmux_ok(&[
        "list-panes",
        "-s",
        "-t",
        "alpha:logs",
        "-F",
        "#{pane_current_path}",
    ]);
    assert_eq!(
        restored_logs_panes, logs_panes,
        "pane working directories restored"
    );
    let restored_size = env.tmux_ok(&[
        "display-message",
        "-p",
        "-t",
        "alpha",
        "#{window_width}x#{window_height}",
    ]);
    assert_eq!(restored_size, size, "window geometry restored");

    // --- full restore: beta appears, alpha is skipped as running ---
    let out = env.omega_ok(&["restore", "--name", "e2e"]);
    let sessions = env.tmux_ok(&["list-sessions", "-F", "#{session_name}"]);
    assert!(
        sessions.contains("alpha") && sessions.contains("beta"),
        "both sessions present after full restore: {sessions}"
    );
    assert!(
        out.contains("alpha"),
        "running session reported as skipped: {out}"
    );

    // --- overwrite forcibly recreates the running session ---
    env.omega_ok(&["restore", "--name", "e2e", "--overwrite"]);
    let sessions = env.tmux_ok(&["list-sessions", "-F", "#{session_name}"]);
    assert!(
        sessions.contains("alpha") && sessions.contains("beta"),
        "sessions survive overwrite restore: {sessions}"
    );

    // --- restore without a tag falls back to the latest pointer ---
    env.omega_ok(&["save"]); // auto-save updates latest.json
    env.omega_ok(&["restore", "alpha"]); // latest snapshot, selective
    let sessions = env.tmux_ok(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("alpha"));

    env.kill_all_sessions();
}

#[test]
#[cfg(unix)]
fn test_daemon_lifecycle_cached_list_and_fallback() {
    let Some(env) = setup() else { return };

    env.tmux_ok(&[
        "new-session",
        "-ds",
        "live",
        "-c",
        &env.project.display().to_string(),
    ]);

    // Always attempt daemon cleanup, even if an assertion fails mid-test.
    struct DaemonGuard<'a> {
        env: &'a crate::HermeticEnv,
    }
    impl Drop for DaemonGuard<'_> {
        fn drop(&mut self) {
            let _ = self.env.omega(&["daemon", "stop"]);
            self.env.kill_all_sessions();
        }
    }
    let _guard = DaemonGuard { env: &env };

    // --- daemon start ---
    env.omega_ok(&["daemon", "start"]);
    let status = env.omega_ok(&["daemon", "status"]);
    assert!(
        status.contains("is running"),
        "status after start: {status}"
    );

    // --- list resolves through the daemon socket ---
    let list = env.omega_ok(&["list", "--json"]);
    let items: Vec<serde_json::Value> =
        serde_json::from_str(&list).expect("list --json returns a JSON array");
    assert!(
        items.iter().any(|v| v["name"] == "live"),
        "daemon-cached list contains the live tmux session: {list}"
    );

    // --- graceful stop (server auto-saves on exit) ---
    env.omega_ok(&["daemon", "stop"]);
    let status = env.omega_ok(&["daemon", "status"]);
    assert!(
        status.contains("not running"),
        "status after stop: {status}"
    );

    // --- fallback: direct synchronous scan still works ---
    let list = env.omega_ok(&["list", "--json"]);
    let items: Vec<serde_json::Value> =
        serde_json::from_str(&list).expect("fallback list returns a JSON array");
    assert!(
        items.iter().any(|v| v["name"] == "live"),
        "fallback list still discovers the live session: {list}"
    );

    // The daemon's graceful stop wrote a final auto-snapshot.
    let snapshots: Vec<PathBuf> = fs::read_dir(env.snapshot_dir())
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    assert!(
        snapshots.iter().any(|p| p
            .file_name()
            .is_some_and(|n| n.to_string_lossy().contains("snapshot-"))),
        "daemon stop auto-saved a snapshot: {:?}",
        snapshots
    );
}

#[test]
#[cfg(unix)]
fn test_daemon_stop_is_idempotent() {
    let Some(env) = setup() else { return };
    // No daemon running: stop must succeed without side effects.
    env.omega_ok(&["daemon", "stop"]);
    let status = env.omega_ok(&["daemon", "status"]);
    assert!(status.contains("not running"));
}
