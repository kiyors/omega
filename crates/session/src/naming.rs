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
        let compiled: Vec<(&omega_config::NameSubstitution, Option<regex::Regex>)> = substitutions
            .iter()
            .map(|s| (s, regex::Regex::new(&s.find).ok()))
            .collect();

        let path_str = path.to_string_lossy();
        let transformed_path = apply_compiled_substitutions(path_str.into_owned(), &compiled);
        let effective_path = Path::new(&transformed_path);

        let base_raw = if let Some(custom) = custom_hook_result {
            custom.to_owned()
        } else {
            let count = dir_length.max(1);
            let mut rev_parts = Vec::with_capacity(count);
            for c in effective_path.components().rev() {
                if let std::path::Component::Normal(os_str) = c
                    && let Some(s) = os_str.to_str()
                {
                    rev_parts.push(s);
                    if rev_parts.len() == count {
                        break;
                    }
                }
            }
            rev_parts.reverse();

            if rev_parts.is_empty() {
                "root".to_owned()
            } else {
                rev_parts.join("-")
            }
        };

        let final_raw = apply_compiled_substitutions(base_raw, &compiled);
        let sanitized = self.sanitize(&final_raw);

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

fn apply_compiled_substitutions(
    mut target: String,
    compiled: &[(&omega_config::NameSubstitution, Option<regex::Regex>)],
) -> String {
    for (subst, re_opt) in compiled {
        if let Some(re) = re_opt {
            target = re.replace_all(&target, subst.replace.as_str()).into_owned();
        } else {
            target = target.replace(&subst.find, &subst.replace);
        }
    }
    target
}
