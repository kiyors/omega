use crate::models::{SessionCandidate, SessionSource};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Default, Debug, Clone)]
pub struct SessionCollector;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum DedupKey {
    CanonicalPath(PathBuf),
    Name(String),
}

impl SessionCollector {
    pub fn new() -> Self {
        Self
    }

    /// Deduplicates candidate targets.
    /// Real paths are resolved to their canonical filesystem path.
    /// Non-directory or unresolvable targets fall back to the session name.
    /// High-priority sources shadow lower-priority sources, while inheriting alias and icon metadata.
    /// If `hide_attached` is true, attached tmux sessions are removed.
    pub fn dedup_candidates(
        &self,
        candidates: Vec<SessionCandidate>,
        hide_attached: bool,
    ) -> Vec<SessionCandidate> {
        let mut key_to_index: HashMap<DedupKey, usize> = HashMap::new();
        let mut result: Vec<SessionCandidate> = Vec::with_capacity(candidates.len());

        for mut candidate in candidates {
            if hide_attached && candidate.source == SessionSource::Tmux && candidate.attached {
                continue;
            }

            let key = match fs::canonicalize(&candidate.path) {
                Ok(canonical) => DedupKey::CanonicalPath(canonical),
                Err(_) => DedupKey::Name(candidate.name.clone()),
            };

            match key_to_index.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    let existing_idx = *entry.get();
                    let existing = &mut result[existing_idx];

                    if candidate.source.priority() < existing.source.priority() {
                        candidate.inherit_metadata(existing);
                        *existing = candidate;
                    } else {
                        existing.inherit_metadata(&candidate);
                    }
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let idx = result.len();
                    result.push(candidate);
                    entry.insert(idx);
                }
            }
        }

        result
    }

    /// Discovers candidates across tmux, config, git, and zoxide, then deduplicates them.
    pub fn collect_from_system(&self, config: &omega_config::Config) -> Vec<SessionCandidate> {
        let mut all = Vec::new();

        // 1. Tmux
        if let Ok(output) = std::process::Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}:#{session_path}:#{session_attached}:#{session_windows}",
            ])
            .output()
            && output.status.success()
        {
            let raw = String::from_utf8_lossy(&output.stdout);
            all.extend(crate::discovery::parse_tmux_output(&raw));
        }

        // 2. Config aliases & static roots
        let (aliases, static_roots) = crate::discovery::scan_config_sources(config);
        all.extend(aliases);
        all.extend(static_roots);

        // 3. Git repositories in configured roots (or current working directory)
        let git_roots = if !config.roots.is_empty() {
            config.roots.clone()
        } else if let Ok(cwd) = std::env::current_dir() {
            vec![cwd]
        } else {
            Vec::new()
        };

        if !git_roots.is_empty() {
            let git_repos = crate::discovery::scan_git_repositories(&git_roots, 3);
            all.extend(git_repos);
        }

        // 4. Zoxide frecency
        if let Ok(output) = std::process::Command::new("zoxide")
            .args(["query", "--list", "--score"])
            .output()
            && output.status.success()
        {
            let raw = String::from_utf8_lossy(&output.stdout);
            all.extend(crate::discovery::parse_zoxide_output(&raw));
        }

        let mut results = self.dedup_candidates(all, config.hide_attached);
        let home = std::env::var("HOME").ok();
        for cand in &mut results {
            if cand.icon.is_none() {
                if let Some(rule) = crate::layout::match_wildcard_with_home(
                    &cand.path,
                    &config.wildcards,
                    home.as_deref(),
                ) && let Some(icon) = &rule.icon
                {
                    cand.icon = Some(icon.clone());
                } else if let Some(icon) = config.icons.get(cand.source.as_str()) {
                    cand.icon = Some(icon.clone());
                }
            }
        }
        results
    }
}
