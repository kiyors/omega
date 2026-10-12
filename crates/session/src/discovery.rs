use crate::models::{SessionCandidate, SessionSource};
use omega_config::Config;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Parses `tmux list-sessions` format: `#{session_name}:#{session_path}:#{session_attached}:#{session_windows}`
pub fn parse_tmux_output(raw: &str) -> Vec<SessionCandidate> {
    let mut candidates = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0];
        let path = if parts.len() > 1 {
            PathBuf::from(parts[1])
        } else {
            PathBuf::new()
        };

        let attached = if parts.len() > 2 {
            parts[2] == "1"
        } else {
            false
        };

        let windows = if parts.len() > 3 {
            parts[3].parse::<usize>().ok()
        } else {
            None
        };

        let mut candidate = SessionCandidate::new(name, path, SessionSource::Tmux);
        candidate.attached = attached;
        candidate.windows = windows;
        candidates.push(candidate);
    }

    candidates
}

/// Parses `zoxide query --list --score` output lines.
/// Each line is either: `<score>  <path>` or `<path>`
pub fn parse_zoxide_output(raw: &str) -> Vec<SessionCandidate> {
    let mut candidates = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (score, path_str) = match trimmed.split_once(char::is_whitespace) {
            Some((first, rest)) => match first.parse::<f64>() {
                Ok(sc) => (Some(sc), rest.trim_start()),
                Err(_) => (Some(0.0), trimmed),
            },
            None => (Some(0.0), trimmed),
        };

        let path = PathBuf::from(path_str);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path_str)
            .to_string();

        let mut candidate = SessionCandidate::new(name, path, SessionSource::Zoxide);
        candidate.score = score;
        candidates.push(candidate);
    }

    candidates
}

/// Discovers Git repository roots and linked worktrees by detecting `.git` entries.
pub fn scan_git_repositories(roots: &[PathBuf], max_depth: usize) -> Vec<SessionCandidate> {
    let mut candidates = Vec::new();

    for root in roots {
        if !root.exists() {
            continue;
        }

        let mut it = WalkDir::new(root).max_depth(max_depth).into_iter();
        while let Some(entry_res) = it.next() {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip top-level root directory itself when evaluating repo status
            if entry.depth() == 0 {
                continue;
            }

            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy();

            // Skip hidden directories like .git or .cache
            if file_name.starts_with('.') {
                if entry.file_type().is_dir() {
                    it.skip_current_dir();
                }
                continue;
            }

            if entry.file_type().is_dir() {
                let git_path = path.join(".git");
                if git_path.exists() {
                    let name = file_name.to_string();
                    candidates.push(SessionCandidate::new(name, path, SessionSource::Git));
                    // Don't recurse into subdirectories of an already detected repo
                    it.skip_current_dir();
                }
            }
        }
    }

    candidates
}

/// Extracts configured aliases and scans configured static project root folders.
pub fn scan_config_sources(config: &Config) -> (Vec<SessionCandidate>, Vec<SessionCandidate>) {
    let mut aliases = Vec::with_capacity(config.aliases.len());
    for alias in &config.aliases {
        let mut candidate = SessionCandidate::new(&alias.name, &alias.path, SessionSource::Alias);
        candidate.alias = Some(alias.name.clone());
        candidate.icon = alias.icon.clone();
        aliases.push(candidate);
    }

    let mut static_roots = Vec::new();
    for root in &config.roots {
        if !root.exists() || !root.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        static_roots.push(SessionCandidate::new(name, path, SessionSource::Config));
                    }
                }
            }
        }
    }

    (aliases, static_roots)
}
