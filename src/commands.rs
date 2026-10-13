use crate::cli::{Commands, DaemonAction};
use color_eyre::Result;
use omega_config::Config;
use omega_daemon::{
    DaemonServer, SnapshotManager, SnapshotStore, daemon_start, daemon_status, daemon_stop,
    fetch_candidates_or_fallback,
};
use omega_session::SessionSource;
use omega_session::find_workspace_root;
use omega_session::naming::NamingPipeline;
use omega_tmux::{TmuxActionRunner, TmuxClient};
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
    execute_command_with_engine(command, tmux, config, None)
}

pub fn execute_command_with_engine<T: TmuxClient>(
    command: Commands,
    tmux: &mut T,
    config: &Config,
    engine: Option<&omega_lua::LuaEngine>,
) -> Result<()> {
    let runner = TmuxActionRunner::new();
    let namer = NamingPipeline::new();

    match command {
        Commands::Connect {
            target,
            switch: _,
            command,
        } => {
            // Check if target matches an existing candidate or alias
            let mut candidates = fetch_candidates_or_fallback(None, config);
            if let Some(eng) = engine
                && let Ok(custom) = eng.eval_custom_sources()
            {
                for item in custom {
                    candidates.push(omega_session::SessionCandidate::from_custom(
                        item.name,
                        item.path,
                        item.source_name,
                        item.icon,
                    ));
                }
            }

            let matched = candidates
                .iter()
                .find(|c| c.name == target || c.alias.as_deref() == Some(&target));

            let (session_name, session_path) = match matched {
                Some(cand) => (cand.name.clone(), cand.path.clone()),
                None => {
                    if let Some(alias) = config.aliases.iter().find(|a| a.name == target) {
                        let path = PathBuf::from(&alias.path);
                        (alias.name.clone(), path)
                    } else {
                        let target_path = PathBuf::from(&target);
                        if target.starts_with('/')
                            || target.starts_with('~')
                            || target.starts_with('.')
                            || target_path.exists()
                        {
                            let existing: Vec<(&str, &Path)> = candidates
                                .iter()
                                .filter(|c| c.source == SessionSource::Tmux)
                                .map(|c| (c.name.as_str(), c.path.as_path()))
                                .collect();
                            let git_ctx = omega_session::find_git_context(&target_path);
                            let custom_name = engine.and_then(|e| {
                                e.call_session_name_hook(&target_path, git_ctx.as_ref())
                            });
                            let name = namer.resolve_full(
                                &target_path,
                                config.dir_length,
                                &existing,
                                custom_name.as_deref(),
                                &config.name_substitutions,
                            );
                            (name, target_path)
                        } else {
                            // Treat as raw session name in current dir
                            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                            (target.clone(), cwd)
                        }
                    }
                }
            };

            smart_connect(
                tmux,
                &session_name,
                &session_path,
                command.as_deref(),
                config,
                engine,
            )?;
        }

        Commands::List {
            tmux: only_tmux,
            git: only_git,
            zoxide: only_zoxide,
            json,
            format,
        } => {
            let mut candidates = fetch_candidates_or_fallback(None, config);
            if let Some(eng) = engine
                && let Ok(custom) = eng.eval_custom_sources()
            {
                for item in custom {
                    candidates.push(omega_session::SessionCandidate::from_custom(
                        item.name,
                        item.path,
                        item.source_name,
                        item.icon,
                    ));
                }
            }

            if !config.sort_order.is_empty() {
                let collector = omega_session::SessionCollector::new();
                candidates = collector.apply_sort_order(candidates, &config.sort_order);
            }

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

        Commands::Rename { first, second } => {
            let (target, new_name) = match second {
                Some(new) => (Some(first.as_str()), new),
                None => (None, first),
            };
            tmux.rename_session(target, &new_name)?;
            if let Some(t) = target {
                println!("Renamed session '{t}' to '{new_name}'");
            } else {
                println!("Renamed active session to '{new_name}'");
            }
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
            let name = resolve_tmux_session_name(&namer, &canonical, config, engine, tmux);
            smart_connect(tmux, &name, &canonical, command.as_deref(), config, engine)?;
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
            let name = resolve_tmux_session_name(&namer, &canonical, config, engine, tmux);
            smart_connect(tmux, &name, &canonical, None, config, engine)?;
        }

        Commands::Save { name } => {
            let manager = SnapshotManager::new();
            let snapshot = manager.capture(tmux)?;
            if snapshot.sessions.is_empty() {
                println!("No active tmux sessions found to save.");
                return Ok(());
            }
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

            if let Some(target) = session.as_deref()
                && !summary.restored.iter().any(|s| s == target)
                && !summary.skipped.iter().any(|s| s == target)
            {
                eprintln!("Session '{target}' not found in snapshot");
                return Ok(());
            }

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

        Commands::Doctor => {
            let env = crate::doctor::RealDoctorEnv;
            let report = crate::doctor::run_doctor(&env, None);
            crate::doctor::print_doctor_report(&report);
        }

        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = crate::cli::Cli::command();
            clap_complete::generate(shell, &mut cmd, "omega", &mut std::io::stdout());
        }
    }

    Ok(())
}

pub fn smart_connect<T: TmuxClient>(
    tmux: &mut T,
    session_name: &str,
    session_path: &Path,
    command: Option<&str>,
    config: &Config,
    engine: Option<&omega_lua::LuaEngine>,
) -> Result<()> {
    if !tmux.has_session(session_name) {
        let home_var = std::env::var("HOME").ok();
        let session_path_str = session_path.to_string_lossy();
        let matched_alias = config.aliases.iter().find(|a| {
            if a.name == session_name {
                return true;
            }
            if let Some(rest) = a.path.strip_prefix("~/")
                && let Some(home) = home_var.as_deref()
            {
                if let Some(suffix) = session_path_str.strip_prefix(home) {
                    return suffix.strip_prefix('/').unwrap_or(suffix) == rest;
                }
                return false;
            }
            a.path == session_path_str
        });

        let windows_to_provision = if let Some(alias) = matched_alias
            && !alias.windows.is_empty()
        {
            Some(&alias.windows)
        } else if let Some(rule) =
            omega_session::layout::match_wildcard(session_path, &config.wildcards)
            && !rule.windows.is_empty()
        {
            Some(&rule.windows)
        } else {
            None
        };

        if let Some(windows) = windows_to_provision {
            omega_session::layout::provision_session_layout(
                tmux,
                session_name,
                session_path,
                windows,
            )?;
        } else {
            let startup_cmd = command.or_else(|| {
                omega_session::layout::match_wildcard(session_path, &config.wildcards)
                    .and_then(|r| r.startup_command.as_deref())
            });
            let created = tmux.create_session(session_name, session_name, session_path, None)?;
            if let Some(cmd) = startup_cmd {
                tmux.send_keys(&created.pane_id, cmd)?;
            }
        }

        if let Some(eng) = engine {
            let _ = eng.dispatch_hook(
                "session_created",
                &omega_config::SessionEventPayload {
                    name: session_name.to_owned(),
                    path: session_path.to_path_buf(),
                },
            );
        }
    }

    if tmux.is_inside_tmux() {
        tmux.switch_client(session_name)?;
    } else {
        tmux.attach_session(session_name)?;
    }

    if let Some(eng) = engine {
        let _ = eng.dispatch_hook(
            "session_switched",
            &omega_config::SessionEventPayload {
                name: session_name.to_owned(),
                path: session_path.to_path_buf(),
            },
        );
    }

    Ok(())
}

fn resolve_tmux_session_name<T: TmuxClient>(
    namer: &NamingPipeline,
    target_path: &Path,
    config: &Config,
    engine: Option<&omega_lua::LuaEngine>,
    tmux: &T,
) -> String {
    let active_sessions = tmux.list_sessions().unwrap_or_default();
    let existing: Vec<(&str, &Path)> = active_sessions
        .iter()
        .map(|s| (s.name.as_str(), s.path.as_path()))
        .collect();
    let git_ctx = omega_session::find_git_context(target_path);
    let custom_name = engine.and_then(|e| e.call_session_name_hook(target_path, git_ctx.as_ref()));
    namer.resolve_full(
        target_path,
        config.dir_length,
        &existing,
        custom_name.as_deref(),
        &config.name_substitutions,
    )
}
