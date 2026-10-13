//! Background cache daemon.
//!
//! The daemon listens on a Unix domain socket (`$XDG_RUNTIME_DIR/omega.sock`
//! by default), keeps a hot in-memory candidate cache refreshed by background
//! polling, periodically auto-saves tmux snapshots, and snapshots state on
//! graceful termination. CLI and TUI consumers query the socket first and fall
//! back to direct synchronous scans when the daemon is not running.
//!
//! Socket transport is Unix-only; on other platforms `DaemonClient::connect`
//! always fails and `fetch_candidates_or_fallback` uses the direct scan.

use crate::collector::SessionCollector;
use crate::models::SessionCandidate;
use crate::snapshot::SnapshotStore;
use crate::tmux::{RealTmuxClient, SnapshotManager};
use omega_config::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Interval between periodic auto-saves (default: 15 minutes).
pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Interval between background candidate cache refreshes.
pub const CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Poll interval of the non-blocking accept loop.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(15);

/// How long `daemon_start` waits for the spawned server to become reachable.
const START_READY_TIMEOUT: Duration = Duration::from_secs(3);

/// How long `daemon_stop` waits for the server process to exit.
const STOP_EXIT_TIMEOUT: Duration = Duration::from_secs(3);

/// Pure auto-save scheduling rule: an auto-save is due once `AUTOSAVE_INTERVAL`
/// has elapsed since the previous save.
pub fn autosave_due(last_save: Instant, now: Instant) -> bool {
    now.saturating_duration_since(last_save) >= AUTOSAVE_INTERVAL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonRequest {
    Ping,
    GetCandidates,
    SaveSnapshot { tag: Option<String> },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: u32,
    pub uptime_secs: u64,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonResponse {
    Pong(DaemonStatus),
    Candidates(Vec<SessionCandidate>),
    SnapshotSaved(PathBuf),
    Stopped,
    Error(String),
}

/// Path of the daemon's pid file for a given socket path.
pub fn pid_path(socket_path: &Path) -> PathBuf {
    socket_path.with_extension("pid")
}

pub struct DaemonServer {
    socket_path: PathBuf,
    candidates: Arc<Mutex<Vec<SessionCandidate>>>,
    start_time: Instant,
    config: Config,
    /// Ensures the graceful-termination snapshot is written exactly once.
    final_saved: AtomicBool,
}

impl DaemonServer {
    pub fn new(socket_path: PathBuf, config: Config) -> Self {
        let collector = SessionCollector::new();
        let initial_candidates = collector.collect_from_system(&config);
        Self {
            socket_path,
            candidates: Arc::new(Mutex::new(initial_candidates)),
            start_time: Instant::now(),
            config,
            final_saved: AtomicBool::new(false),
        }
    }

    /// Test seam: builds a server around a fixed candidate list without
    /// touching live system state.
    pub fn new_with_candidates(socket_path: PathBuf, candidates: Vec<SessionCandidate>) -> Self {
        Self {
            socket_path,
            candidates: Arc::new(Mutex::new(candidates)),
            start_time: Instant::now(),
            config: Config::default(),
            final_saved: AtomicBool::new(false),
        }
    }

    /// Resolves the default socket path:
    /// 1. `$XDG_RUNTIME_DIR/omega.sock`
    /// 2. `/tmp/omega-<uid>.sock` (fallback)
    pub fn default_socket_path() -> PathBuf {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime_dir).join("omega.sock")
        } else if let Ok(user) = std::env::var("USER") {
            PathBuf::from(format!("/tmp/omega-{user}.sock"))
        } else {
            PathBuf::from("/tmp/omega.sock")
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Listens for client requests until the `running` flag becomes false or a
    /// termination signal arrives, then performs a final snapshot save.
    pub fn listen_until(&self, running: Arc<AtomicBool>) -> io::Result<()> {
        #[cfg(unix)]
        let terminate = register_termination_signals()?;
        #[cfg(unix)]
        while running.load(Ordering::SeqCst) && !terminate.load(Ordering::SeqCst) {
            self.serve_once(&running)?;
        }

        #[cfg(not(unix))]
        while running.load(Ordering::SeqCst) {
            self.serve_once(&running)?;
        }

        // Graceful termination: snapshot state exactly once on server exit.
        if !self.final_saved.swap(true, Ordering::SeqCst) {
            let _ = self.save_snapshot_now(None);
        }

        Ok(())
    }

    #[cfg(unix)]
    fn serve_once(&self, running: &Arc<AtomicBool>) -> io::Result<()> {
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }
        if let Some(parent) = self.socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let listener = std::os::unix::net::UnixListener::bind(&self.socket_path)?;
        listener.set_nonblocking(true)?;

        let mut last_scan = Instant::now();
        let mut last_save = Instant::now();
        let stop_requested = Arc::new(AtomicBool::new(false));

        while running.load(Ordering::SeqCst) && !stop_requested.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let candidates = Arc::clone(&self.candidates);
                    let start_time = self.start_time;
                    let stop_flag = Arc::clone(&stop_requested);
                    let running_flag = Arc::clone(running);
                    std::thread::spawn(move || {
                        let _ = Self::handle_client_stream(
                            stream,
                            &candidates,
                            start_time,
                            &stop_flag,
                            &running_flag,
                        );
                    });
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(e) => {
                    let _ = fs::remove_file(&self.socket_path);
                    return Err(e);
                }
            }

            // Periodic candidate cache refresh
            if last_scan.elapsed() >= CACHE_REFRESH_INTERVAL {
                self.refresh_candidates();
                last_scan = Instant::now();
            }

            // Periodic auto-save
            let now = Instant::now();
            if autosave_due(last_save, now) {
                let _ = self.save_snapshot_now(None);
                last_save = now;
            }
        }

        let _ = fs::remove_file(&self.socket_path);
        Ok(())
    }

    #[cfg(not(unix))]
    fn serve_once(&self, _running: &Arc<AtomicBool>) -> io::Result<()> {
        Err(Error::other(
            "The omega daemon requires a Unix domain socket and is not supported on this platform",
        ))
    }

    #[cfg(unix)]
    fn handle_client_stream(
        stream: std::os::unix::net::UnixStream,
        candidates: &Arc<Mutex<Vec<SessionCandidate>>>,
        start_time: Instant,
        stop_requested: &Arc<AtomicBool>,
        running: &Arc<AtomicBool>,
    ) -> io::Result<()> {
        stream.set_nonblocking(false)?;
        // Bound idle clients so a silent connection cannot stall indefinitely.
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = stream;
        let mut line = String::new();

        loop {
            if stop_requested.load(Ordering::SeqCst) || !running.load(Ordering::SeqCst) {
                break;
            }
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // Client disconnected
                Ok(_) => {
                    let Ok(request) = serde_json::from_str::<DaemonRequest>(&line) else {
                        let response = DaemonResponse::Error("Invalid request format".to_owned());
                        let _ = Self::write_response_stream(&mut writer, &response);
                        continue;
                    };

                    let mut should_stop = false;
                    let response = match request {
                        DaemonRequest::Ping => DaemonResponse::Pong(DaemonStatus {
                            running: true,
                            pid: std::process::id(),
                            uptime_secs: start_time.elapsed().as_secs(),
                            candidate_count: candidates.lock().map(|c| c.len()).unwrap_or(0),
                        }),
                        DaemonRequest::GetCandidates => DaemonResponse::Candidates(
                            candidates.lock().map(|c| c.clone()).unwrap_or_default(),
                        ),
                        DaemonRequest::SaveSnapshot { tag } => {
                            match Self::save_snapshot_direct(tag.as_deref()) {
                                Ok(path) => DaemonResponse::SnapshotSaved(path),
                                Err(e) => DaemonResponse::Error(format!("{e}")),
                            }
                        }
                        DaemonRequest::Stop => {
                            should_stop = true;
                            let _ = Self::save_snapshot_direct(None);
                            DaemonResponse::Stopped
                        }
                    };

                    let _ = Self::write_response_stream(&mut writer, &response);

                    if should_stop {
                        stop_requested.store(true, Ordering::SeqCst);
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
                Err(ref e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                {
                    if stop_requested.load(Ordering::SeqCst) || !running.load(Ordering::SeqCst) {
                        break;
                    }
                    continue;
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    fn write_response_stream(
        writer: &mut std::os::unix::net::UnixStream,
        response: &DaemonResponse,
    ) -> bool {
        match serde_json::to_string(response) {
            Ok(mut out) => {
                out.push('\n');
                writer.write_all(out.as_bytes()).is_ok() && writer.flush().is_ok()
            }
            Err(_) => false,
        }
    }

    fn refresh_candidates(&self) {
        let collector = SessionCollector::new();
        let fresh = collector.collect_from_system(&self.config);
        if let Ok(mut lock) = self.candidates.lock() {
            *lock = fresh;
        }
    }

    fn save_snapshot_now(&self, tag: Option<&str>) -> io::Result<PathBuf> {
        Self::save_snapshot_direct(tag)
    }

    fn save_snapshot_direct(tag: Option<&str>) -> io::Result<PathBuf> {
        let store = SnapshotStore::new(SnapshotStore::default_dir());
        let tmux = RealTmuxClient;
        let manager = SnapshotManager::new();
        let snap = manager.capture(&tmux)?;
        store.save_snapshot(&snap, tag)
    }
}

// ---------------------------------------------------------------------------
// Unix transport: client
// ---------------------------------------------------------------------------

#[cfg(unix)]
pub struct DaemonClient {
    reader: BufReader<std::os::unix::net::UnixStream>,
    writer: std::os::unix::net::UnixStream,
}

#[cfg(unix)]
impl DaemonClient {
    /// Attempts to connect to the daemon socket. Returns None if the daemon is
    /// not reachable (stopped, or a stale socket file).
    pub fn connect(socket_path: Option<&Path>) -> Option<Self> {
        let path = match socket_path {
            Some(p) => p.to_path_buf(),
            None => DaemonServer::default_socket_path(),
        };

        if !path.exists() {
            return None;
        }

        let stream = std::os::unix::net::UnixStream::connect(path).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
        let writer = stream.try_clone().ok()?;
        let reader = BufReader::new(stream);
        Some(Self { reader, writer })
    }

    pub fn ping(&mut self) -> io::Result<DaemonStatus> {
        match self.send_request(DaemonRequest::Ping)? {
            DaemonResponse::Pong(status) => Ok(status),
            other => Err(Error::other(format!("Unexpected response: {other:?}"))),
        }
    }

    pub fn get_candidates(&mut self) -> io::Result<Vec<SessionCandidate>> {
        match self.send_request(DaemonRequest::GetCandidates)? {
            DaemonResponse::Candidates(c) => Ok(c),
            other => Err(Error::other(format!("Unexpected response: {other:?}"))),
        }
    }

    pub fn save_snapshot(&mut self, tag: Option<&str>) -> io::Result<PathBuf> {
        match self.send_request(DaemonRequest::SaveSnapshot {
            tag: tag.map(ToOwned::to_owned),
        })? {
            DaemonResponse::SnapshotSaved(p) => Ok(p),
            DaemonResponse::Error(msg) => Err(Error::other(msg)),
            other => Err(Error::other(format!("Unexpected response: {other:?}"))),
        }
    }

    pub fn stop(&mut self) -> io::Result<()> {
        match self.send_request(DaemonRequest::Stop)? {
            DaemonResponse::Stopped => Ok(()),
            other => Err(Error::other(format!("Unexpected response: {other:?}"))),
        }
    }

    fn send_request(&mut self, req: DaemonRequest) -> io::Result<DaemonResponse> {
        let mut msg =
            serde_json::to_string(&req).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        msg.push('\n');
        self.writer.write_all(msg.as_bytes())?;
        self.writer.flush()?;

        let mut line = String::new();
        self.reader.read_line(&mut line)?;

        serde_json::from_str(&line).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}

/// Tries to retrieve cached candidates from the background daemon socket.
/// Falls back cleanly to direct synchronous scanning if the daemon is stopped
/// or unreachable.
pub fn fetch_candidates_or_fallback(
    socket_path: Option<&Path>,
    config: &Config,
) -> Vec<SessionCandidate> {
    #[cfg(unix)]
    if let Some(mut client) = DaemonClient::connect(socket_path)
        && let Ok(candidates) = client.get_candidates()
    {
        return candidates;
    }

    // Direct synchronous scan fallback
    let collector = SessionCollector::new();
    collector.collect_from_system(config)
}

// ---------------------------------------------------------------------------
// Daemon lifecycle (start / stop / status)
// ---------------------------------------------------------------------------

/// Starts the background daemon, spawning a detached `omega daemon serve`
/// process. Returns the daemon pid. Succeeds immediately if already running.
pub fn daemon_start(socket_path: &Path, serve_args: &[&str]) -> io::Result<u32> {
    #[cfg(unix)]
    if let Ok(status) = ping_socket(socket_path) {
        return Ok(status.pid);
    }

    // Remove a stale socket file so a fresh server can bind.
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cmd = std::process::Command::new(std::env::current_exe()?);
    cmd.args(serve_args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach from the controlling terminal so the daemon survives shell exit.
        // SAFETY: setsid() in the forked child only affects the child process.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| Error::other(format!("Failed to spawn daemon process: {e}")))?;
    let pid = child.id();
    let _ = fs::write(pid_path(socket_path), format!("{pid}"));

    // Wait for the socket to become reachable.
    let deadline = Instant::now() + START_READY_TIMEOUT;
    while Instant::now() < deadline {
        if ping_socket(socket_path).is_ok() {
            return Ok(pid);
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Startup failed: clean up.
    kill_process(pid);
    let _ = fs::remove_file(pid_path(socket_path));
    Err(Error::other(
        "Daemon failed to start: socket did not become ready",
    ))
}

/// Stops the background daemon. The server writes a final snapshot before
/// exiting. Succeeds if the daemon is already stopped.
pub fn daemon_stop(socket_path: &Path) -> io::Result<()> {
    let pid = read_pid(socket_path);

    #[cfg(unix)]
    if let Some(mut client) = DaemonClient::connect(Some(socket_path)) {
        // The server auto-saves a final snapshot on Stop before exiting.
        client.stop()?;
    }

    // Wait for the process to exit, escalating signals if necessary.
    if let Some(pid) = pid {
        wait_for_exit(pid, STOP_EXIT_TIMEOUT);
        if process_alive(pid) {
            kill_process(pid);
            wait_for_exit(pid, Duration::from_secs(1));
        }
    }

    let _ = fs::remove_file(socket_path);
    let _ = fs::remove_file(pid_path(socket_path));
    Ok(())
}

/// Returns the live daemon status, or None when the daemon is not reachable.
pub fn daemon_status(socket_path: &Path) -> Option<DaemonStatus> {
    #[cfg(unix)]
    {
        ping_socket(socket_path).ok()
    }
    #[cfg(not(unix))]
    {
        let _ = socket_path;
        None
    }
}

#[cfg(unix)]
fn ping_socket(socket_path: &Path) -> io::Result<DaemonStatus> {
    DaemonClient::connect(Some(socket_path))
        .ok_or_else(|| Error::other("daemon socket not reachable"))?
        .ping()
}

fn read_pid(socket_path: &Path) -> Option<u32> {
    fs::read_to_string(pid_path(socket_path))
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // SAFETY: kill(pid, 0) performs an existence check without sending a signal.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0
}

#[cfg(not(unix))]
fn process_alive(pid: u32) -> bool {
    let _ = pid;
    false
}

#[cfg(unix)]
fn kill_process(pid: u32) {
    // SAFETY: best-effort termination of the daemon child process.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
}

#[cfg(unix)]
fn wait_for_exit(pid: u32, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Registers SIGTERM/SIGINT/SIGHUP into a shared flag so the serve loop can
/// shut down gracefully (with a final snapshot) on termination signals.
#[cfg(unix)]
fn register_termination_signals() -> io::Result<Arc<AtomicBool>> {
    let flag = Arc::new(AtomicBool::new(false));
    let signals = [
        signal_hook::consts::signal::SIGTERM,
        signal_hook::consts::signal::SIGINT,
        signal_hook::consts::signal::SIGHUP,
    ];
    for sig in signals {
        signal_hook::flag::register(sig, Arc::clone(&flag)).map_err(Error::other)?;
    }
    Ok(flag)
}
