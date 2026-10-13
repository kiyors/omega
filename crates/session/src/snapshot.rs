//! Session snapshot persistence: schema, capture parsing, and the snapshot store.
//!
//! A [`Snapshot`] is a serialized point-in-time record of active tmux
//! sessions, containing session names, window hierarchies, exact pane
//! geometric layouts (via native tmux layout strings), and active working
//! directories. The [`SnapshotStore`] persists snapshots as JSON under
//! `$XDG_DATA_HOME/omega/snapshots/`, maintaining a rolling window of the
//! last 10 auto-saves alongside an atomic `latest.json` pointer; explicitly
//! named snapshots are exempt from rotation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Capture tree (raw tmux state)
// ---------------------------------------------------------------------------

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
    /// Tmux native checksummed layout string (e.g. `bb62,80x23,0,0,1`).
    pub layout: String,
    pub active: bool,
    pub panes: Vec<TmuxPaneState>,
}

/// Captured state of a single tmux session with its full window hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxSessionState {
    pub name: String,
    pub attached: bool,
    pub windows: Vec<TmuxWindowState>,
}

/// Parses raw `tmux list-sessions`, `list-windows -a`, and `list-panes -a -s`
/// output into a session tree.
///
/// Expected tab-separated fields:
/// - sessions: `#{session_name}\t#{session_attached}`
/// - windows: `#{session_name}\t#{window_index}\t#{window_name}\t#{window_layout}\t#{window_active}`
/// - panes: `#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_current_path}\t#{pane_current_command}\t#{pane_active}`
///
/// Malformed lines and entries referencing unknown sessions or windows are
/// skipped. Windows are ordered by window index and panes by pane index.
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
        let fields: Vec<&str> = line.splitn(5, '\t').collect();
        if fields.len() < 5 {
            continue;
        }
        let (Some(session_name), Some(index)) = (
            fields.first().copied(),
            fields.get(1).and_then(|v| v.parse::<u32>().ok()),
        ) else {
            continue;
        };
        let Some(&session_slot) = session_slots.get(session_name) else {
            continue;
        };
        let name = fields.get(2).copied().unwrap_or_default().to_string();
        let layout = fields.get(3).copied().unwrap_or_default().to_string();
        let active = fields.get(4).copied() == Some("1");

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
        let fields: Vec<&str> = line.splitn(6, '\t').collect();
        if fields.len() < 6 {
            continue;
        }
        let (Some(session_name), Some(window_index), Some(pane_index)) = (
            fields.first().copied(),
            fields.get(1).and_then(|v| v.parse::<u32>().ok()),
            fields.get(2).and_then(|v| v.parse::<u32>().ok()),
        ) else {
            continue;
        };
        let Some(&(session_slot, window_slot)) = window_slots.get(&(session_name, window_index))
        else {
            continue;
        };
        let path = fields.get(3).copied().unwrap_or_default().to_string();
        let command = fields.get(4).copied().unwrap_or_default().to_string();
        let active = fields.get(5).copied() == Some("1");
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
            fs::write(&target_path, content.as_bytes())?;
            Ok(target_path)
        } else {
            let base_name = format!("{AUTO_SAVE_PREFIX}{}", current_timestamp_slug());
            let mut target_path = self.base_dir.join(format!("{base_name}.json"));
            let mut counter = 1usize;
            while target_path.exists() {
                target_path = self.base_dir.join(format!("{base_name}-{counter}.json"));
                counter = counter.saturating_add(1);
            }

            fs::write(&target_path, content.as_bytes())?;

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
            // Non-unix platforms lack symlinks; an atomic rename of a full copy
            // preserves the "latest is always present" guarantee.
            let tmp_copy = self.base_dir.join(format!("{LATEST_POINTER}.tmp"));
            fs::copy(target_path, &tmp_copy)?;
            fs::rename(&tmp_copy, &latest_path)?;
            Ok(())
        }
    }
}

fn tagged_filename(name: &str) -> String {
    if name.ends_with(".json") {
        name.to_owned()
    } else {
        format!("{name}.json")
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
