pub mod collector;
pub mod discovery;
pub mod layout;
pub mod models;
pub mod naming;
pub mod preview;

pub use collector::SessionCollector;
pub use discovery::{
    TmuxWindowInfo, find_git_context, find_workspace_root, parse_tmux_window_candidates,
    parse_tmux_windows, scan_git_repositories,
};
pub use models::{SessionCandidate, SessionSource};
pub use naming::NamingPipeline;
pub use preview::PreviewGenerator;

pub use omega_tmux as tmux;
pub use omega_tmux::{
    CreatedIds, MockTmuxClient, RealTmuxClient, TmuxActionRunner, TmuxClient, TmuxPaneState,
    TmuxSessionRecord, TmuxSessionState, TmuxWindowState, parse_session_state,
};
