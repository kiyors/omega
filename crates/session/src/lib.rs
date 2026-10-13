pub mod collector;
pub mod daemon;
pub mod discovery;
pub mod models;
pub mod naming;
pub mod preview;
pub mod snapshot;
pub mod tmux;

pub use collector::SessionCollector;
pub use daemon::{
    AUTOSAVE_INTERVAL, CACHE_REFRESH_INTERVAL, DaemonClient, DaemonRequest, DaemonResponse,
    DaemonServer, DaemonStatus, autosave_due, daemon_start, daemon_status, daemon_stop,
    fetch_candidates_or_fallback,
};
pub use discovery::{
    TmuxWindowInfo, find_workspace_root, parse_tmux_windows, scan_git_repositories,
};
pub use models::{SessionCandidate, SessionSource};
pub use naming::NamingPipeline;
pub use preview::PreviewGenerator;
pub use snapshot::{
    AUTO_SAVE_PREFIX, AUTO_SAVE_RETENTION, LATEST_POINTER, PaneSnapshot, SessionSnapshotRecord,
    Snapshot, SnapshotStore, SnapshotSummary, TmuxPaneState, TmuxSessionState, TmuxWindowState,
    WindowSnapshot, break_down_timestamp, parse_session_state,
};
pub use tmux::{
    CreatedIds, MockTmuxClient, RealTmuxClient, RestoreSummary, SnapshotManager, TmuxActionRunner,
    TmuxClient,
};
