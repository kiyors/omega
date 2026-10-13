use omega_config::SortTier;
use omega_session::{SessionCandidate, SessionCollector, SessionSource};
use std::path::PathBuf;

#[test]
fn test_sort_order_tier_prioritization() {
    let collector = SessionCollector::new();

    let candidates = vec![
        SessionCandidate::new(
            "zoxide-proj",
            PathBuf::from("/p/zoxide"),
            SessionSource::Zoxide,
        ),
        SessionCandidate::new("tmux-proj", PathBuf::from("/p/tmux"), SessionSource::Tmux),
        SessionCandidate::new(
            "alias-proj",
            PathBuf::from("/p/alias"),
            SessionSource::Alias,
        ),
        SessionCandidate::new("git-proj", PathBuf::from("/p/git"), SessionSource::Git),
    ];

    let sort_order = vec![
        SortTier::Single("alias".to_string()),
        SortTier::Single("tmux".to_string()),
        SortTier::Single("git".to_string()),
    ];

    let sorted = collector.apply_sort_order(candidates, &sort_order);

    assert_eq!(sorted[0].name, "alias-proj");
    assert_eq!(sorted[1].name, "tmux-proj");
    assert_eq!(sorted[2].name, "git-proj");
    assert_eq!(sorted[3].name, "zoxide-proj"); // Unlisted goes last
}

#[test]
fn test_sort_order_interleaving_with_frecency_scores() {
    let collector = SessionCollector::new();

    let mut cand_git_low =
        SessionCandidate::new("git-low", PathBuf::from("/p/git-low"), SessionSource::Git);
    cand_git_low.score = Some(20.0);

    let mut cand_zox_high = SessionCandidate::new(
        "zox-high",
        PathBuf::from("/p/zox-high"),
        SessionSource::Zoxide,
    );
    cand_zox_high.score = Some(150.0);

    let mut cand_git_top =
        SessionCandidate::new("git-top", PathBuf::from("/p/git-top"), SessionSource::Git);
    cand_git_top.score = Some(300.0);

    let mut cand_zox_mid = SessionCandidate::new(
        "zox-mid",
        PathBuf::from("/p/zox-mid"),
        SessionSource::Zoxide,
    );
    cand_zox_mid.score = Some(80.0);

    let cand_tmux = SessionCandidate::new("tmux-s", PathBuf::from("/p/tmux"), SessionSource::Tmux);

    let candidates = vec![
        cand_git_low,
        cand_zox_high,
        cand_tmux,
        cand_git_top,
        cand_zox_mid,
    ];

    // Tier 0: "tmux", Tier 1: ["git", "zoxide"] interleaved by score descending
    let sort_order = vec![
        SortTier::Single("tmux".to_string()),
        SortTier::Group(vec!["git".to_string(), "zoxide".to_string()]),
    ];

    let sorted = collector.apply_sort_order(candidates, &sort_order);

    assert_eq!(sorted[0].name, "tmux-s");
    assert_eq!(sorted[1].name, "git-top"); // 300.0
    assert_eq!(sorted[2].name, "zox-high"); // 150.0
    assert_eq!(sorted[3].name, "zox-mid"); // 80.0
    assert_eq!(sorted[4].name, "git-low"); // 20.0
}

#[test]
fn test_sort_order_custom_source_alias_name() {
    let collector = SessionCollector::new();

    let cand_tmux = SessionCandidate::new("tmux-s", PathBuf::from("/p/tmux"), SessionSource::Tmux);
    let cand_docker =
        SessionCandidate::from_custom("docker:redis", PathBuf::from("/p/docker"), "docker", None);

    let candidates = vec![cand_tmux, cand_docker];

    let sort_order = vec![
        SortTier::Single("docker".to_string()),
        SortTier::Single("tmux".to_string()),
    ];

    let sorted = collector.apply_sort_order(candidates, &sort_order);

    assert_eq!(sorted[0].name, "docker:redis");
    assert_eq!(sorted[1].name, "tmux-s");
}

#[test]
fn test_sort_order_with_custom_source_does_not_collide_with_alias() {
    let collector = SessionCollector::new();

    let cand_tmux = SessionCandidate::new("tmux-s", PathBuf::from("/p/tmux"), SessionSource::Tmux);
    let cand_alias =
        SessionCandidate::new("alias-s", PathBuf::from("/p/alias"), SessionSource::Alias);
    let cand_docker =
        SessionCandidate::from_custom("docker:redis", PathBuf::from("/p/docker"), "docker", None);

    // sort_order prioritizes alias, then docker, then tmux
    let sort_order = vec![
        SortTier::Single("alias".to_string()),
        SortTier::Single("docker".to_string()),
        SortTier::Single("tmux".to_string()),
    ];

    let sorted = collector.apply_sort_order(vec![cand_tmux, cand_docker, cand_alias], &sort_order);

    assert_eq!(sorted[0].name, "alias-s");
    assert_eq!(sorted[1].name, "docker:redis");
    assert_eq!(sorted[2].name, "tmux-s");
}

#[test]
fn test_sort_order_empty_preserves_original_order() {
    let collector = SessionCollector::new();

    let candidates = vec![
        SessionCandidate::new("one", PathBuf::from("/p/1"), SessionSource::Zoxide),
        SessionCandidate::new("two", PathBuf::from("/p/2"), SessionSource::Git),
        SessionCandidate::new("three", PathBuf::from("/p/3"), SessionSource::Tmux),
    ];

    let sorted = collector.apply_sort_order(candidates, &[]);

    assert_eq!(sorted[0].name, "one");
    assert_eq!(sorted[1].name, "two");
    assert_eq!(sorted[2].name, "three");
}
