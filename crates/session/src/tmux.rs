use crate::snapshot::{
    PaneSnapshot, SessionSnapshotRecord, Snapshot, TmuxSessionState, WindowSnapshot,
    parse_session_state,
};
use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxSessionRecord {
    pub name: String,
    pub path: PathBuf,
    pub attached: bool,
}

/// Identifiers of a freshly created tmux window and its first pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedIds {
    /// tmux window id (e.g. `@3`).
    pub window_id: String,
    /// tmux pane id of the window's first pane (e.g. `%7`).
    pub pane_id: String,
}

pub trait TmuxClient {
    fn is_inside_tmux(&self) -> bool;
    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>>;
    fn has_session(&self, name: &str) -> bool;

    /// Creates a detached session with one named window; returns the created
    /// window and first pane identifiers.
    fn create_session(
        &mut self,
        session: &str,
        window_name: &str,
        path: &Path,
        command: Option<&str>,
    ) -> io::Result<CreatedIds>;

    /// Creates an additional window in an existing session.
    fn create_window(&mut self, session: &str, name: &str, path: &Path) -> io::Result<CreatedIds>;

    /// Splits the window containing `window_target`, starting the new pane at
    /// `path`. Returns the new pane id.
    fn split_pane(&mut self, window_target: &str, path: &Path) -> io::Result<String>;

    /// Applies a captured native layout string to a window.
    fn apply_layout(&mut self, window_target: &str, layout: &str) -> io::Result<()>;

    /// Sends a command line to a pane followed by Enter.
    fn send_keys(&mut self, pane_target: &str, command: &str) -> io::Result<()>;

    fn switch_client(&mut self, target: &str) -> io::Result<()>;
    fn attach_session(&mut self, target: &str) -> io::Result<()>;
    fn kill_session(&mut self, target: &str) -> io::Result<()>;
    fn switch_last(&mut self) -> io::Result<()>;
    fn capture_pane(&self, target: &str) -> io::Result<String>;

    /// Captures the full session/window/pane state of the tmux server.
    fn capture_state(&self) -> io::Result<Vec<TmuxSessionState>>;
}

/// Parses `-P -F '#{window_id}\t#{pane_id}'` output from tmux.
fn parse_created_ids(stdout: &[u8]) -> io::Result<CreatedIds> {
    let raw = String::from_utf8_lossy(stdout);
    let line = raw.lines().next().unwrap_or_default();
    let mut fields = line.splitn(2, '\t');
    let window_id = fields.next().unwrap_or_default().trim().to_owned();
    let pane_id = fields.next().unwrap_or_default().trim().to_owned();
    if window_id.is_empty() || pane_id.is_empty() {
        return Err(Error::other(format!(
            "Unexpected tmux window/pane identifiers: '{line}'"
        )));
    }
    Ok(CreatedIds { window_id, pane_id })
}

#[derive(Default, Debug, Clone)]
pub struct RealTmuxClient;

impl TmuxClient for RealTmuxClient {
    fn is_inside_tmux(&self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>> {
        let output = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}:#{session_path}:#{session_attached}",
            ])
            .output()?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let records = raw
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.trim().split(':').collect();
                if parts.len() >= 3 {
                    Some(TmuxSessionRecord {
                        name: parts[0].to_string(),
                        path: PathBuf::from(parts[1]),
                        attached: parts[2] == "1",
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(records)
    }

    fn has_session(&self, name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn create_session(
        &mut self,
        session: &str,
        window_name: &str,
        path: &Path,
        command: Option<&str>,
    ) -> io::Result<CreatedIds> {
        let path_str = path.to_string_lossy();
        let mut cmd = Command::new("tmux");
        cmd.args([
            "new-session",
            "-ds",
            session,
            "-n",
            window_name,
            "-c",
            &path_str,
            "-P",
            "-F",
            "#{window_id}\t#{pane_id}",
        ]);
        if let Some(c) = command {
            cmd.arg(c);
        }

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "Failed to create tmux session '{session}'"
            )));
        }
        parse_created_ids(&output.stdout)
    }

    fn create_window(&mut self, session: &str, name: &str, path: &Path) -> io::Result<CreatedIds> {
        let output = Command::new("tmux")
            .args([
                "new-window",
                "-t",
                session,
                "-n",
                name,
                "-c",
                &path.to_string_lossy(),
                "-P",
                "-F",
                "#{window_id}\t#{pane_id}",
            ])
            .output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "Failed to create window '{name}' in tmux session '{session}'"
            )));
        }
        parse_created_ids(&output.stdout)
    }

    fn split_pane(&mut self, window_target: &str, path: &Path) -> io::Result<String> {
        let output = Command::new("tmux")
            .args([
                "split-window",
                "-t",
                window_target,
                "-c",
                &path.to_string_lossy(),
                "-P",
                "-F",
                "#{pane_id}",
            ])
            .output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "Failed to split tmux window '{window_target}'"
            )));
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let pane_id = raw.lines().next().unwrap_or_default().trim().to_owned();
        if pane_id.is_empty() {
            return Err(Error::other(
                "tmux split-window did not report a pane id".to_string(),
            ));
        }
        Ok(pane_id)
    }

    fn apply_layout(&mut self, window_target: &str, layout: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["select-layout", "-t", window_target, layout])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to apply layout to tmux window '{window_target}'"
            )));
        }
        Ok(())
    }

    fn send_keys(&mut self, pane_target: &str, command: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["send-keys", "-t", pane_target, command, "Enter"])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to send command to tmux pane '{pane_target}'"
            )));
        }
        Ok(())
    }

    fn switch_client(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["switch-client", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to switch tmux client to '{target}'"
            )));
        }
        Ok(())
    }

    fn attach_session(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to attach to tmux session '{target}'"
            )));
        }
        Ok(())
    }

    fn kill_session(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["kill-session", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to kill tmux session '{target}'"
            )));
        }
        Ok(())
    }

    fn switch_last(&mut self) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["switch-client", "-l"])
            .status()?;
        if !status.success() {
            return Err(Error::other("Failed to switch to last tmux session"));
        }
        Ok(())
    }

    fn capture_pane(&self, target: &str) -> io::Result<String> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-e", "-p", "-t", target])
            .output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "Failed to capture pane for tmux session '{target}'"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn capture_state(&self) -> io::Result<Vec<TmuxSessionState>> {
        let sessions = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}\t#{session_attached}",
            ])
            .output()?;
        if !sessions.status.success() {
            // No tmux server running: an empty state.
            return Ok(Vec::new());
        }

        let windows = Command::new("tmux")
            .args([
                "list-windows",
                "-a",
                "-F",
                "#{session_name}\t#{window_index}\t#{window_name}\t#{window_layout}\t#{window_active}",
            ])
            .output()?;
        let panes = Command::new("tmux")
            .args([
                "list-panes",
                "-a",
                "-s",
                "-F",
                "#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_current_path}\t#{pane_current_command}\t#{pane_active}",
            ])
            .output()?;

        let sessions_raw = String::from_utf8_lossy(&sessions.stdout);
        let windows_raw = String::from_utf8_lossy(&windows.stdout);
        let panes_raw = String::from_utf8_lossy(&panes.stdout);
        Ok(parse_session_state(&sessions_raw, &windows_raw, &panes_raw))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedSessionRecord {
    pub name: String,
    pub window_name: String,
    pub path: PathBuf,
    pub command: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct MockTmuxClient {
    pub inside_tmux: bool,
    pub existing_sessions: Vec<String>,
    pub sessions: Vec<TmuxSessionRecord>,
    pub switched_to: Option<String>,
    pub attached_to: Option<String>,
    pub created_sessions: Vec<CreatedSessionRecord>,
    pub killed_sessions: Vec<String>,
    pub switched_last_called: bool,
    pub captured_panes: std::collections::HashMap<String, String>,
    /// Fixture returned by `capture_state`.
    pub snapshot_state: Vec<TmuxSessionState>,
    pub created_windows: Vec<(String, String, PathBuf)>,
    pub split_panes: Vec<(String, PathBuf)>,
    pub applied_layouts: Vec<(String, String)>,
    pub sent_keys: Vec<(String, String)>,
}

impl MockTmuxClient {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_window_id(&self) -> String {
        format!(
            "@win{}",
            self.created_sessions.len() + self.created_windows.len() + 1
        )
    }

    fn next_pane_id(&self) -> String {
        format!(
            "%pane{}",
            self.created_sessions.len() + self.created_windows.len() + self.split_panes.len() + 1
        )
    }
}

impl TmuxClient for MockTmuxClient {
    fn is_inside_tmux(&self) -> bool {
        self.inside_tmux
    }

    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>> {
        if !self.sessions.is_empty() {
            Ok(self.sessions.clone())
        } else {
            Ok(self
                .existing_sessions
                .iter()
                .map(|s| TmuxSessionRecord {
                    name: s.clone(),
                    path: PathBuf::from(format!("/path/{}", s)),
                    attached: false,
                })
                .collect())
        }
    }

    fn has_session(&self, name: &str) -> bool {
        self.existing_sessions.iter().any(|s| s == name)
            || self.created_sessions.iter().any(|s| s.name == name)
    }

    fn create_session(
        &mut self,
        session: &str,
        window_name: &str,
        path: &Path,
        command: Option<&str>,
    ) -> io::Result<CreatedIds> {
        self.created_sessions.push(CreatedSessionRecord {
            name: session.to_owned(),
            window_name: window_name.to_owned(),
            path: path.to_path_buf(),
            command: command.map(ToOwned::to_owned),
        });
        Ok(CreatedIds {
            window_id: self.next_window_id(),
            pane_id: self.next_pane_id(),
        })
    }

    fn create_window(&mut self, session: &str, name: &str, path: &Path) -> io::Result<CreatedIds> {
        self.created_windows
            .push((session.to_owned(), name.to_owned(), path.to_path_buf()));
        Ok(CreatedIds {
            window_id: self.next_window_id(),
            pane_id: self.next_pane_id(),
        })
    }

    fn split_pane(&mut self, window_target: &str, path: &Path) -> io::Result<String> {
        self.split_panes
            .push((window_target.to_owned(), path.to_path_buf()));
        Ok(format!("%split{}", self.split_panes.len()))
    }

    fn apply_layout(&mut self, window_target: &str, layout: &str) -> io::Result<()> {
        self.applied_layouts
            .push((window_target.to_owned(), layout.to_owned()));
        Ok(())
    }

    fn send_keys(&mut self, pane_target: &str, command: &str) -> io::Result<()> {
        self.sent_keys
            .push((pane_target.to_owned(), command.to_owned()));
        Ok(())
    }

    fn switch_client(&mut self, target: &str) -> io::Result<()> {
        self.switched_to = Some(target.to_owned());
        Ok(())
    }

    fn attach_session(&mut self, target: &str) -> io::Result<()> {
        self.attached_to = Some(target.to_owned());
        Ok(())
    }

    fn kill_session(&mut self, target: &str) -> io::Result<()> {
        self.killed_sessions.push(target.to_owned());
        Ok(())
    }

    fn switch_last(&mut self) -> io::Result<()> {
        self.switched_last_called = true;
        Ok(())
    }

    fn capture_pane(&self, target: &str) -> io::Result<String> {
        if let Some(content) = self.captured_panes.get(target) {
            Ok(content.clone())
        } else {
            Ok(format!("[tmux captured pane for {target}]"))
        }
    }

    fn capture_state(&self) -> io::Result<Vec<TmuxSessionState>> {
        Ok(self.snapshot_state.clone())
    }
}

/// Pane commands that must not be replayed on restore (spawning a nested
/// shell inside a restored pane would be harmful).
const SHELL_COMMANDS: &[&str] = &["zsh", "bash", "sh", "fish", "dash", "ksh", "nu", "tmux"];

fn is_shell_command(command: &str) -> bool {
    let binary = command.split_whitespace().next().unwrap_or_default();
    SHELL_COMMANDS.contains(&binary)
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct RestoreSummary {
    pub restored: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Default, Debug, Clone)]
pub struct SnapshotManager;

impl SnapshotManager {
    pub fn new() -> Self {
        Self
    }

    /// Captures the live tmux state into a [`Snapshot`].
    pub fn capture<T: TmuxClient>(&self, tmux: &T) -> io::Result<Snapshot> {
        let sessions = tmux
            .capture_state()?
            .into_iter()
            .map(|state| {
                let path = state
                    .windows
                    .first()
                    .and_then(|w| w.panes.first())
                    .map(|p| PathBuf::from(&p.path))
                    .unwrap_or_default();
                let windows = state
                    .windows
                    .into_iter()
                    .map(|w| WindowSnapshot {
                        index: w.index as usize,
                        name: w.name,
                        layout: w.layout,
                        active: w.active,
                        panes: w
                            .panes
                            .into_iter()
                            .map(|p| PaneSnapshot {
                                index: p.index as usize,
                                path: PathBuf::from(p.path),
                                command: p.command,
                                active: p.active,
                            })
                            .collect(),
                    })
                    .collect();
                SessionSnapshotRecord {
                    name: state.name,
                    path,
                    windows,
                }
            })
            .collect();
        Ok(Snapshot::new(sessions))
    }

    /// Restores sessions from a snapshot.
    ///
    /// Non-destructive by default: sessions already running in the live tmux
    /// server are skipped to protect uncommitted in-memory work. `overwrite`
    /// forces recreation. `target_session` restricts the restore to a single
    /// named session (selective restore).
    pub fn restore<T: TmuxClient>(
        &self,
        tmux: &mut T,
        snapshot: &Snapshot,
        target_session: Option<&str>,
        overwrite: bool,
    ) -> io::Result<RestoreSummary> {
        let mut summary = RestoreSummary::default();

        for session in &snapshot.sessions {
            if let Some(target) = target_session
                && session.name != target
            {
                continue;
            }

            let exists = tmux.has_session(&session.name);
            if exists && !overwrite {
                summary.skipped.push(session.name.clone());
                continue;
            }
            if exists && overwrite {
                tmux.kill_session(&session.name)?;
            }

            self.restore_session(tmux, session)?;
            summary.restored.push(session.name.clone());
        }

        Ok(summary)
    }

    fn restore_session<T: TmuxClient>(
        &self,
        tmux: &mut T,
        session: &SessionSnapshotRecord,
    ) -> io::Result<()> {
        // Windows without panes cannot be reconstructed; the first restorable
        // window anchors session creation.
        let restorable: Vec<&WindowSnapshot> = session
            .windows
            .iter()
            .filter(|w| !w.panes.is_empty())
            .collect();

        let Some((first_window, other_windows)) = restorable.split_first() else {
            // Degenerate snapshot: recreate a bare session at the recorded path.
            tmux.create_session(&session.name, &session.name, &session.path, None)?;
            return Ok(());
        };

        let first_pane = match first_window.panes.first() {
            Some(pane) => pane,
            None => return Ok(()),
        };
        let created =
            tmux.create_session(&session.name, &first_window.name, &first_pane.path, None)?;
        self.rebuild_window(tmux, first_window, &created)?;

        for window in other_windows {
            let Some(pane) = window.panes.first() else {
                continue;
            };
            let created = tmux.create_window(&session.name, &window.name, &pane.path)?;
            self.rebuild_window(tmux, window, &created)?;
        }

        Ok(())
    }

    /// Re-splits panes, applies the native layout string, and replays pane
    /// commands for a single restored window.
    fn rebuild_window<T: TmuxClient>(
        &self,
        tmux: &mut T,
        window: &WindowSnapshot,
        created: &CreatedIds,
    ) -> io::Result<()> {
        let mut pane_ids = vec![created.pane_id.clone()];
        for pane in window.panes.iter().skip(1) {
            let pane_id = tmux.split_pane(&created.window_id, &pane.path)?;
            pane_ids.push(pane_id);
        }

        if !window.layout.is_empty() {
            tmux.apply_layout(&created.window_id, &window.layout)?;
        }

        for (pane, pane_id) in window.panes.iter().zip(pane_ids.iter()) {
            if !pane.command.is_empty() && !is_shell_command(&pane.command) {
                tmux.send_keys(pane_id, &pane.command)?;
            }
        }

        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct TmuxActionRunner;

impl TmuxActionRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn connect<T: TmuxClient>(
        &self,
        client: &mut T,
        target: &str,
        path: &Path,
        command: Option<&str>,
    ) -> io::Result<()> {
        if !client.has_session(target) {
            client.create_session(target, target, path, command)?;
        }

        if client.is_inside_tmux() {
            client.switch_client(target)
        } else {
            client.attach_session(target)
        }
    }

    pub fn kill<T: TmuxClient>(&self, client: &mut T, target: &str) -> io::Result<()> {
        client.kill_session(target)
    }

    pub fn last<T: TmuxClient>(&self, client: &mut T) -> io::Result<()> {
        if client.is_inside_tmux() {
            client.switch_last()
        } else {
            let sessions = client.list_sessions()?;
            if let Some(target) = sessions
                .iter()
                .find(|s| !s.attached)
                .or_else(|| sessions.first())
            {
                client.attach_session(&target.name)
            } else {
                Err(Error::other("No tmux sessions found to attach to"))
            }
        }
    }
}
