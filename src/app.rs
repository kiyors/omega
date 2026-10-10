use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Clone, Debug)]
pub enum SessionType {
    Tmux,
    #[allow(dead_code)]
    Git,
}

#[derive(Clone, Debug)]
pub struct SessionItem {
    pub name: String,
    pub path: String,
    pub kind: SessionType,
}

pub struct App {
    pub query: String,
    pub items: Vec<SessionItem>,
    pub filtered: Vec<usize>, // Indices pointing to `items`
    pub selected: usize,
    pub matcher: SkimMatcherV2,
}

impl App {
    pub fn new(items: Vec<SessionItem>) -> Self {
        let count = items.len();
        let mut app = Self {
            query: String::new(),
            items,
            filtered: (0..count).collect(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
        };
        app.refilter();
        app
    }

    pub fn refilter(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    self.matcher
                        .fuzzy_match(&item.name, &self.query)
                        .map(|score| (idx, score))
                })
                .collect();

            // Highest score first
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(idx, _)| idx).collect();
        }

        if self.selected >= self.filtered.len() && !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
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

    pub fn selected_item(&self) -> Option<&SessionItem> {
        self.filtered
            .get(self.selected)
            .map(|&idx| &self.items[idx])
    }
}
