use std::path::Path;

#[derive(Default, Debug, Clone)]
pub struct NamingPipeline;

impl NamingPipeline {
    pub fn new() -> Self {
        Self
    }

    /// Resolves a deterministic tmux session name for a path.
    pub fn resolve_name(
        &self,
        path: &Path,
        dir_length: usize,
        existing_sessions: &[(&str, &Path)],
    ) -> String {
        self.resolve_full(path, dir_length, existing_sessions, None, &[])
    }

    /// Full naming pipeline:
    /// 1. Custom Lua Naming Hook (if Some)
    /// 2. Plain Directory segments (last `dir_length` non-empty components)
    /// 3. Name Substitutions (find/replace)
    /// 4. Tmux Sanitization (replace illegal characters '.' and ':' with '_')
    /// 5. Collision Resolution (append -2, -3 if taken by another directory)
    pub fn resolve_full(
        &self,
        path: &Path,
        dir_length: usize,
        existing_sessions: &[(&str, &Path)],
        custom_hook_result: Option<&str>,
        substitutions: &[omega_config::NameSubstitution],
    ) -> String {
        let mut path_str = path.to_string_lossy().into_owned();
        for subst in substitutions {
            if path_str.contains(&subst.find) {
                path_str = path_str.replace(&subst.find, &subst.replace);
            }
        }
        let effective_path = Path::new(&path_str);

        let mut base_raw = if let Some(custom) = custom_hook_result {
            custom.to_owned()
        } else {
            let components: Vec<&str> = effective_path
                .components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(os_str) => os_str.to_str(),
                    _ => None,
                })
                .collect();

            let count = dir_length.max(1);
            let selected: &[&str] = if components.len() <= count {
                &components
            } else {
                &components[components.len() - count..]
            };

            if selected.is_empty() {
                "root".to_owned()
            } else {
                selected.join("-")
            }
        };

        for subst in substitutions {
            if base_raw.contains(&subst.find) {
                base_raw = base_raw.replace(&subst.find, &subst.replace);
            }
        }

        let sanitized = self.sanitize(&base_raw);

        // Check collision
        let mut candidate_name = sanitized.clone();
        let mut counter = 2;

        loop {
            let conflict = existing_sessions
                .iter()
                .find(|(name, _)| *name == candidate_name);
            match conflict {
                Some((_, existing_path)) => {
                    if *existing_path == path {
                        // Same path: no collision
                        return candidate_name;
                    }
                    // Different path with same name: append incrementing suffix
                    candidate_name = format!("{}-{}", sanitized, counter);
                    counter += 1;
                }
                None => {
                    return candidate_name;
                }
            }
        }
    }

    /// Resolves normalized session name for a Git worktree: `<repo>--<sanitized-branch>`
    pub fn resolve_worktree_name(&self, repo: &str, branch: &str) -> String {
        let repo_sanitized = self.sanitize(repo);
        let branch_sanitized = self.sanitize_branch(branch);
        format!("{}--{}", repo_sanitized, branch_sanitized)
    }

    /// Sanitizes branch name: converts slashes to '-' and characters illegal in tmux ('.' and ':') to '_'.
    pub fn sanitize_branch(&self, branch: &str) -> String {
        branch
            .chars()
            .map(|c| match c {
                '/' => '-',
                '.' | ':' => '_',
                other => other,
            })
            .collect()
    }

    /// Replaces characters illegal in tmux ('.' and ':') with '_'.
    pub fn sanitize(&self, name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '.' | ':' => '_',
                other => other,
            })
            .collect()
    }
}
