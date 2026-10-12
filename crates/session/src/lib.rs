pub mod collector;
pub mod discovery;
pub mod models;
pub mod naming;
pub mod preview;
pub mod tmux;

pub use collector::SessionCollector;
pub use discovery::{
    TmuxWindowInfo, find_workspace_root, parse_tmux_windows, scan_git_repositories,
};
pub use models::{SessionCandidate, SessionSource};
pub use naming::NamingPipeline;
pub use preview::PreviewGenerator;
pub use tmux::{MockTmuxClient, RealTmuxClient, TmuxActionRunner, TmuxClient};
