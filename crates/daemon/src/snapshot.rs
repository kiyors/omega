//! Session snapshot persistence: schema, snapshot store, and snapshot manager.

use omega_tmux::{CreatedIds, TmuxClient};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Snapshot schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub index: usize,
    pub path: PathBuf,
    /// Running process command captured from `#{pane_current_command}`.
    pub command: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub index: usize,
    pub name: String,
    /// Tmux native checksummed layout string (e.g. "bb62,159x48,0,0,1").
    pub layout: String,
    pub active: bool,
    pub panes: Vec<PaneSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshotRecord {
    pub name: String,
    pub path: PathBuf,
    pub windows: Vec<WindowSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub created_at: String,
    pub sessions: Vec<SessionSnapshotRecord>,
}

impl Snapshot {
    pub fn new(sessions: Vec<SessionSnapshotRecord>) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            created_at: current_timestamp_iso(),
            sessions,
        }
    }
}

/// Current snapshot schema version.
pub const SNAPSHOT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Snapshot store
// ---------------------------------------------------------------------------

/// Number of rolling auto-saves retained in the snapshot directory.
pub const AUTO_SAVE_RETENTION: usize = 10;

/// File name of the atomic latest-snapshot pointer.
pub const LATEST_POINTER: &str = "latest.json";

/// Prefix shared by all auto-save snapshot files.
pub const AUTO_SAVE_PREFIX: &str = "snapshot-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSummary {
    pub name: String,
    pub path: PathBuf,
    pub is_auto: bool,
}

pub struct SnapshotStore {
    base_dir: PathBuf,
}

impl SnapshotStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Resolves the snapshot directory (`$XDG_DATA_HOME/omega/snapshots`,
    /// falling back to `~/.local/share/omega/snapshots`).
    pub fn default_dir() -> PathBuf {
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(data_home).join("omega/snapshots")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/omega/snapshots")
        } else {
            PathBuf::from(".omega/snapshots")
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Persists a snapshot. Tagged saves are stored as `<tag>.json` and are
    /// exempt from rotation; untagged saves are auto-saves named
    /// `snapshot-YYYY-MM-DD-HHMMSS.json` and participate in the rolling window.
    pub fn save_snapshot(&self, snapshot: &Snapshot, tag: Option<&str>) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.base_dir)?;

        let content = serde_json::to_string_pretty(snapshot)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        if let Some(name) = tag {
            let target_path = self.base_dir.join(tagged_filename(name));
            let tmp_path = target_path.with_extension("tmp");
            fs::write(&tmp_path, content.as_bytes())?;
            fs::rename(&tmp_path, &target_path)?;
            Ok(target_path)
        } else {
            let base_name = format!("{AUTO_SAVE_PREFIX}{}", current_timestamp_slug());
            let mut target_path = self.base_dir.join(format!("{base_name}.json"));
            let mut counter = 1usize;
            while target_path.exists() {
                target_path = self.base_dir.join(format!("{base_name}-{counter}.json"));
                counter = counter.saturating_add(1);
            }

            let tmp_path = target_path.with_extension("tmp");
            fs::write(&tmp_path, content.as_bytes())?;
            fs::rename(&tmp_path, &target_path)?;

            self.update_latest_pointer(&target_path)?;
            self.rotate_auto_saves(AUTO_SAVE_RETENTION)?;

            Ok(target_path)
        }
    }

    /// Loads a snapshot by tag, or the latest auto-save when no tag is given.
    pub fn load_snapshot(&self, tag: Option<&str>) -> io::Result<Snapshot> {
        let file_path = match tag {
            Some(name) => self.base_dir.join(tagged_filename(name)),
            None => self.base_dir.join(LATEST_POINTER),
        };

        if !file_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Snapshot not found at '{}'", file_path.display()),
            ));
        }

        let content = fs::read_to_string(&file_path)?;
        serde_json::from_str(&content).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }

    /// Keeps only the newest `keep` auto-saves; named snapshots are never rotated.
    pub fn rotate_auto_saves(&self, keep: usize) -> io::Result<()> {
        if !self.base_dir.exists() {
            return Ok(());
        }

        let mut auto_saves = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(AUTO_SAVE_PREFIX) && name.ends_with(".json") {
                auto_saves.push(entry.path());
            }
        }

        // Timestamp-prefixed names sort chronologically.
        auto_saves.sort();

        if auto_saves.len() > keep {
            let excess = auto_saves.len().saturating_sub(keep);
            for old_path in auto_saves.iter().take(excess) {
                let _ = fs::remove_file(old_path);
            }
        }

        Ok(())
    }

    /// Lists all retained snapshots, sorted by file name.
    pub fn list_snapshots(&self) -> io::Result<Vec<SnapshotSummary>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == LATEST_POINTER {
                    continue;
                }
                let is_auto = name.starts_with(AUTO_SAVE_PREFIX);
                list.push(SnapshotSummary {
                    name,
                    path,
                    is_auto,
                });
            }
        }

        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    /// Atomically points `latest.json` at the newest auto-save.
    fn update_latest_pointer(&self, target_path: &Path) -> io::Result<()> {
        let latest_path = self.base_dir.join(LATEST_POINTER);

        #[cfg(unix)]
        {
            let tmp_link = self.base_dir.join(format!("{LATEST_POINTER}.tmp"));
            let _ = fs::remove_file(&tmp_link);
            let filename = target_path
                .file_name()
                .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid file name"))?;
            std::os::unix::fs::symlink(filename, &tmp_link)?;
            fs::rename(&tmp_link, &latest_path)?;
            Ok(())
        }

        #[cfg(not(unix))]
        {
            let tmp_copy = self.base_dir.join(format!("{LATEST_POINTER}.tmp"));
            fs::copy(target_path, &tmp_copy)?;
            fs::rename(&tmp_copy, &latest_path)?;
            Ok(())
        }
    }
}

fn tagged_filename(name: &str) -> String {
    let clean = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name);
    if clean.ends_with(".json") {
        clean.to_owned()
    } else {
        format!("{clean}.json")
    }
}

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------

fn current_timestamp_iso() -> String {
    let (y, m, d, hh, mm, ss) = break_down_timestamp(unix_now_secs());
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn current_timestamp_slug() -> String {
    let (y, m, d, hh, mm, ss) = break_down_timestamp(unix_now_secs());
    format!("{y:04}-{m:02}-{d:02}-{hh:02}{mm:02}{ss:02}")
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Converts unix epoch seconds into a UTC calendar date-time tuple:
/// `(year, month, day, hour, minute, second)`.
pub fn break_down_timestamp(total_secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let total_hours = total_mins / 60;
    let hours = total_hours % 24;
    let mut days = total_hours / 24;

    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days >= days_in_year {
            days = days.saturating_sub(days_in_year);
            year = year.saturating_add(1);
        } else {
            break;
        }
    }

    let leap = is_leap_year(year);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1u64;
    for &dim in &days_in_months {
        if days >= dim {
            days = days.saturating_sub(dim);
            month = month.saturating_add(1);
        } else {
            break;
        }
    }
    let day = days.saturating_add(1);

    (year, month, day, hours, mins, secs)
}

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

// ---------------------------------------------------------------------------
// Snapshot Manager & Restore Engine
// ---------------------------------------------------------------------------

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
        let mut target_matched = false;

        for session in &snapshot.sessions {
            if let Some(target) = target_session
                && session.name != target
            {
                continue;
            }
            target_matched = true;

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

        if let Some(target) = target_session
            && !target_matched
        {
            return Err(io::Error::other(format!(
                "Snapshot contains no session named '{target}'"
            )));
        }

        Ok(summary)
    }

    fn restore_session<T: TmuxClient>(
        &self,
        tmux: &mut T,
        session: &SessionSnapshotRecord,
    ) -> io::Result<()> {
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

        // Focus restoration: select active window and active pane
        let active_window = restorable
            .iter()
            .find(|w| w.active)
            .or_else(|| restorable.first());
        if let Some(w) = active_window {
            let target = format!("{}:{}", session.name, w.index);
            let _ = tmux.select_window(&target);

            if let Some(p) = w.panes.iter().find(|p| p.active) {
                let pane_target = format!("{}:{}.{}", session.name, w.index, p.index);
                let _ = tmux.select_pane(&pane_target);
            }
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
