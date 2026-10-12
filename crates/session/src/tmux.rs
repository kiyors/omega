use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait TmuxClient {
    fn is_inside_tmux(&self) -> bool;
    fn has_session(&self, name: &str) -> bool;
    fn create_session(&mut self, name: &str, path: &Path, command: Option<&str>) -> io::Result<()>;
    fn switch_client(&mut self, target: &str) -> io::Result<()>;
    fn attach_session(&mut self, target: &str) -> io::Result<()>;
    fn kill_session(&mut self, target: &str) -> io::Result<()>;
    fn switch_last(&mut self) -> io::Result<()>;
}

#[derive(Default, Debug, Clone)]
pub struct RealTmuxClient;

impl TmuxClient for RealTmuxClient {
    fn is_inside_tmux(&self) -> bool {
        std::env::var("TMUX").is_ok()
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
    pub switched_to: Option<String>,
    pub attached_to: Option<String>,
    pub created_sessions: Vec<CreatedSessionRecord>,
    pub killed_sessions: Vec<String>,
    pub switched_last_called: bool,
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
        client.switch_last()
    }
}
