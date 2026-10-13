use omega_config::TuiConfig;
use omega_search::SearchEngine;
use omega_session::{SessionCandidate, SessionSource};

pub struct App {
    pub query: String,
    pub items: Vec<SessionCandidate>,
    pub filtered: Vec<usize>, // Reusable index buffer pointing into `items`
    pub selected: usize,
    pub search_engine: SearchEngine,

    // Preview settings
    pub preview_enabled: bool,
    pub preview_width: u16,
    pub preview_scroll: u16,
    pub preview_content: String,
    pub preview_border: String,

    // Popup & Layout customizations
    pub popup_enabled: bool,
    pub popup_width: u16,
    pub popup_height: u16,
    pub popup_border: String,
    pub border_color: String,
    pub highlight_color: String,
    pub search_prompt: String,
    pub search_title: String,
    pub show_badges: bool,
    pub show_icons: bool,
    pub show_path: bool,
    pub show_footer: bool,

    scored_buffer: Vec<(usize, i64)>, // Reusable scoring buffer avoiding allocations
}

impl App {
    #[must_use]
    pub fn new(items: Vec<SessionCandidate>, initial_query: Option<String>) -> Self {
        let count = items.len();
        let query = initial_query.unwrap_or_default();
        let default_tui = TuiConfig::default();

        let mut app = Self {
            query,
            items,
            filtered: Vec::with_capacity(count),
            selected: 0,
            search_engine: SearchEngine::new(),
            preview_enabled: default_tui.preview,
            preview_width: default_tui.preview_width,
            preview_scroll: 0,
            preview_content: String::new(),
            preview_border: default_tui.preview_border,
            popup_enabled: default_tui.popup,
            popup_width: default_tui.popup_width,
            popup_height: default_tui.popup_height,
            popup_border: default_tui.popup_border,
            border_color: default_tui.border_color,
            highlight_color: default_tui.highlight_color,
            search_prompt: default_tui.search_prompt,
            search_title: default_tui.search_title,
            show_badges: default_tui.show_badges,
            show_icons: default_tui.show_icons,
            show_path: default_tui.show_path,
            show_footer: default_tui.show_footer,
            scored_buffer: Vec::with_capacity(count),
        };
        app.refilter();
        app
    }

    pub fn apply_tui_config(&mut self, cfg: &TuiConfig) {
        self.preview_enabled = cfg.preview;
        self.preview_width = cfg.preview_width;
        self.preview_border.clone_from(&cfg.preview_border);
        self.popup_enabled = cfg.popup;
        self.popup_width = cfg.popup_width;
        self.popup_height = cfg.popup_height;
        self.popup_border.clone_from(&cfg.popup_border);
        self.border_color.clone_from(&cfg.border_color);
        self.highlight_color.clone_from(&cfg.highlight_color);
        self.search_prompt.clone_from(&cfg.search_prompt);
        self.search_title.clone_from(&cfg.search_title);
        self.show_badges = cfg.show_badges;
        self.show_icons = cfg.show_icons;
        self.show_path = cfg.show_path;
        self.show_footer = cfg.show_footer;
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
                self.filtered.len().saturating_sub(1)
            };
        }
        self.preview_scroll = 0;
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected.saturating_add(1) < self.filtered.len() {
            self.selected = self.selected.saturating_add(1);
            self.preview_scroll = 0;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected = self.selected.saturating_sub(1);
            self.preview_scroll = 0;
        }
    }

    pub fn scroll_preview_up(&mut self) {
        if self.preview_scroll > 0 {
            self.preview_scroll = self.preview_scroll.saturating_sub(1);
        }
    }

    pub fn scroll_preview_down(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_add(1);
    }

    pub fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
    }

    #[must_use]
    pub fn selected_item(&self) -> Option<&SessionCandidate> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
    }

    pub fn remove_selected(&mut self) -> Option<SessionCandidate> {
        if self.filtered.is_empty() {
            return None;
        }
        let original_idx = self.filtered.get(self.selected).copied()?;
        if original_idx < self.items.len() {
            let removed = self.items.remove(original_idx);
            self.refilter();
            Some(removed)
        } else {
            None
        }
    }
}
