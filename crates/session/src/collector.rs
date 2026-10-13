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
        let mut key_to_index: HashMap<DedupKey, usize> = HashMap::with_capacity(candidates.len());
        let mut result: Vec<SessionCandidate> = Vec::with_capacity(candidates.len());

        for mut candidate in candidates {
            if hide_attached && candidate.source == SessionSource::Tmux && candidate.attached {
                continue;
            }

            let key = if candidate.source == SessionSource::Window {
                DedupKey::Name(candidate.name.clone())
            } else {
                match fs::canonicalize(&candidate.path) {
                    Ok(canonical) => DedupKey::CanonicalPath(canonical),
                    Err(_) => DedupKey::Name(candidate.name.clone()),
                }
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

    /// Groups and orders candidates according to declarative `sort_order` configuration.
    ///
    /// `sort_order` is an array of tiers:
    /// - A single source (e.g. `SortTier::Single("tmux")`) assigns that source to that tier.
    /// - A grouped source (e.g. `SortTier::Group(vec!["config", "git", "zoxide"])`) groups those sources into the same tier,
    ///   interleaving candidates by descending frecency `score`.
    /// - Candidates with no explicit tier match are placed at the final tier.
    /// - Empty `sort_order` preserves the original candidate ordering.
    pub fn apply_sort_order(
        &self,
        candidates: Vec<SessionCandidate>,
        sort_order: &[omega_config::SortTier],
    ) -> Vec<SessionCandidate> {
        if sort_order.is_empty() {
            return candidates;
        }

        let find_tier = |cand: &SessionCandidate| -> usize {
            for (tier_idx, tier) in sort_order.iter().enumerate() {
                if tier.matches(
                    cand.source.as_str(),
                    cand.alias.as_deref(),
                    cand.custom_source.as_deref(),
                ) {
                    return tier_idx;
                }
            }
            sort_order.len()
        };

        // Precompute tiers once to avoid O(N log N) redundant traversals during sorting
        let tiers: Vec<usize> = candidates.iter().map(find_tier).collect();
        let mut indices: Vec<usize> = (0..candidates.len()).collect();

        indices.sort_by(|&a, &b| {
            if tiers[a] != tiers[b] {
                tiers[a].cmp(&tiers[b])
            } else {
                let score_a = candidates[a].score.unwrap_or(0.0);
                let score_b = candidates[b].score.unwrap_or(0.0);
                let score_cmp = score_b.total_cmp(&score_a);
                if score_cmp != std::cmp::Ordering::Equal {
                    score_cmp
                } else {
                    a.cmp(&b)
                }
            }
        });

        let mut sorted = Vec::with_capacity(candidates.len());
        let mut opt_candidates: Vec<Option<SessionCandidate>> =
            candidates.into_iter().map(Some).collect();
        for idx in indices {
            if let Some(cand) = opt_candidates[idx].take() {
                sorted.push(cand);
            }
        }
        sorted
    }

    /// Discovers candidates across tmux, config, git, and zoxide, then deduplicates them.
    pub fn collect_from_system(&self, config: &omega_config::Config) -> Vec<SessionCandidate> {
        let mut all = Vec::new();

        // 1. Tmux sessions
        let mut tmux_sessions = Vec::new();
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
            tmux_sessions = crate::discovery::parse_tmux_output(&raw);
        }

        // 1b. Active Tmux Windows
        if let Ok(output) = std::process::Command::new("tmux")
            .args([
                "list-windows",
                "-a",
                "-F",
                "#{session_name}\t#{window_name}\t#{pane_current_path}\t#{window_active}\t#{window_id}",
            ])
            .output()
            && output.status.success()
        {
            let raw = String::from_utf8_lossy(&output.stdout);
            let window_candidates = crate::discovery::parse_tmux_window_candidates(&raw);

            // Populate window_names on the corresponding tmux session candidates
            let mut session_windows: std::collections::HashMap<&str, Vec<String>> =
                std::collections::HashMap::new();
            for line in raw.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut parts = trimmed.split('\t');
                if let (Some(s_name), Some(w_name)) = (parts.next(), parts.next()) {
                    session_windows.entry(s_name).or_default().push(w_name.to_owned());
                }
            }
            for sess in &mut tmux_sessions {
                if let Some(wins) = session_windows.get(sess.name.as_str()) {
                    sess.window_names = wins.clone();
                }
            }

            all.extend(tmux_sessions);
            all.extend(window_candidates);
        } else {
            all.extend(tmux_sessions);
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
        if !config.sort_order.is_empty() {
            results = self.apply_sort_order(results, &config.sort_order);
        }
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
