pub mod collector;
pub mod discovery;
pub mod models;
pub mod naming;
pub mod tmux;

pub use collector::SessionCollector;
pub use models::{SessionCandidate, SessionSource};
pub use naming::NamingPipeline;
pub use tmux::{MockTmuxClient, RealTmuxClient, TmuxActionRunner, TmuxClient};
