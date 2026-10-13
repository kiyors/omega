use omega_session::SessionCandidate;
use omega_session::discovery::parse_tmux_window_candidates;
use omega_session::models::SessionSource;
use omega_session::preview::PreviewGenerator;
use omega_tmux::MockTmuxClient;
use std::path::PathBuf;

#[test]
fn test_window_source_sigils_and_attributes() {
    assert_eq!(SessionSource::Window.as_str(), "window");
    assert_eq!(SessionSource::Window.sigil_prefix(), ":w ");

    assert_eq!(
        SessionSource::from_sigil(":w"),
        Some((SessionSource::Window, ""))
    );
    assert_eq!(
        SessionSource::from_sigil(":w editor"),
        Some((SessionSource::Window, "editor"))
    );
    assert_eq!(SessionSource::from_sigil(":window"), None);
}

#[test]
fn test_parse_tmux_window_candidates() {
    let raw = "\
omega\teditor\t/Users/gaurav/personal/projects/omega\t1
omega\ttests\t/Users/gaurav/personal/projects/omega\t0
dotfiles\tconfig\t/Users/gaurav/.dotfiles\t1
";

    let candidates = parse_tmux_window_candidates(raw);
    assert_eq!(candidates.len(), 3);

    assert_eq!(candidates[0].name, "omega:editor");
    assert_eq!(
        candidates[0].path,
        PathBuf::from("/Users/gaurav/personal/projects/omega")
    );
    assert_eq!(candidates[0].source, SessionSource::Window);
    assert!(candidates[0].attached);

    assert_eq!(candidates[1].name, "omega:tests");
    assert_eq!(candidates[1].source, SessionSource::Window);
    assert!(!candidates[1].attached);

    assert_eq!(candidates[2].name, "dotfiles:config");
    assert_eq!(candidates[2].source, SessionSource::Window);
    assert!(candidates[2].attached);
}

#[test]
fn test_window_preview_captures_pane() {
    let mut mock = MockTmuxClient::new();
    mock.captured_panes.insert(
        "omega:editor".to_string(),
        "\u{1b}[32mWelcome to Neovim\u{1b}[0m\n".to_string(),
    );

    let candidate = SessionCandidate::new(
        "omega:editor",
        "/Users/gaurav/personal/projects/omega",
        SessionSource::Window,
    );

    let generator = PreviewGenerator::new();
    let preview = generator.generate(&candidate, &mock);

    assert!(
        preview.contains("Welcome to Neovim"),
        "Preview should capture pane content for window targets"
    );
}

#[test]
fn test_benchmark_window_aggregation_1000() {
    use std::time::Instant;

    let mut raw = String::with_capacity(1000 * 60);
    for i in 0..1000 {
        use std::fmt::Write;
        let _ = writeln!(raw, "session_{i}\twindow_{i}\t/tmp/path/{i}\t0");
    }

    let start = Instant::now();
    let candidates = parse_tmux_window_candidates(&raw);
    let duration = start.elapsed();

    assert_eq!(candidates.len(), 1000);
    assert!(
        duration.as_millis() < 15,
        "Aggregation of 1,000 windows took {:?}, must be < 15ms",
        duration
    );
}
