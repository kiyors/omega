//! Daemon IPC server/client, background cache worker, and snapshot manager for Omega.

pub mod daemon;
pub mod snapshot;

pub use daemon::{
    AUTOSAVE_INTERVAL, CACHE_REFRESH_INTERVAL, DaemonClient, DaemonRequest, DaemonResponse,
    DaemonServer, DaemonStatus, autosave_due, daemon_start, daemon_status, daemon_stop,
    fetch_candidates_or_fallback,
};
pub use snapshot::{
    AUTO_SAVE_PREFIX, AUTO_SAVE_RETENTION, LATEST_POINTER, PaneSnapshot, RestoreSummary,
    SessionSnapshotRecord, Snapshot, SnapshotManager, SnapshotStore, WindowSnapshot,
    break_down_timestamp,
};
