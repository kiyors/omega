use omega_search::SearchEngine;

#[test]
fn test_empty_query_preserves_all() {
    let engine = SearchEngine::new();
    let candidates = vec!["omega", "dotfiles", "scratch"];
    let results = engine.search("", &candidates);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].index, 0);
    assert_eq!(results[1].index, 1);
    assert_eq!(results[2].index, 2);
}

#[test]
fn test_fuzzy_scoring_orders_best_match_first() {
    let engine = SearchEngine::new();
    let candidates = vec!["some_other_thing", "omega", "om_e_ga"];
    let results = engine.search("omega", &candidates);

    assert!(!results.is_empty());
    // Exact match "omega" (index 1) should rank higher than loose match "om_e_ga" (index 2)
    assert_eq!(results[0].index, 1);
}

#[test]
fn test_no_match_returns_empty() {
    let engine = SearchEngine::new();
    let candidates = vec!["omega", "tmux"];
    let results = engine.search("xyz123", &candidates);

    assert!(results.is_empty());
}

#[test]
fn test_case_insensitive_matching() {
    let engine = SearchEngine::new();
    let candidates = vec!["Omega-Session", "other"];
    let results = engine.search("omega", &candidates);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].index, 0);
}
