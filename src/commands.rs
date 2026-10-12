use crate::cli::Commands;
use color_eyre::Result;
use omega_config::Config;
use omega_session::naming::NamingPipeline;
use omega_session::{SessionCollector, SessionSource, TmuxActionRunner, TmuxClient};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_command<T: TmuxClient>(
    command: Commands,
    tmux: &mut T,
    config: &Config,
) -> Result<()> {
    let runner = TmuxActionRunner::new();
    let collector = SessionCollector::new();
    let namer = NamingPipeline::new();

    match command {
        Commands::Connect {
            target,
            switch: _,
            command,
        } => {
            // Check if target matches an existing candidate
            let candidates = collector.collect_from_system(config);
            let matched = candidates
                .iter()
                .find(|c| c.name == target || c.alias.as_deref() == Some(&target));

            let (session_name, session_path) = match matched {
                Some(cand) => (cand.name.clone(), cand.path.clone()),
                None => {
                    let target_path = PathBuf::from(&target);
                    if target_path.exists() {
                        let name = namer.resolve_name(&target_path, config.dir_length, &[]);
                        (name, target_path)
                    } else {
                        // Treat as raw session name in current dir
                        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        (target.clone(), cwd)
                    }
                }
            };

            runner.connect(tmux, &session_name, &session_path, command.as_deref())?;
        }

        Commands::List {
            tmux: only_tmux,
            git: only_git,
            zoxide: only_zoxide,
            json,
            format,
        } => {
            let mut candidates = collector.collect_from_system(config);

            if only_tmux || only_git || only_zoxide {
                candidates.retain(|c| {
                    (only_tmux && c.source == SessionSource::Tmux)
                        || (only_git && c.source == SessionSource::Git)
                        || (only_zoxide && c.source == SessionSource::Zoxide)
                });
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&candidates)?);
            } else if let Some(fmt) = format {
                for c in &candidates {
                    let formatted = fmt
                        .replace("{name}", &c.name)
                        .replace("{path}", &c.path.to_string_lossy())
                        .replace("{source}", &format!("{:?}", c.source).to_lowercase());
                    println!("{}", formatted);
                }
            } else {
                for c in &candidates {
                    println!("[{:?}]\t{}\t{}", c.source, c.name, c.path.display());
                }
            }
        }

        Commands::Kill { target, force: _ } => {
            runner.kill(tmux, &target)?;
            println!("Terminated session '{}'", target);
        }

        Commands::Last => {
            runner.last(tmux)?;
        }

        Commands::Root => {
            let root = find_workspace_root(None)?;
            println!("{}", root.display());
        }

        Commands::Mkdir { path, command } => {
            fs::create_dir_all(&path)?;
            let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let name = namer.resolve_name(&canonical, config.dir_length, &[]);
            runner.connect(tmux, &name, &canonical, command.as_deref())?;
        }
    }

    Ok(())
}

/// Finds the root directory of the current git repository or defaults to current working directory.
pub fn find_workspace_root(start_dir: Option<&Path>) -> Result<PathBuf> {
    let mut current = match start_dir {
        Some(d) => d.to_path_buf(),
        None => env::current_dir()?,
    };

    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Default to starting directory if no git root found
    match start_dir {
        Some(d) => Ok(d.to_path_buf()),
        None => Ok(env::current_dir()?),
    }
}
