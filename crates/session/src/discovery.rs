use crate::models::{SessionCandidate, SessionSource};
use crate::naming::NamingPipeline;
use omega_config::Config;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Parses `tmux list-sessions` format: `#{session_name}:#{session_path}:#{session_attached}:#{session_windows}`
pub fn parse_tmux_output(raw: &str) -> Vec<SessionCandidate> {
    let mut candidates = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split(':');
        let Some(name) = parts.next() else {
            continue;
        };

        let path = parts.next().map(PathBuf::from).unwrap_or_default();
        let attached = parts.next().is_some_and(|s| s == "1");
        let windows = parts.next().and_then(|s| s.parse::<usize>().ok());

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

/// Window information discovered from tmux
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TmuxWindowInfo {
    pub session_name: String,
    pub window_name: String,
    pub active: bool,
}

/// Parses `tmux list-windows -a` format: `#{session_name}:#{window_name}:#{window_active}`
pub fn parse_tmux_windows(raw: &str) -> Vec<TmuxWindowInfo> {
    let mut windows = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split(':');
        let (Some(session_name), Some(window_name), Some(active_str)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };

        windows.push(TmuxWindowInfo {
            session_name: session_name.to_string(),
            window_name: window_name.to_string(),
            active: active_str == "1",
        });
    }

    windows
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
                    if git_path.is_file() {
                        // Linked worktree checkout
                        let name =
                            if let Some((repo, branch)) = resolve_worktree_info(path, &git_path) {
                                NamingPipeline::new().resolve_worktree_name(&repo, &branch)
                            } else {
                                file_name.to_string()
                            };
                        candidates.push(SessionCandidate::new(name, path, SessionSource::Git));
                    } else if git_path.is_dir() {
                        // Standard Git repository
                        let name = file_name.to_string();
                        candidates.push(SessionCandidate::new(name, path, SessionSource::Git));
                        let linked = discover_worktrees_from_admin(&git_path);
                        candidates.extend(linked);
                    }
                    // Don't recurse into subdirectories of an already detected repo
                    it.skip_current_dir();
                }
            }
        }
    }

    candidates
}

/// Inspects a worktree administrative directory (e.g. `<main_repo>/.git/worktrees/<name>`)
/// and extracts the base repository name and checked out branch.
pub fn inspect_worktree_admin(admin_path: &Path) -> Option<(String, String)> {
    let head_path = admin_path.join("HEAD");
    let head_content = std::fs::read_to_string(head_path).ok()?;
    let branch = if let Some(ref_line) = head_content
        .lines()
        .find(|l| l.starts_with("ref: refs/heads/"))
    {
        ref_line
            .strip_prefix("ref: refs/heads/")?
            .trim()
            .to_string()
    } else {
        head_content.trim().chars().take(7).collect::<String>()
    };

    let canonical_admin =
        std::fs::canonicalize(admin_path).unwrap_or_else(|_| admin_path.to_path_buf());
    let repo_root = canonical_admin.parent()?.parent()?.parent()?;
    let repo_name = repo_root.file_name()?.to_str()?.to_string();

    Some((repo_name, branch))
}

/// Resolves worktree information from a linked worktree root containing a `.git` file.
pub fn resolve_worktree_info(worktree_root: &Path, git_file: &Path) -> Option<(String, String)> {
    let content = std::fs::read_to_string(git_file).ok()?;
    let gitdir_line = content.lines().find(|l| l.starts_with("gitdir:"))?;
    let raw_admin = gitdir_line.strip_prefix("gitdir:")?.trim();
    let admin_path = if Path::new(raw_admin).is_relative() {
        worktree_root.join(raw_admin)
    } else {
        PathBuf::from(raw_admin)
    };
    inspect_worktree_admin(&admin_path)
}

/// Discovers linked worktrees registered in a main repository's `.git/worktrees` folder.
pub fn discover_worktrees_from_admin(git_dir: &Path) -> Vec<SessionCandidate> {
    let mut worktrees = Vec::new();
    let wt_dir = git_dir.join("worktrees");
    if !wt_dir.is_dir() {
        return worktrees;
    }

    if let Ok(entries) = std::fs::read_dir(wt_dir) {
        let namer = NamingPipeline::new();
        for entry in entries.flatten() {
            let admin_path = entry.path();
            if !admin_path.is_dir() {
                continue;
            }
            let gitdir_file = admin_path.join("gitdir");
            if let Ok(target_git) = std::fs::read_to_string(gitdir_file) {
                let raw_path = Path::new(target_git.trim());
                let target_git_path = if raw_path.is_relative() {
                    admin_path.join(raw_path)
                } else {
                    raw_path.to_path_buf()
                };
                if let Some(target_dir) = target_git_path.parent()
                    && target_dir.exists()
                    && let Some((repo, branch)) = inspect_worktree_admin(&admin_path)
                {
                    let name = namer.resolve_worktree_name(&repo, &branch);
                    let canonical = std::fs::canonicalize(target_dir)
                        .unwrap_or_else(|_| target_dir.to_path_buf());
                    worktrees.push(SessionCandidate::new(name, canonical, SessionSource::Git));
                }
            }
        }
    }

    worktrees
}

/// Extracts configured aliases and scans configured static project root folders using WalkDir.
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

        for entry in WalkDir::new(root)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .flatten()
        {
            if entry.file_type().is_dir() {
                let file_name = entry.file_name().to_string_lossy();
                if !file_name.starts_with('.') {
                    static_roots.push(SessionCandidate::new(
                        file_name.to_string(),
                        entry.path(),
                        SessionSource::Config,
                    ));
                }
            }
        }
    }

    (aliases, static_roots)
}

/// Finds the root directory of the current git repository or defaults to starting directory.
pub fn find_workspace_root(start_dir: Option<&std::path::Path>) -> std::io::Result<PathBuf> {
    let mut current = match start_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
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

    match start_dir {
        Some(d) => Ok(d.to_path_buf()),
        None => std::env::current_dir(),
    }
}

/// Discovers Git repository information (repo name and active branch) for a given path.
/// Walks up the directory tree until a `.git` directory or linked worktree file is found.
pub fn find_git_context(path: &Path) -> Option<omega_config::GitContext> {
    let mut current = path;
    loop {
        let git_entry = current.join(".git");
        if git_entry.is_dir() {
            let repo = current.file_name()?.to_str()?.to_owned();
            let head_path = git_entry.join("HEAD");
            let head_content = std::fs::read_to_string(head_path).ok()?;
            let branch = if let Some(ref_line) = head_content
                .lines()
                .find(|l| l.starts_with("ref: refs/heads/"))
            {
                ref_line.strip_prefix("ref: refs/heads/")?.trim().to_owned()
            } else {
                head_content.trim().chars().take(7).collect::<String>()
            };
            return Some(omega_config::GitContext { repo, branch });
        } else if git_entry.is_file() {
            let (repo, branch) = resolve_worktree_info(current, &git_entry)?;
            return Some(omega_config::GitContext { repo, branch });
        }
        current = current.parent()?;
    }
}
