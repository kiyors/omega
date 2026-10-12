use omega_search::SearchEngine;
use omega_session::{SessionCandidate, SessionSource};

pub struct App {
    pub query: String,
    pub items: Vec<SessionCandidate>,
    pub filtered: Vec<usize>, // Indices pointing into `items`
    pub selected: usize,
    pub search_engine: SearchEngine,
}

impl App {
    pub fn new(items: Vec<SessionCandidate>, initial_query: Option<String>) -> Self {
        let count = items.len();
        let query = initial_query.unwrap_or_default();
        let mut app = Self {
            query,
            items,
            filtered: (0..count).collect(),
            selected: 0,
            search_engine: SearchEngine::new(),
        };
        app.refilter();
        app
    }

    pub fn refilter(&mut self) {
        let trimmed = self.query.trim();

        // Check for interactive sigil prefixes
        let (source_filter, search_query) = if let Some(rest) = trimmed.strip_prefix(":t ") {
            (Some(SessionSource::Tmux), rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix(":g ") {
            (Some(SessionSource::Git), rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix('@') {
            (Some(SessionSource::Alias), rest.trim())
        } else {
            (None, trimmed)
        };

        // Filter indices by source if requested
        let candidate_indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if let Some(expected_source) = source_filter {
                    if item.source == expected_source {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    Some(idx)
                }
            })
            .collect();

        if search_query.is_empty() {
            self.filtered = candidate_indices;
        } else {
            let names: Vec<&str> = candidate_indices
                .iter()
                .map(|&idx| self.items[idx].name.as_str())
                .collect();

            let matches = self.search_engine.search(search_query, &names);
            self.filtered = matches
                .into_iter()
                .map(|m| candidate_indices[m.index])
                .collect();
        }

        if self.selected >= self.filtered.len() {
            self.selected = if self.filtered.is_empty() {
                0
            } else {
                self.filtered.len() - 1
            };
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn selected_item(&self) -> Option<&SessionCandidate> {
        self.filtered
            .get(self.selected)
            .map(|&idx| &self.items[idx])
    }
}
