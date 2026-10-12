use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub index: usize,
    pub score: i64,
}

#[derive(Default)]
pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Match a query against a slice of candidate strings.
    /// Returns matching indices sorted by score in descending order.
    pub fn search(&self, query: &str, candidates: &[&str]) -> Vec<MatchResult> {
        if query.trim().is_empty() {
            return candidates
                .iter()
                .enumerate()
                .map(|(index, _)| MatchResult { index, score: 0 })
                .collect();
        }

        let mut results: Vec<MatchResult> = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                self.matcher
                    .fuzzy_match(candidate, query)
                    .map(|score| MatchResult { index, score })
            })
            .collect();

        results.sort_by_key(|a| std::cmp::Reverse(a.score));
        results
    }

    /// Computes match score of query against a single candidate string.
    pub fn score(&self, query: &str, candidate: &str) -> Option<i64> {
        self.matcher.fuzzy_match(candidate, query)
    }
}
