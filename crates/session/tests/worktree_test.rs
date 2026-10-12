use omega_session::naming::NamingPipeline;
use omega_session::{SessionSource, scan_git_repositories};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_worktree_session_naming_sanitization() {
    let namer = NamingPipeline::new();

    // Standard branch
    let name1 = namer.resolve_worktree_name("omega", "feature-preview");
    assert_eq!(name1, "omega--feature-preview");

    // Branch with slashes, dots, and colons
    let name2 = namer.resolve_worktree_name("omega", "feat/user:profile.v2");
    assert_eq!(name2, "omega--feat-user_profile_v2");

    // Collision handling with existing sessions
    let existing = [("omega--main", std::path::Path::new("/work/omega-main"))];
    let name3 = namer.resolve_worktree_name_with_existing(
        "omega",
        "main",
        std::path::Path::new("/work/omega-main-other"),
        &existing,
    );
    assert_eq!(name3, "omega--main-2");
}

#[test]
fn test_linked_worktree_discovery_and_metadata() {
    let dir = tempdir().expect("tempdir failed");
    let base = dir.path();

    // 1. Create main git repository: base/main_repo
    let main_repo = base.join("myrepo");
    let git_dir = main_repo.join(".git");
    fs::create_dir_all(git_dir.join("worktrees/feature-box")).expect("mkdir failed");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write failed");

    // Worktree admin dir in main repo
    let wt_admin = git_dir.join("worktrees/feature-box");
    fs::write(wt_admin.join("HEAD"), "ref: refs/heads/feat/preview-box\n").expect("write failed");

    // 2. Create linked worktree: base/myrepo-feat
    let wt_repo = base.join("myrepo-feat");
    fs::create_dir_all(&wt_repo).expect("mkdir failed");
    fs::write(
        wt_repo.join(".git"),
        format!("gitdir: {}\n", wt_admin.display()),
    )
    .expect("write failed");

    // Scan repositories in base
    let candidates = scan_git_repositories(&[base.to_path_buf()], 3);

    let main_cand = candidates
        .iter()
        .find(|c| c.path == main_repo)
        .expect("main repo candidate should be found");
    assert_eq!(main_cand.source, SessionSource::Git);
    assert_eq!(main_cand.name, "myrepo");

    let wt_cand = candidates
        .iter()
        .find(|c| c.path == wt_repo)
        .expect("worktree candidate should be found");
    assert_eq!(wt_cand.source, SessionSource::Git);
    // Normalized to <repo>--<sanitized-branch>
    assert_eq!(wt_cand.name, "myrepo--feat-preview-box");
}
