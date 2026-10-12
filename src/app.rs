use omega_search::SearchEngine;
use omega_session::{SessionCandidate, SessionSource};

pub struct App {
    pub query: String,
    pub items: Vec<SessionCandidate>,
    pub filtered: Vec<usize>, // Reusable index buffer pointing into `items`
    pub selected: usize,
    pub search_engine: SearchEngine,
    pub preview_enabled: bool,
    pub preview_width: u16,
    pub preview_scroll: u16,
    pub preview_content: String,
    pub preview_border: String,
    scored_buffer: Vec<(usize, i64)>, // Reusable scoring buffer avoiding allocations
}

impl App {
    pub fn new(items: Vec<SessionCandidate>, initial_query: Option<String>) -> Self {
        let count = items.len();
        let query = initial_query.unwrap_or_default();
        let mut app = Self {
            query,
            items,
            filtered: Vec::with_capacity(count),
            selected: 0,
            search_engine: SearchEngine::new(),
            preview_enabled: true,
            preview_width: 55,
            preview_scroll: 0,
            preview_content: String::new(),
            preview_border: "rounded".to_string(),
            scored_buffer: Vec::with_capacity(count),
        };
        app.refilter();
        app
    }

    pub fn refilter(&mut self) {
        let trimmed = self.query.trim();

        // Check for interactive sigil prefixes using domain method
        let (source_filter, search_query) = match SessionSource::from_sigil(trimmed) {
            Some((source, query)) => (Some(source), query),
            None => (None, trimmed),
        };

        self.filtered.clear();

        if search_query.is_empty() {
            for (idx, item) in self.items.iter().enumerate() {
                if let Some(expected) = source_filter
                    && item.source != expected
                {
                    continue;
                }
                self.filtered.push(idx);
            }
        } else {
            self.scored_buffer.clear();
            for (idx, item) in self.items.iter().enumerate() {
                if let Some(expected) = source_filter
                    && item.source != expected
                {
                    continue;
                }
                if let Some(score) = self.search_engine.score(search_query, &item.name) {
                    self.scored_buffer.push((idx, score));
                }
            }

            self.scored_buffer
                .sort_by_key(|&(_, score)| std::cmp::Reverse(score));

            self.filtered
                .extend(self.scored_buffer.iter().map(|&(idx, _)| idx));
        }

        if self.selected >= self.filtered.len() {
            self.selected = if self.filtered.is_empty() {
                0
            } else {
                self.filtered.len() - 1
            };
        }
        self.preview_scroll = 0;
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            self.preview_scroll = 0;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.preview_scroll = 0;
        }
    }

    pub fn scroll_preview_up(&mut self) {
        if self.preview_scroll > 0 {
            self.preview_scroll -= 1;
        }
    }

    pub fn scroll_preview_down(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_add(1);
    }

    pub fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
    }

    pub fn selected_item(&self) -> Option<&SessionCandidate> {
        self.filtered
            .get(self.selected)
            .map(|&idx| &self.items[idx])
    }
}
