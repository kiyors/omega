use omega_session::preview::PreviewGenerator;
use omega_session::{MockTmuxClient, SessionCandidate, SessionSource};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_tmux_session_preview() {
    let mut mock = MockTmuxClient::default();
    mock.captured_panes.insert(
        "omega-session".to_string(),
        "\x1b[32mActive Process\x1b[0m\nLine 2".to_string(),
    );

    let candidate = SessionCandidate::new("omega-session", "/tmp", SessionSource::Tmux);
    let generator = PreviewGenerator::new();
    let preview = generator.generate(&candidate, &mock);

    assert!(preview.contains("Active Process"));
    assert!(preview.contains("Line 2"));
}

#[test]
fn test_plain_directory_preview() {
    let dir = tempdir().expect("tempdir failed");
    let path = dir.path();
    fs::create_dir(path.join("src")).expect("mkdir failed");
    fs::write(path.join("src/main.rs"), "fn main() {}\n").expect("write failed");
    fs::write(path.join("Cargo.toml"), "[package]\n").expect("write failed");

    let candidate = SessionCandidate::new("testdir", path, SessionSource::Config);
    let mock = MockTmuxClient::default();
    let generator = PreviewGenerator::new();
    let preview = generator.generate(&candidate, &mock);

    assert!(preview.contains("Cargo.toml"));
    assert!(preview.contains("src"));
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_git_repository_preview() {
    if !git_available() {
        eprintln!("skipping test_git_repository_preview: git not found");
        return;
    }

    let dir = tempdir().expect("tempdir failed");
    let path = dir.path();

    // Initialize git repository
    let init = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");
    assert!(init.status.success());

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config user.name failed");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("git config user.email failed");

    fs::write(path.join("hello.txt"), "hello world\n").expect("write failed");

    Command::new("git")
        .args(["add", "hello.txt"])
        .current_dir(path)
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(["commit", "-m", "feat: initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    // Add uncommitted file
    fs::write(path.join("uncommitted.txt"), "changes\n").expect("write failed");

    let candidate = SessionCandidate::new("git-test", path, SessionSource::Git);
    let mock = MockTmuxClient::default();
    let generator = PreviewGenerator::new();
    let preview = generator.generate(&candidate, &mock);

    assert!(preview.contains("initial commit") || preview.contains("feat:"));
    assert!(preview.contains("uncommitted.txt") || preview.contains("??"));
}
