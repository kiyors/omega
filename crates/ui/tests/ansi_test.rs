use omega_ui::ansi::parse_ansi_to_text;
use ratatui::style::{Color, Modifier};

#[test]
fn test_parse_plain_text() {
    let text = parse_ansi_to_text("Hello\nWorld");
    assert_eq!(text.lines.len(), 2);
    assert_eq!(
        text.lines.first().unwrap().spans.first().unwrap().content,
        "Hello"
    );
    assert_eq!(
        text.lines.get(1).unwrap().spans.first().unwrap().content,
        "World"
    );
}

#[test]
fn test_parse_ansi_colors_and_modifiers() {
    let raw = "\x1b[32mGreen\x1b[0m Normal \x1b[1;31mBoldRed\x1b[0m";
    let text = parse_ansi_to_text(raw);

    assert_eq!(text.lines.len(), 1);
    let spans = &text.lines.first().unwrap().spans;
    assert_eq!(spans.len(), 3);
    assert_eq!(spans.first().unwrap().content, "Green");
    assert_eq!(spans.first().unwrap().style.fg, Some(Color::Green));
    assert_eq!(spans.get(1).unwrap().content, " Normal ");
    assert_eq!(spans.get(2).unwrap().content, "BoldRed");
    assert_eq!(spans.get(2).unwrap().style.fg, Some(Color::Red));
    assert!(
        spans
            .get(2)
            .unwrap()
            .style
            .add_modifier
            .contains(Modifier::BOLD)
    );
}
