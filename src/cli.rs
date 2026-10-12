use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, PartialEq)]
#[command(
    name = "omega",
    version,
    about = "A high-performance terminal session and workspace navigator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Initial query for interactive picker (when no subcommand is given)
    #[arg(trailing_var_arg = true)]
    pub query: Vec<String>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Smart connect to a workspace or session
    Connect {
        /// Target session name, path, or alias
        target: String,

        /// Force switch-client even if attached
        #[arg(short, long)]
        switch: bool,

        /// Startup command to execute if a new session is created
        #[arg(short, long)]
        command: Option<String>,
    },

    /// List available workspaces and sessions
    List {
        /// List only active tmux sessions
        #[arg(long)]
        tmux: bool,

        /// List only Git repositories
        #[arg(long)]
        git: bool,

        /// List only Zoxide frecency directories
        #[arg(long)]
        zoxide: bool,

        /// Output results in JSON format
        #[arg(long)]
        json: bool,

        /// Output format template (e.g. "{name} | {path}")
        #[arg(long)]
        format: Option<String>,
    },

    /// Terminate an active tmux session
    Kill {
        /// Target session name to terminate
        target: String,

        /// Force kill
        #[arg(short, long)]
        force: bool,
    },

    /// Bounce to the previously attached session
    Last,

    /// Print root path of current workspace or Git worktree
    Root,

    /// Create directory and immediately connect to a session in it
    Mkdir {
        /// Path of directory to create
        path: PathBuf,

        /// Startup command
        #[arg(short, long)]
        command: Option<String>,
    },
}
