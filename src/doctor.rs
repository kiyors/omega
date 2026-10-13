use omega_config::Config;
use omega_lua::LuaEngine;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorItem {
    pub name: String,
    pub status: DoctorStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub items: Vec<DoctorItem>,
    pub all_ok: bool,
}

pub trait DoctorEnv {
    fn command_output(&self, cmd: &str, args: &[&str]) -> Option<String>;
    fn check_socket(&self, socket_path: &Path) -> bool;
}

#[derive(Default, Debug, Clone)]
pub struct RealDoctorEnv;

impl DoctorEnv for RealDoctorEnv {
    fn command_output(&self, cmd: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(cmd).args(args).output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            None
        }
    }

    fn check_socket(&self, socket_path: &Path) -> bool {
        omega_daemon::daemon_status(socket_path).is_some()
    }
}

pub fn run_doctor<E: DoctorEnv>(env: &E, config_dir: Option<&Path>) -> DoctorReport {
    let mut items = Vec::new();
    let mut all_ok = true;

    // 1. tmux check
    match env.command_output("tmux", &["-V"]) {
        Some(ver) => {
            items.push(DoctorItem {
                name: "tmux".to_owned(),
                status: DoctorStatus::Ok,
                message: format!("Installed ({ver})"),
            });
        }
        None => {
            all_ok = false;
            items.push(DoctorItem {
                name: "tmux".to_owned(),
                status: DoctorStatus::Error,
                message: "Not found in PATH (required for session multiplexing)".to_owned(),
            });
        }
    }

    // 2. git check
    match env.command_output("git", &["--version"]) {
        Some(ver) => {
            items.push(DoctorItem {
                name: "git".to_owned(),
                status: DoctorStatus::Ok,
                message: format!("Installed ({ver})"),
            });
        }
        None => {
            all_ok = false;
            items.push(DoctorItem {
                name: "git".to_owned(),
                status: DoctorStatus::Error,
                message: "Not found in PATH (required for repo and worktree discovery)".to_owned(),
            });
        }
    }

    // 3. zoxide check (optional)
    match env.command_output("zoxide", &["--version"]) {
        Some(ver) => {
            items.push(DoctorItem {
                name: "zoxide".to_owned(),
                status: DoctorStatus::Ok,
                message: format!("Installed ({ver})"),
            });
        }
        None => {
            items.push(DoctorItem {
                name: "zoxide".to_owned(),
                status: DoctorStatus::Warning,
                message: "Not found in PATH (optional: frecency discovery disabled)".to_owned(),
            });
        }
    }

    // 4. Daemon socket check
    let socket_path = omega_daemon::DaemonServer::default_socket_path();
    if env.check_socket(&socket_path) {
        items.push(DoctorItem {
            name: "daemon".to_owned(),
            status: DoctorStatus::Ok,
            message: "Running and responsive".to_owned(),
        });
    } else {
        items.push(DoctorItem {
            name: "daemon".to_owned(),
            status: DoctorStatus::Warning,
            message: "Not running (optional: falling back to synchronous discovery)".to_owned(),
        });
    }

    // 5. Configuration check
    let dir_to_check = config_dir
        .map(Path::to_path_buf)
        .or_else(Config::config_dir);

    match dir_to_check {
        Some(dir) => {
            let init_lua = dir.join("init.lua");
            let plugins_dir = dir.join("plugins");
            let config_json = dir.join("config.json");
            if init_lua.exists() || plugins_dir.is_dir() {
                match LuaEngine::from_config_dir(&dir) {
                    Ok(engine) => {
                        let cfg = engine.config();
                        items.push(DoctorItem {
                            name: "config".to_owned(),
                            status: DoctorStatus::Ok,
                            message: format!(
                                "Valid configuration at {} ({} aliases, {} wildcards)",
                                dir.display(),
                                cfg.aliases.len(),
                                cfg.wildcards.len()
                            ),
                        });
                    }
                    Err(e) => {
                        all_ok = false;
                        items.push(DoctorItem {
                            name: "config".to_owned(),
                            status: DoctorStatus::Error,
                            message: format!(
                                "Error loading configuration at {}: {e}",
                                dir.display()
                            ),
                        });
                    }
                }
            } else if config_json.exists() {
                match Config::load_from_path(&config_json) {
                    Ok(cfg) => {
                        items.push(DoctorItem {
                            name: "config".to_owned(),
                            status: DoctorStatus::Ok,
                            message: format!(
                                "Valid config.json at {} ({} aliases)",
                                config_json.display(),
                                cfg.aliases.len()
                            ),
                        });
                    }
                    Err(e) => {
                        all_ok = false;
                        items.push(DoctorItem {
                            name: "config".to_owned(),
                            status: DoctorStatus::Error,
                            message: format!(
                                "Error loading config.json at {}: {e}",
                                config_json.display()
                            ),
                        });
                    }
                }
            } else {
                items.push(DoctorItem {
                    name: "config".to_owned(),
                    status: DoctorStatus::Ok,
                    message: "No init.lua or config.json found, using defaults".to_owned(),
                });
            }
        }
        None => {
            items.push(DoctorItem {
                name: "config".to_owned(),
                status: DoctorStatus::Ok,
                message: "No config directory found, using defaults".to_owned(),
            });
        }
    }

    DoctorReport { items, all_ok }
}

pub fn print_doctor_report(report: &DoctorReport) {
    println!("Omega Health Diagnostics:\n");
    for item in &report.items {
        let tag = match item.status {
            DoctorStatus::Ok => "[\x1b[32mOK\x1b[0m]",
            DoctorStatus::Warning => "[\x1b[33mWARN\x1b[0m]",
            DoctorStatus::Error => "[\x1b[31mFAIL\x1b[0m]",
        };
        println!("{:<8} {:<10} {}", tag, item.name, item.message);
    }
    println!();
    if report.all_ok {
        println!("Result: All critical systems healthy!");
    } else {
        println!("Result: Some critical issues were found.");
    }
}
