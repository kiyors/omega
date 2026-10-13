use crate::cli::{Commands, DaemonAction};
use color_eyre::Result;
use omega_config::Config;
use omega_session::daemon::{
    DaemonServer, daemon_start, daemon_status, daemon_stop, fetch_candidates_or_fallback,
};
use omega_session::find_workspace_root;
use omega_session::naming::NamingPipeline;
use omega_session::snapshot::SnapshotStore;
use omega_session::tmux::SnapshotManager;
use omega_session::{SessionCollector, SessionSource, TmuxActionRunner, TmuxClient};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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
            switch,
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
                        let existing: Vec<(&str, &Path)> = candidates
                            .iter()
                            .filter(|c| c.source == SessionSource::Tmux)
                            .map(|c| (c.name.as_str(), c.path.as_path()))
                            .collect();
                        let name = namer.resolve_name(&target_path, config.dir_length, &existing);
                        (name, target_path)
                    } else {
                        // Treat as raw session name in current dir
                        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        (target.clone(), cwd)
                    }
                }
            };

            if switch && tmux.is_inside_tmux() {
                if !tmux.has_session(&session_name) {
                    tmux.create_session(
                        &session_name,
                        &session_name,
                        &session_path,
                        command.as_deref(),
                    )?;
                }
                tmux.switch_client(&session_name)?;
            } else {
                runner.connect(tmux, &session_name, &session_path, command.as_deref())?;
            }
        }

        Commands::List {
            tmux: only_tmux,
            git: only_git,
            zoxide: only_zoxide,
            json,
            format,
        } => {
            let mut candidates = fetch_candidates_or_fallback(None, config);

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
                        .replace("{source}", &format!("{:?}", c.source).to_lowercase())
                        .replace("{icon}", c.icon.as_deref().unwrap_or(""))
                        .replace(
                            "{windows}",
                            &c.windows
                                .map(|w| w.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        )
                        .replace(
                            "{attached}",
                            if c.attached { "attached" } else { "detached" },
                        )
                        .replace(
                            "{score}",
                            &c.score
                                .map(|s| format!("{:.1}", s))
                                .unwrap_or_else(|| "0.0".to_string()),
                        );
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
            let name = resolve_tmux_session_name(&namer, &canonical, config.dir_length, tmux);
            runner.connect(tmux, &name, &canonical, command.as_deref())?;
        }

        Commands::Clone { repo, dir } => {
            let target_path = match dir {
                Some(d) => d,
                None => {
                    let clean = repo.trim_end_matches('/').trim_end_matches(".git");
                    let name = clean.split(['/', ':']).next_back().unwrap_or("repo");
                    PathBuf::from(name)
                }
            };

            if !target_path.exists() {
                let status = std::process::Command::new("git")
                    .args(["clone", &repo])
                    .arg(&target_path)
                    .status()?;
                if !status.success() {
                    return Err(color_eyre::eyre::eyre!(
                        "Failed to clone repository '{}'",
                        repo
                    ));
                }
            }

            let canonical = fs::canonicalize(&target_path).unwrap_or_else(|_| target_path.clone());
            let name = resolve_tmux_session_name(&namer, &canonical, config.dir_length, tmux);
            runner.connect(tmux, &name, &canonical, None)?;
        }

        Commands::Save { name } => {
            let manager = SnapshotManager::new();
            let snapshot = manager.capture(tmux)?;
            let store = SnapshotStore::new(SnapshotStore::default_dir());
            let path = store.save_snapshot(&snapshot, name.as_deref())?;
            println!("Saved snapshot to '{}'", path.display());
        }

        Commands::Restore {
            session,
            name,
            overwrite,
        } => {
            let store = SnapshotStore::new(SnapshotStore::default_dir());
            let snapshot = store.load_snapshot(name.as_deref())?;
            let manager = SnapshotManager::new();
            let summary = manager.restore(tmux, &snapshot, session.as_deref(), overwrite)?;

            if !summary.restored.is_empty() {
                println!("Restored sessions: {}", summary.restored.join(", "));
            }
            if !summary.skipped.is_empty() {
                println!(
                    "Skipped active sessions (use --overwrite to force): {}",
                    summary.skipped.join(", ")
                );
            }
        }

        Commands::Daemon { action } => match action {
            DaemonAction::Start => {
                let sock = DaemonServer::default_socket_path();
                match daemon_start(&sock, &["daemon", "serve"]) {
                    Ok(pid) => println!("Started omega daemon (PID {pid})"),
                    Err(e) => eprintln!("Failed to start daemon: {e}"),
                }
            }
            DaemonAction::Stop => {
                let sock = DaemonServer::default_socket_path();
                if let Err(e) = daemon_stop(&sock) {
                    eprintln!("Failed to stop daemon: {e}");
                } else {
                    println!("Omega daemon stopped");
                }
            }
            DaemonAction::Status => {
                let sock = DaemonServer::default_socket_path();
                match daemon_status(&sock) {
                    Some(status) => {
                        println!(
                            "Omega daemon is running (PID {}, uptime {}s, {} cached sessions)",
                            status.pid, status.uptime_secs, status.candidate_count
                        );
                    }
                    None => {
                        println!("Omega daemon is not running");
                    }
                }
            }
            DaemonAction::Serve => {
                let sock = DaemonServer::default_socket_path();
                let server = DaemonServer::new(sock, config.clone());
                let running = Arc::new(AtomicBool::new(true));
                server.listen_until(running)?;
            }
        },
    }

    Ok(())
}

fn resolve_tmux_session_name<T: TmuxClient>(
    namer: &NamingPipeline,
    target_path: &Path,
    dir_length: usize,
    tmux: &T,
) -> String {
    let active_sessions = tmux.list_sessions().unwrap_or_default();
    let existing: Vec<(&str, &Path)> = active_sessions
        .iter()
        .map(|s| (s.name.as_str(), s.path.as_path()))
        .collect();
    namer.resolve_name(target_path, dir_length, &existing)
}
