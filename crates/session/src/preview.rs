use crate::models::{SessionCandidate, SessionSource};
use crate::tmux::TmuxClient;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Default, Debug, Clone)]
pub struct PreviewGenerator;

impl PreviewGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generates context-aware preview text for a session candidate.
    /// - Tmux sessions: Live terminal capture via `tmux capture-pane -e -p -t <session>`
    /// - Git repositories: `git status -s` and `git log -n 5 --oneline`
    /// - Directories: Compact directory tree listing
    pub fn generate<T: TmuxClient>(&self, candidate: &SessionCandidate, tmux: &T) -> String {
        match candidate.source {
            SessionSource::Tmux | SessionSource::Window => {
                let raw = tmux.capture_pane(&candidate.name).unwrap_or_else(|e| {
                    format!("Error capturing tmux pane for '{}': {}", candidate.name, e)
                });
                if raw.trim().is_empty() {
                    let kind = if candidate.source == SessionSource::Window {
                        "Window"
                    } else {
                        "Session"
                    };
                    format!("[{kind} active - empty terminal pane]")
                } else {
                    raw
                }
            }
            SessionSource::Git => self.generate_git_preview(&candidate.path),
            SessionSource::Alias
            | SessionSource::Config
            | SessionSource::Zoxide
            | SessionSource::Custom => {
                if candidate.path.join(".git").exists() {
                    self.generate_git_preview(&candidate.path)
                } else if candidate.path.is_dir() {
                    self.generate_dir_preview(&candidate.path)
                } else {
                    format!("Path: {}\n(Not a directory)", candidate.path.display())
                }
            }
        }
    }

    pub fn generate_git_preview(&self, path: &Path) -> String {
        let mut output = String::new();

        // 1. Git branch and status
        let status_cmd = Command::new("git")
            .args(["status", "-s", "-b"])
            .current_dir(path)
            .output();

        if let Ok(st) = status_cmd
            && st.status.success()
        {
            output.push_str("=== Git Status ===\n");
            output.push_str(&String::from_utf8_lossy(&st.stdout));
            output.push('\n');
        }

        // 2. Recent commit log
        let log_cmd = Command::new("git")
            .args(["log", "-n", "5", "--oneline", "--decorate"])
            .current_dir(path)
            .output();

        if let Ok(lg) = log_cmd {
            if lg.status.success() {
                output.push_str("=== Recent Commits ===\n");
                output.push_str(&String::from_utf8_lossy(&lg.stdout));
                output.push('\n');
            } else if output.contains("=== Git Status ===") {
                output.push_str("=== Recent Commits ===\n(No commits yet)\n");
            }
        }

        if output.trim().is_empty() {
            self.generate_dir_preview(path)
        } else {
            output
        }
    }

    pub fn generate_dir_preview(&self, path: &Path) -> String {
        let mut output = String::new();
        output.push_str(&format!("=== Directory: {} ===\n", path.display()));

        let mut count = 0;
        let mut it = WalkDir::new(path).min_depth(1).max_depth(3).into_iter();

        while let Some(entry_res) = it.next() {
            if count >= 30 {
                output.push_str("... (truncated)\n");
                break;
            }
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') {
                if entry.file_type().is_dir() {
                    it.skip_current_dir();
                }
                continue;
            }

            let depth = entry.depth();
            let indent = "  ".repeat(depth.saturating_sub(1));
            let marker = if entry.file_type().is_dir() {
                "📁 "
            } else {
                "📄 "
            };
            output.push_str(&format!("{}{}{}\n", indent, marker, name));
            count += 1;
        }

        output
    }
}
