//! Low-level Tmux multiplexer client, parser, and action runner.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Tmux Models
// ---------------------------------------------------------------------------

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

/// Captured state of a single tmux pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxPaneState {
    pub index: u32,
    pub path: String,
    /// Running process command (e.g. `nvim`, `zsh`).
    pub command: String,
    pub active: bool,
}

/// Captured state of a single tmux window, including its native layout string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxWindowState {
    pub index: u32,
    pub name: String,
    /// Native checksummed layout string (`#{window_layout}`).
    pub layout: String,
    pub active: bool,
    pub panes: Vec<TmuxPaneState>,
}

/// Captured state of a single tmux session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxSessionState {
    pub name: String,
    pub attached: bool,
    pub windows: Vec<TmuxWindowState>,
}

// ---------------------------------------------------------------------------
// Tmux Output Parser
// ---------------------------------------------------------------------------

/// Parses raw `tmux list-sessions`, `list-windows -a`, and `list-panes -a -s`
/// output into a session tree.
pub fn parse_session_state(
    sessions_raw: &str,
    windows_raw: &str,
    panes_raw: &str,
) -> Vec<TmuxSessionState> {
    let mut sessions: Vec<TmuxSessionState> = Vec::new();
    let mut session_slots: HashMap<&str, usize> = HashMap::new();

    for line in sessions_raw.lines() {
        let mut fields = line.splitn(2, '\t');
        let (Some(name), Some(attached_flag)) = (fields.next(), fields.next()) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let attached = attached_flag == "1";
        session_slots.insert(name, sessions.len());
        sessions.push(TmuxSessionState {
            name: name.to_string(),
            attached,
            windows: Vec::new(),
        });
    }

    // Maps (session name, window index) -> (session slot, window slot).
    let mut window_slots: HashMap<(&str, u32), (usize, usize)> = HashMap::new();

    for line in windows_raw.lines() {
        let mut fields = line.splitn(5, '\t');
        let (
            Some(session_name),
            Some(index_str),
            Some(name_str),
            Some(layout_str),
            Some(active_str),
        ) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        )
        else {
            continue;
        };
        let Ok(index) = index_str.parse::<u32>() else {
            continue;
        };
        let Some(&session_slot) = session_slots.get(session_name) else {
            continue;
        };
        let name = name_str.to_string();
        let layout = layout_str.to_string();
        let active = active_str == "1";

        let window_slot = sessions[session_slot].windows.len();
        sessions[session_slot].windows.push(TmuxWindowState {
            index,
            name,
            layout,
            active,
            panes: Vec::new(),
        });
        window_slots.insert((session_name, index), (session_slot, window_slot));
    }

    for line in panes_raw.lines() {
        let mut fields = line.splitn(6, '\t');
        let (
            Some(session_name),
            Some(win_str),
            Some(pane_str),
            Some(path_str),
            Some(cmd_str),
            Some(active_str),
        ) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        )
        else {
            continue;
        };
        let (Ok(window_index), Ok(pane_index)) = (win_str.parse::<u32>(), pane_str.parse::<u32>())
        else {
            continue;
        };
        let Some(&(session_slot, window_slot)) = window_slots.get(&(session_name, window_index))
        else {
            continue;
        };
        let path = path_str.to_string();
        let command = cmd_str.to_string();
        let active = active_str == "1";
        sessions[session_slot].windows[window_slot]
            .panes
            .push(TmuxPaneState {
                index: pane_index,
                path,
                command,
                active,
            });
    }

    // tmux reports windows and panes in index order already; sort defensively.
    for session in &mut sessions {
        session.windows.sort_by_key(|w| w.index);
        for window in &mut session.windows {
            window.panes.sort_by_key(|p| p.index);
        }
    }

    sessions
}

// ---------------------------------------------------------------------------
// Tmux Client Trait
// ---------------------------------------------------------------------------

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

    /// Selects active window.
    fn select_window(&mut self, window_target: &str) -> io::Result<()>;

    /// Selects active pane.
    fn select_pane(&mut self, pane_target: &str) -> io::Result<()>;

    fn switch_client(&mut self, target: &str) -> io::Result<()>;
    fn attach_session(&mut self, target: &str) -> io::Result<()>;
    fn kill_session(&mut self, target: &str) -> io::Result<()>;
    /// Renames an active tmux session. If `target` is None, renames the current session.
    fn rename_session(&mut self, target: Option<&str>, new_name: &str) -> io::Result<()>;
    fn switch_last(&mut self) -> io::Result<()>;
    fn capture_pane(&self, target: &str) -> io::Result<String>;

    /// Lists all windows across sessions: #{session_name}\t#{window_name}\t#{pane_current_path}\t#{window_active}
    fn list_windows_raw(&self) -> io::Result<String>;

    /// Displays a popup in tmux: `tmux display-popup -E -w <width> -h <height> <command>`
    fn display_popup(&mut self, width: &str, height: &str, command: &str) -> io::Result<()>;

    /// Captures the full session/window/pane state of the tmux server.
    fn capture_state(&self) -> io::Result<Vec<TmuxSessionState>>;
}

fn parse_created_ids(output: &std::process::Output) -> io::Result<CreatedIds> {
    if !output.status.success() {
        return Err(Error::other(format!(
            "tmux command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let line = raw.lines().next().unwrap_or_default().trim();
    let mut fields = line.split('\t');
    let window_id = fields.next().unwrap_or_default().trim().to_owned();
    let pane_id = fields.next().unwrap_or_default().trim().to_owned();
    if window_id.is_empty() || pane_id.is_empty() {
        return Err(Error::other(format!(
            "Unexpected tmux window/pane identifiers: '{line}'"
        )));
    }
    Ok(CreatedIds { window_id, pane_id })
}

// ---------------------------------------------------------------------------
// Real Tmux Client
// ---------------------------------------------------------------------------

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
                let mut parts = line.trim().split(':');
                let name = parts.next()?;
                let path = parts.next()?;
                let attached = parts.next()? == "1";
                Some(TmuxSessionRecord {
                    name: name.to_string(),
                    path: PathBuf::from(path),
                    attached,
                })
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
            "-d",
            "-s",
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
        parse_created_ids(&output)
    }

    fn create_window(&mut self, session: &str, name: &str, path: &Path) -> io::Result<CreatedIds> {
        let path_str = path.to_string_lossy();
        let output = Command::new("tmux")
            .args([
                "new-window",
                "-d",
                "-t",
                session,
                "-n",
                name,
                "-c",
                &path_str,
                "-P",
                "-F",
                "#{window_id}\t#{pane_id}",
            ])
            .output()?;
        parse_created_ids(&output)
    }

    fn split_pane(&mut self, window_target: &str, path: &Path) -> io::Result<String> {
        let path_str = path.to_string_lossy();
        let output = Command::new("tmux")
            .args([
                "split-window",
                "-d",
                "-t",
                window_target,
                "-c",
                &path_str,
                "-P",
                "-F",
                "#{pane_id}",
            ])
            .output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "tmux split-window failed: {}",
                String::from_utf8_lossy(&output.stderr)
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
            // Resilient fallback: if the terminal is smaller than layout proportions, fall back to tiled.
            let _ = Command::new("tmux")
                .args(["select-layout", "-t", window_target, "tiled"])
                .status();
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

    fn select_window(&mut self, window_target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["select-window", "-t", window_target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to select tmux window '{window_target}'"
            )));
        }
        Ok(())
    }

    fn select_pane(&mut self, pane_target: &str) -> io::Result<()> {
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", pane_target])
            .status();
        Ok(())
    }

    fn switch_client(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["switch-client", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to switch to tmux session '{target}'"
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

    fn rename_session(&mut self, target: Option<&str>, new_name: &str) -> io::Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.arg("rename-session");
        if let Some(t) = target {
            cmd.args(["-t", t]);
        }
        cmd.arg(new_name);
        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to rename session to '{new_name}'"
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

    fn list_windows_raw(&self) -> io::Result<String> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-a",
                "-F",
                "#{session_name}\t#{window_name}\t#{pane_current_path}\t#{window_active}",
            ])
            .output()?;
        if !output.status.success() {
            return Ok(String::new());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn display_popup(&mut self, width: &str, height: &str, command: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["display-popup", "-E", "-w", width, "-h", height, command])
            .status()?;
        if !status.success() {
            return Err(Error::other("Failed to display tmux popup"));
        }
        Ok(())
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

// ---------------------------------------------------------------------------
// Mock Tmux Client
// ---------------------------------------------------------------------------

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
    pub killed_sessions: Vec<String>,
    pub switched_last_called: bool,
    pub captured_panes: HashMap<String, String>,
    pub created_sessions: Vec<CreatedSessionRecord>,
    pub created_windows: Vec<(String, String, PathBuf)>,
    pub split_panes: Vec<(String, PathBuf)>,
    pub applied_layouts: Vec<(String, String)>,
    pub sent_keys: Vec<(String, String)>,
    pub selected_windows: Vec<String>,
    pub selected_panes: Vec<String>,
    pub renamed_sessions: Vec<(Option<String>, String)>,
    pub raw_windows: String,
    pub popups: Vec<(String, String, String)>,
    pub snapshot_state: Vec<TmuxSessionState>,
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
            return Ok(self.sessions.clone());
        }
        let list: Vec<TmuxSessionRecord> = self
            .existing_sessions
            .iter()
            .map(|name| TmuxSessionRecord {
                name: name.clone(),
                path: PathBuf::from(format!("/mock/path/{name}")),
                attached: false,
            })
            .collect();
        Ok(list)
    }

    fn has_session(&self, name: &str) -> bool {
        self.existing_sessions.iter().any(|s| s == name)
            || self.sessions.iter().any(|s| s.name == name)
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

    fn select_window(&mut self, window_target: &str) -> io::Result<()> {
        self.selected_windows.push(window_target.to_owned());
        Ok(())
    }

    fn select_pane(&mut self, pane_target: &str) -> io::Result<()> {
        self.selected_panes.push(pane_target.to_owned());
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

    fn rename_session(&mut self, target: Option<&str>, new_name: &str) -> io::Result<()> {
        self.renamed_sessions
            .push((target.map(ToOwned::to_owned), new_name.to_owned()));
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

    fn list_windows_raw(&self) -> io::Result<String> {
        Ok(self.raw_windows.clone())
    }

    fn display_popup(&mut self, width: &str, height: &str, command: &str) -> io::Result<()> {
        self.popups
            .push((width.to_owned(), height.to_owned(), command.to_owned()));
        Ok(())
    }

    fn capture_state(&self) -> io::Result<Vec<TmuxSessionState>> {
        Ok(self.snapshot_state.clone())
    }
}

// ---------------------------------------------------------------------------
// Tmux Action Runner
// ---------------------------------------------------------------------------

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

    pub fn switch_window<T: TmuxClient>(
        &self,
        client: &mut T,
        window_target: &str,
    ) -> io::Result<()> {
        client.select_window(window_target)?;
        if client.is_inside_tmux() {
            client.switch_client(window_target)
        } else {
            let session_target = window_target
                .split_once(':')
                .map_or(window_target, |(s, _)| s);
            client.attach_session(session_target)
        }
    }

    pub fn scratchpad<T: TmuxClient>(&self, client: &mut T, name: &str) -> io::Result<()> {
        if client.is_inside_tmux() {
            let cmd = format!("tmux new-session -A -s {name}");
            client.display_popup("80%", "80%", &cmd)
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            self.connect(client, name, &cwd, None)
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
