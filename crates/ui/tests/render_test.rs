use omega_session::{SessionCandidate, SessionSource};
use omega_ui::app::App;
use omega_ui::render::render;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn test_render_in_popup_mode() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let items = vec![
        SessionCandidate::new("my-session", "/projects/my-session", SessionSource::Tmux),
        SessionCandidate::new("dotfiles", "/home/user/.dotfiles", SessionSource::Alias),
    ];
    let mut app = App::new(items, None);
    app.popup_enabled = true;
    app.popup_width = 80;
    app.popup_height = 80;

    terminal.draw(|f| render(f, &app)).unwrap();

    let buffer = terminal.backend().buffer();

    // Row 0 should be empty because popup is centered vertically with margin
    let row0_str: String = (0..100)
        .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol()))
        .collect();
    assert_eq!(row0_str.trim(), "");

    // Inside popup area (e.g. y = 4), should contain search or border elements
    let row4_str: String = (0..100)
        .filter_map(|x| buffer.cell((x, 4)).map(|c| c.symbol()))
        .collect();
    assert!(row4_str.contains("Search") || row4_str.contains("─") || row4_str.contains("╭"));
}

#[test]
fn test_render_in_fullscreen_mode() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let items = vec![SessionCandidate::new("test", "/path", SessionSource::Git)];
    let mut app = App::new(items, None);
    app.popup_enabled = false;

    terminal.draw(|f| render(f, &app)).unwrap();

    let buffer = terminal.backend().buffer();

    // In fullscreen mode, row 0 has the top border of the search bar
    let row0_str: String = (0..100)
        .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol()))
        .collect();
    assert!(row0_str.contains("Search") || row0_str.contains("─") || row0_str.contains("╭"));
}
