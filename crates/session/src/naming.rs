use std::path::Path;

#[derive(Default, Debug, Clone)]
pub struct NamingPipeline;

impl NamingPipeline {
    pub fn new() -> Self {
        Self
    }

    /// Resolves a deterministic tmux session name for a path.
    /// 1. Extracts the last `dir_length` non-empty path components.
    /// 2. Sanitizes characters illegal in tmux ('.' and ':') by replacing them with '_'.
    /// 3. Resolves collisions against existing sessions.
    pub fn resolve_name(
        &self,
        path: &Path,
        dir_length: usize,
        existing_sessions: &[(&str, &Path)],
    ) -> String {
        let components: Vec<&str> = path
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

        let base_raw = if selected.is_empty() {
            "root".to_string()
        } else {
            selected.join("-")
        };

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
