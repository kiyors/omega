use omega::ansi::parse_ansi_to_text;
use ratatui::style::{Color, Modifier, Style};

#[test]
fn test_parse_plain_text() {
    let text = parse_ansi_to_text("Hello\nWorld");
    assert_eq!(text.lines.len(), 2);
    assert_eq!(text.lines[0].spans[0].content, "Hello");
    assert_eq!(text.lines[1].spans[0].content, "World");
}

#[test]
fn test_parse_ansi_colors_and_modifiers() {
    // Green text, then reset, then bold red text
    let raw = "\x1b[32mGreen\x1b[0m Normal \x1b[1;31mBoldRed\x1b[0m";
    let text = parse_ansi_to_text(raw);

    assert_eq!(text.lines.len(), 1);
    let spans = &text.lines[0].spans;
    assert_eq!(spans.len(), 3);

    // Span 1: Green
    assert_eq!(spans[0].content, "Green");
    assert_eq!(spans[0].style.fg, Some(Color::Green));

    // Span 2: Normal
    assert_eq!(spans[1].content, " Normal ");
    assert_eq!(spans[1].style, Style::default());

    // Span 3: BoldRed
    assert_eq!(spans[2].content, "BoldRed");
    assert_eq!(spans[2].style.fg, Some(Color::Red));
    assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn test_parse_ansi_256_colors() {
    // 256 foreground color (e.g. 196 = bright red)
    let raw = "\x1b[38;5;196mColor256\x1b[0m";
    let text = parse_ansi_to_text(raw);

    assert_eq!(text.lines.len(), 1);
    let spans = &text.lines[0].spans;
    assert_eq!(spans[0].content, "Color256");
    assert_eq!(spans[0].style.fg, Some(Color::Indexed(196)));
}
