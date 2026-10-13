use omega_session::{SessionCandidate, SessionSource};
use omega_ui::app::App;
use omega_ui::render::{format_windows_peek, render};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn test_format_windows_peek_formatting_and_elision() {
    let names = vec![
        "editor".to_owned(),
        "term".to_owned(),
        "window-three".to_owned(),
    ];

    // With generous width, all window names are visible
    let wide = format_windows_peek(&names, 40).unwrap();
    assert_eq!(wide, "  editor term window-three");

    // With medium width, fits editor and term, elides (+1)
    let med = format_windows_peek(&names, 22).unwrap();
    assert_eq!(med, "  editor term (+1)");

    // With tight width, fits only editor (+2)
    let tight = format_windows_peek(&names, 16).unwrap();
    assert_eq!(tight, "  editor (+2)");

    // Empty list returns None
    assert!(format_windows_peek(&[], 40).is_none());
}

#[test]
fn test_render_with_inline_windows() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut session = SessionCandidate::new("my-session", "/path", SessionSource::Tmux);
    session.window_names = vec!["code".to_owned(), "cargo".to_owned()];

    let mut app = App::new(vec![session], None);
    app.tui.show_windows = true;

    terminal.draw(|f| render(f, &app)).unwrap();

    let buffer = terminal.backend().buffer();
    let screen_text: String = (0..30)
        .flat_map(|y| (0..100).filter_map(move |x| buffer.cell((x, y)).map(|c| c.symbol())))
        .collect();

    assert!(screen_text.contains("code cargo") || screen_text.contains("code"));
}

#[test]
fn test_confirm_modal_workflow() {
    let items = vec![SessionCandidate::new(
        "killme",
        "/path",
        SessionSource::Tmux,
    )];
    let mut app = App::new(items, None);

    assert!(app.confirm_modal.is_none());

    // Prompt kill
    assert!(app.prompt_kill_selected());
    let modal = app.confirm_modal.as_ref().unwrap();
    assert_eq!(modal.target, "killme");
    assert!(modal.yes);

    // Toggle to No
    app.confirm_modal_toggle();
    assert!(!app.confirm_modal.as_ref().unwrap().yes);

    // Accept while No returns None
    let accepted = app.confirm_modal_accept();
    assert_eq!(accepted, None);
    assert!(app.confirm_modal.is_none());

    // Prompt again, stay on Yes, accept
    app.prompt_kill_selected();
    let accepted = app.confirm_modal_accept();
    assert_eq!(accepted, Some("killme".to_owned()));
    assert!(app.confirm_modal.is_none());
}

#[test]
fn test_render_confirm_modal_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let items = vec![SessionCandidate::new(
        "target-app",
        "/path",
        SessionSource::Tmux,
    )];
    let mut app = App::new(items, None);
    app.prompt_kill_selected();

    terminal.draw(|f| render(f, &app)).unwrap();

    let buffer = terminal.backend().buffer();
    let screen_text: String = (0..24)
        .flat_map(|y| (0..80).filter_map(move |x| buffer.cell((x, y)).map(|c| c.symbol())))
        .collect();

    assert!(screen_text.contains("Confirm Action"));
    assert!(screen_text.contains("target-app"));
    assert!(screen_text.contains("Yes"));
    assert!(screen_text.contains("No"));
}
