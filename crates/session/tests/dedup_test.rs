use omega_session::{SessionCandidate, SessionCollector, SessionSource};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use tempfile::tempdir;

#[test]
#[cfg(unix)]
fn test_canonical_path_deduplication_priority() {
    let tmp = tempdir().unwrap();
    let real_dir = tmp.path().join("my-project");
    fs::create_dir(&real_dir).unwrap();

    let symlink_dir = tmp.path().join("my-symlink");
    symlink(&real_dir, &symlink_dir).unwrap();

    let candidates = vec![
        SessionCandidate::new("zoxide-proj", &symlink_dir, SessionSource::Zoxide),
        SessionCandidate::new("git-proj", &real_dir, SessionSource::Git),
        SessionCandidate::new("tmux-proj", &real_dir, SessionSource::Tmux),
    ];

    let collector = SessionCollector;
    let result = collector.dedup_candidates(candidates, false);

    assert_eq!(
        result.len(),
        1,
        "Expected exactly 1 candidate after deduplicating symlink and real path"
    );
    assert_eq!(
        result[0].source,
        SessionSource::Tmux,
        "Tmux source should have priority over Git and Zoxide"
    );
    assert_eq!(result[0].name, "tmux-proj");
}

#[test]
fn test_metadata_inheritance_from_alias() {
    let tmp = tempdir().unwrap();
    let real_dir = tmp.path().join("app");
    fs::create_dir(&real_dir).unwrap();

    let mut alias_cand = SessionCandidate::new("app-alias", &real_dir, SessionSource::Alias);
    alias_cand.alias = Some("app-alias".to_string());
    alias_cand.icon = Some("🚀".to_string());

    let tmux_cand = SessionCandidate::new("app", &real_dir, SessionSource::Tmux);

    let candidates = vec![alias_cand, tmux_cand];

    let collector = SessionCollector;
    let result = collector.dedup_candidates(candidates, false);

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].source,
        SessionSource::Tmux,
        "Tmux wins over Alias"
    );
    assert_eq!(
        result[0].alias.as_deref(),
        Some("app-alias"),
        "Winning session inherits alias name"
    );
    assert_eq!(
        result[0].icon.as_deref(),
        Some("🚀"),
        "Winning session inherits alias icon"
    );
}

#[test]
fn test_hide_attached_tmux_session() {
    let tmp = tempdir().unwrap();
    let dir1 = tmp.path().join("proj1");
    let dir2 = tmp.path().join("proj2");
    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();

    let mut attached_tmux = SessionCandidate::new("current", &dir1, SessionSource::Tmux);
    attached_tmux.attached = true;

    let other_tmux = SessionCandidate::new("other", &dir2, SessionSource::Tmux);

    let candidates = vec![attached_tmux, other_tmux];

    let collector = SessionCollector;
    let result = collector.dedup_candidates(candidates, true);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "other");
}

#[test]
fn test_non_directory_fallback_dedup() {
    let cand1 = SessionCandidate::new("remote-sesh", "/non/existent/path/1", SessionSource::Tmux);
    let cand2 = SessionCandidate::new("remote-sesh", "/non/existent/path/2", SessionSource::Tmux);
    let cand3 = SessionCandidate::new(
        "different-sesh",
        "/non/existent/path/3",
        SessionSource::Tmux,
    );

    let collector = SessionCollector;
    let result = collector.dedup_candidates(vec![cand1, cand2, cand3], false);

    assert_eq!(
        result.len(),
        2,
        "Candidates with missing paths should deduplicate by session name"
    );
    assert_eq!(result[0].name, "remote-sesh");
    assert_eq!(result[1].name, "different-sesh");
}

#[test]
fn test_sigil_matching_precision() {
    assert_eq!(
        SessionSource::from_sigil(":t"),
        Some((SessionSource::Tmux, ""))
    );
    assert_eq!(
        SessionSource::from_sigil(":t my-session"),
        Some((SessionSource::Tmux, "my-session"))
    );
    assert_eq!(SessionSource::from_sigil(":today"), None);

    assert_eq!(
        SessionSource::from_sigil(":w"),
        Some((SessionSource::Window, ""))
    );
    assert_eq!(
        SessionSource::from_sigil(":w my-win"),
        Some((SessionSource::Window, "my-win"))
    );
    assert_eq!(SessionSource::from_sigil(":window"), None);

    assert_eq!(
        SessionSource::from_sigil(":g"),
        Some((SessionSource::Git, ""))
    );
    assert_eq!(
        SessionSource::from_sigil(":g repo-name"),
        Some((SessionSource::Git, "repo-name"))
    );
    assert_eq!(SessionSource::from_sigil(":game"), None);

    assert_eq!(
        SessionSource::from_sigil("@my-alias"),
        Some((SessionSource::Alias, "my-alias"))
    );
    assert_eq!(
        SessionSource::from_sigil("@ my-alias"),
        Some((SessionSource::Alias, "my-alias"))
    );
}
