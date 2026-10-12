use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxSessionRecord {
    pub name: String,
    pub path: PathBuf,
    pub attached: bool,
}

pub trait TmuxClient {
    fn is_inside_tmux(&self) -> bool;
    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>>;
    fn has_session(&self, name: &str) -> bool;
    fn create_session(&mut self, name: &str, path: &Path, command: Option<&str>) -> io::Result<()>;
    fn switch_client(&mut self, target: &str) -> io::Result<()>;
    fn attach_session(&mut self, target: &str) -> io::Result<()>;
    fn kill_session(&mut self, target: &str) -> io::Result<()>;
    fn switch_last(&mut self) -> io::Result<()>;
    fn capture_pane(&self, target: &str) -> io::Result<String>;
}

#[derive(Default, Debug, Clone)]
pub struct RealTmuxClient;

impl TmuxClient for RealTmuxClient {
    fn is_inside_tmux(&self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>> {
        let output = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}:#{session_path}:#{session_attached}",
            ])
            .output()?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let records = raw
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.trim().split(':').collect();
                if parts.len() >= 3 {
                    Some(TmuxSessionRecord {
                        name: parts[0].to_string(),
                        path: PathBuf::from(parts[1]),
                        attached: parts[2] == "1",
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(records)
    }

    fn has_session(&self, name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn create_session(&mut self, name: &str, path: &Path, command: Option<&str>) -> io::Result<()> {
        let path_str = path.to_string_lossy();
        let mut cmd = Command::new("tmux");
        cmd.args(["new-session", "-ds", name, "-c", &path_str]);
        if let Some(c) = command {
            cmd.arg(c);
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to create tmux session '{}'",
                name
            )));
        }
        Ok(())
    }

    fn switch_client(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["switch-client", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to switch tmux client to '{}'",
                target
            )));
        }
        Ok(())
    }

    fn attach_session(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to attach to tmux session '{}'",
                target
            )));
        }
        Ok(())
    }

    fn kill_session(&mut self, target: &str) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["kill-session", "-t", target])
            .status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "Failed to kill tmux session '{}'",
                target
            )));
        }
        Ok(())
    }

    fn switch_last(&mut self) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["switch-client", "-l"])
            .status()?;
        if !status.success() {
            return Err(Error::other("Failed to switch to last tmux session"));
        }
        Ok(())
    }

    fn capture_pane(&self, target: &str) -> io::Result<String> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-e", "-p", "-t", target])
            .output()?;
        if !output.status.success() {
            return Err(Error::other(format!(
                "Failed to capture pane for tmux session '{target}'"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatedSessionRecord {
    pub name: String,
    pub path: PathBuf,
    pub command: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct MockTmuxClient {
    pub inside_tmux: bool,
    pub existing_sessions: Vec<String>,
    pub sessions: Vec<TmuxSessionRecord>,
    pub switched_to: Option<String>,
    pub attached_to: Option<String>,
    pub created_sessions: Vec<CreatedSessionRecord>,
    pub killed_sessions: Vec<String>,
    pub switched_last_called: bool,
    pub captured_panes: std::collections::HashMap<String, String>,
}

impl MockTmuxClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TmuxClient for MockTmuxClient {
    fn is_inside_tmux(&self) -> bool {
        self.inside_tmux
    }

    fn list_sessions(&self) -> io::Result<Vec<TmuxSessionRecord>> {
        if !self.sessions.is_empty() {
            Ok(self.sessions.clone())
        } else {
            Ok(self
                .existing_sessions
                .iter()
                .map(|s| TmuxSessionRecord {
                    name: s.clone(),
                    path: PathBuf::from(format!("/path/{}", s)),
                    attached: false,
                })
                .collect())
        }
    }

    fn has_session(&self, name: &str) -> bool {
        self.existing_sessions.iter().any(|s| s == name)
            || self.created_sessions.iter().any(|s| s.name == name)
    }

    fn create_session(&mut self, name: &str, path: &Path, command: Option<&str>) -> io::Result<()> {
        self.created_sessions.push(CreatedSessionRecord {
            name: name.to_string(),
            path: path.to_path_buf(),
            command: command.map(|s| s.to_string()),
        });
        Ok(())
    }

    fn switch_client(&mut self, target: &str) -> io::Result<()> {
        self.switched_to = Some(target.to_string());
        Ok(())
    }

    fn attach_session(&mut self, target: &str) -> io::Result<()> {
        self.attached_to = Some(target.to_string());
        Ok(())
    }

    fn kill_session(&mut self, target: &str) -> io::Result<()> {
        self.killed_sessions.push(target.to_string());
        Ok(())
    }

    fn switch_last(&mut self) -> io::Result<()> {
        self.switched_last_called = true;
        Ok(())
    }

    fn capture_pane(&self, target: &str) -> io::Result<String> {
        if let Some(content) = self.captured_panes.get(target) {
            Ok(content.clone())
        } else {
            Ok(format!("[tmux captured pane for {target}]"))
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct TmuxActionRunner;

impl TmuxActionRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn connect<T: TmuxClient>(
        &self,
        client: &mut T,
        target: &str,
        path: &Path,
        command: Option<&str>,
    ) -> io::Result<()> {
        if !client.has_session(target) {
            client.create_session(target, path, command)?;
        }

        if client.is_inside_tmux() {
            client.switch_client(target)
        } else {
            client.attach_session(target)
        }
    }

    pub fn kill<T: TmuxClient>(&self, client: &mut T, target: &str) -> io::Result<()> {
        client.kill_session(target)
    }

    pub fn last<T: TmuxClient>(&self, client: &mut T) -> io::Result<()> {
        if client.is_inside_tmux() {
            client.switch_last()
        } else {
            let sessions = client.list_sessions()?;
            if let Some(target) = sessions
                .iter()
                .find(|s| !s.attached)
                .or_else(|| sessions.first())
            {
                client.attach_session(&target.name)
            } else {
                Err(Error::other("No tmux sessions found to attach to"))
            }
        }
    }
}
