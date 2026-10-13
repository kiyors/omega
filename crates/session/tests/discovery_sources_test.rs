use omega_config::{AliasConfig, Config};
use omega_session::discovery::{
    parse_tmux_output, parse_zoxide_output, scan_config_sources, scan_git_repositories,
};
use omega_session::models::SessionSource;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_parse_tmux_output() {
    let raw_output = "\
omega:/Users/gaurav/personal/projects/omega:1:2
dotfiles:/Users/gaurav/.dotfiles:0:1
scratch:/tmp/scratch:0:3
";
    let candidates = parse_tmux_output(raw_output);
    assert_eq!(candidates.len(), 3);

    assert_eq!(candidates[0].name, "omega");
    assert_eq!(
        candidates[0].path.to_str().unwrap(),
        "/Users/gaurav/personal/projects/omega"
    );
    assert_eq!(candidates[0].source, SessionSource::Tmux);
    assert!(candidates[0].attached);
    assert_eq!(candidates[0].windows, Some(2));

    assert_eq!(candidates[1].name, "dotfiles");
    assert!(!candidates[1].attached);
    assert_eq!(candidates[1].windows, Some(1));
}

#[test]
fn test_parse_zoxide_output() {
    let raw_output = "\
 125.5  /Users/gaurav/personal/projects/omega
 42.0  /Users/gaurav/.dotfiles
/Users/gaurav/plain/path
";
    let candidates = parse_zoxide_output(raw_output);
    assert_eq!(candidates.len(), 3);

    assert_eq!(candidates[0].name, "omega");
    assert_eq!(
        candidates[0].path.to_str().unwrap(),
        "/Users/gaurav/personal/projects/omega"
    );
    assert_eq!(candidates[0].source, SessionSource::Zoxide);
    assert_eq!(candidates[0].score, Some(125.5));

    assert_eq!(candidates[1].name, ".dotfiles");
    assert_eq!(candidates[1].score, Some(42.0));

    assert_eq!(candidates[2].name, "path");
    assert_eq!(candidates[2].score, Some(0.0));
}

#[test]
fn test_scan_git_repositories() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Repo 1: normal git repo with .git directory
    let repo1 = root.join("repo-one");
    fs::create_dir_all(repo1.join(".git")).unwrap();

    // Repo 2: linked worktree with .git file pointer
    let repo2 = root.join("repo-two");
    fs::create_dir_all(&repo2).unwrap();
    let mut git_file = File::create(repo2.join(".git")).unwrap();
    writeln!(git_file, "gitdir: /some/path").unwrap();

    // Non-repo directory
    let normal_dir = root.join("plain-folder");
    fs::create_dir_all(&normal_dir).unwrap();

    let candidates = scan_git_repositories(&[root.to_path_buf()], 2);
    assert_eq!(candidates.len(), 2);

    let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"repo-one"));
    assert!(names.contains(&"repo-two"));
    assert!(!names.contains(&"plain-folder"));
}

#[test]
fn test_scan_config_sources() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let proj = root.join("workspace-proj");
    fs::create_dir(&proj).unwrap();

    let config = Config {
        hide_attached: true,
        dir_length: 1,
        roots: vec![root.to_path_buf()],
        aliases: vec![AliasConfig {
            name: "my-alias".to_string(),
            path: proj.to_str().unwrap().to_string(),
            icon: Some("🦀".to_string()),
            windows: Vec::new(),
        }],
        ..Default::default()
    };

    let (aliases, static_roots) = scan_config_sources(&config);
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0].name, "my-alias");
    assert_eq!(aliases[0].source, SessionSource::Alias);
    assert_eq!(aliases[0].icon.as_deref(), Some("🦀"));

    assert_eq!(static_roots.len(), 1);
    assert_eq!(static_roots[0].name, "workspace-proj");
    assert_eq!(static_roots[0].source, SessionSource::Config);
}

#[test]
fn test_find_git_context() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("my-rust-repo");
    let git_dir = repo_dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let head_file = git_dir.join("HEAD");
    let mut f = File::create(head_file).unwrap();
    writeln!(f, "ref: refs/heads/main").unwrap();

    // From inside repo
    let subfolder = repo_dir.join("src/models");
    fs::create_dir_all(&subfolder).unwrap();

    let ctx = omega_session::find_git_context(&subfolder);
    assert!(ctx.is_some());
    let unwrapped = ctx.unwrap();
    assert_eq!(unwrapped.repo, "my-rust-repo");
    assert_eq!(unwrapped.branch, "main");

    // Outside repo
    let non_repo = tmp.path().join("other");
    fs::create_dir(&non_repo).unwrap();
    assert!(omega_session::find_git_context(&non_repo).is_none());
}
