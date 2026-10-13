use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// Parses raw text containing ANSI SGR escape codes into Ratatui `Text` using borrowed string slices.
#[must_use]
pub fn parse_ansi_to_text(input: &str) -> Text<'_> {
    let mut lines = Vec::new();
    let mut current_style = Style::default();

    for line_str in input.lines() {
        let mut spans = Vec::new();
        let bytes = line_str.as_bytes();
        let mut i = 0;
        let mut chunk_start = 0;

        while i < bytes.len() {
            if bytes.get(i).copied() == Some(0x1b)
                && bytes.get(i.saturating_add(1)).copied() == Some(b'[')
            {
                // Emit preceding text chunk if any
                if chunk_start < i
                    && let Some(text) = line_str.get(chunk_start..i)
                    && !text.is_empty()
                {
                    spans.push(Span::styled(text, current_style));
                }

                // Look for sequence terminator (usually 'm' for SGR, or other command letter)
                let seq_start = i.saturating_add(2);
                let mut seq_end = seq_start;
                while seq_end < bytes.len()
                    && !bytes.get(seq_end).is_some_and(u8::is_ascii_alphabetic)
                {
                    seq_end = seq_end.saturating_add(1);
                }

                if seq_end < bytes.len() {
                    let cmd = bytes.get(seq_end).copied();
                    if cmd == Some(b'm')
                        && let Some(params_str) = line_str.get(seq_start..seq_end)
                    {
                        current_style = update_sgr_style(current_style, params_str);
                    }
                    i = seq_end.saturating_add(1);
                    chunk_start = i;
                } else {
                    // Unterminated sequence: skip ESC and continue
                    i = i.saturating_add(1);
                    chunk_start = i;
                }
            } else {
                i = i.saturating_add(1);
            }
        }

        if chunk_start < bytes.len()
            && let Some(text) = line_str.get(chunk_start..)
            && !text.is_empty()
        {
            spans.push(Span::styled(text, current_style));
        }

        if spans.is_empty() {
            lines.push(Line::raw(""));
        } else {
            lines.push(Line::from(spans));
        }
    }

    Text::from(lines)
}

fn update_sgr_style(mut style: Style, params_str: &str) -> Style {
    if params_str.is_empty() {
        return Style::default();
    }

    let codes: Vec<u8> = params_str
        .split(';')
        .filter_map(|p| p.parse::<u8>().ok())
        .collect();

    let mut idx = 0;
    while idx < codes.len() {
        let code = codes.get(idx).copied().unwrap_or_default();
        match code {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            9 => style = style.add_modifier(Modifier::CROSSED_OUT),

            // SGR Off resets
            22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style = style.remove_modifier(Modifier::ITALIC),
            24 => style = style.remove_modifier(Modifier::UNDERLINED),
            27 => style = style.remove_modifier(Modifier::REVERSED),
            29 => style = style.remove_modifier(Modifier::CROSSED_OUT),

            // Standard FG
            30 => style = style.fg(Color::Black),
            31 => style = style.fg(Color::Red),
            32 => style = style.fg(Color::Green),
            33 => style = style.fg(Color::Yellow),
            34 => style = style.fg(Color::Blue),
            35 => style = style.fg(Color::Magenta),
            36 => style = style.fg(Color::Cyan),
            37 => style = style.fg(Color::White),
            38 => {
                // Extended FG: 38;5;n or 38;2;r;g;b
                if idx.saturating_add(2) < codes.len()
                    && codes.get(idx.saturating_add(1)).copied() == Some(5)
                {
                    if let Some(&n) = codes.get(idx.saturating_add(2)) {
                        style = style.fg(Color::Indexed(n));
                    }
                    idx = idx.saturating_add(2);
                } else if idx.saturating_add(4) < codes.len()
                    && codes.get(idx.saturating_add(1)).copied() == Some(2)
                {
                    if let (Some(&r), Some(&g), Some(&b)) = (
                        codes.get(idx.saturating_add(2)),
                        codes.get(idx.saturating_add(3)),
                        codes.get(idx.saturating_add(4)),
                    ) {
                        style = style.fg(Color::Rgb(r, g, b));
                    }
                    idx = idx.saturating_add(4);
                }
            }
            39 => style.fg = None,

            // Standard BG
            40 => style = style.bg(Color::Black),
            41 => style = style.bg(Color::Red),
            42 => style = style.bg(Color::Green),
            43 => style = style.bg(Color::Yellow),
            44 => style = style.bg(Color::Blue),
            45 => style = style.bg(Color::Magenta),
            46 => style = style.bg(Color::Cyan),
            47 => style = style.bg(Color::White),
            48 => {
                // Extended BG: 48;5;n or 48;2;r;g;b
                if idx.saturating_add(2) < codes.len()
                    && codes.get(idx.saturating_add(1)).copied() == Some(5)
                {
                    if let Some(&n) = codes.get(idx.saturating_add(2)) {
                        style = style.bg(Color::Indexed(n));
                    }
                    idx = idx.saturating_add(2);
                } else if idx.saturating_add(4) < codes.len()
                    && codes.get(idx.saturating_add(1)).copied() == Some(2)
                {
                    if let (Some(&r), Some(&g), Some(&b)) = (
                        codes.get(idx.saturating_add(2)),
                        codes.get(idx.saturating_add(3)),
                        codes.get(idx.saturating_add(4)),
                    ) {
                        style = style.bg(Color::Rgb(r, g, b));
                    }
                    idx = idx.saturating_add(4);
                }
            }
            49 => style.bg = None,

            // Bright FG
            90 => style = style.fg(Color::DarkGray),
            91 => style = style.fg(Color::LightRed),
            92 => style = style.fg(Color::LightGreen),
            93 => style = style.fg(Color::LightYellow),
            94 => style = style.fg(Color::LightBlue),
            95 => style = style.fg(Color::LightMagenta),
            96 => style = style.fg(Color::LightCyan),
            97 => style = style.fg(Color::White),

            // Bright BG
            100 => style = style.bg(Color::DarkGray),
            101 => style = style.bg(Color::LightRed),
            102 => style = style.bg(Color::LightGreen),
            103 => style = style.bg(Color::LightYellow),
            104 => style = style.bg(Color::LightBlue),
            105 => style = style.bg(Color::LightMagenta),
            106 => style = style.bg(Color::LightCyan),
            107 => style = style.bg(Color::White),

            _ => {}
        }
        idx = idx.saturating_add(1);
    }

    style
}
