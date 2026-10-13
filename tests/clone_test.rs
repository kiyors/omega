use clap::Parser;
use omega::cli::{Cli, Commands};
use omega::commands::execute_command;
use omega_config::Config;
use omega_session::MockTmuxClient;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_parse_clone() {
    let args1 = ["omega", "clone", "https://github.com/kiyors/omega.git"];
    let parsed1 = Cli::parse_from(args1);
    assert_eq!(
        parsed1.command,
        Some(Commands::Clone {
            repo: "https://github.com/kiyors/omega.git".to_string(),
            dir: None,
        })
    );

    let args2 = [
        "omega",
        "clone",
        "https://github.com/kiyors/omega.git",
        "--dir",
        "custom_dir",
    ];
    let parsed2 = Cli::parse_from(args2);
    assert_eq!(
        parsed2.command,
        Some(Commands::Clone {
            repo: "https://github.com/kiyors/omega.git".to_string(),
            dir: Some(PathBuf::from("custom_dir")),
        })
    );
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_clone_command_execution() {
    if !git_available() {
        eprintln!("skipping test_clone_command_execution: git not found");
        return;
    }

    let tmp = tempdir().expect("tempdir failed");
    let base = tmp.path();

    // 1. Create a local source git repository
    let src_repo = base.join("src_repo");
    fs::create_dir_all(&src_repo).expect("mkdir failed");

    Command::new("git")
        .args(["init"])
        .current_dir(&src_repo)
        .output()
        .expect("git init failed");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&src_repo)
        .output()
        .expect("git config failed");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&src_repo)
        .output()
        .expect("git config failed");
    fs::write(src_repo.join("README.md"), "# Test\n").expect("write failed");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&src_repo)
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(&src_repo)
        .output()
        .expect("git commit failed");

    // 2. Target clone destination
    let dest_dir = base.join("cloned_repo");
    let mut mock_tmux = MockTmuxClient::default();
    let config = Config::default();

    let cmd = Commands::Clone {
        repo: src_repo.to_string_lossy().to_string(),
        dir: Some(dest_dir.clone()),
    };

    let result = execute_command(cmd, &mut mock_tmux, &config);
    assert!(result.is_ok());

    // Verify clone succeeded on filesystem
    assert!(dest_dir.join(".git").exists());
    assert!(dest_dir.join("README.md").exists());

    // Verify tmux session creation & attachment
    assert_eq!(mock_tmux.attached_to, Some("cloned_repo".to_string()));
}
