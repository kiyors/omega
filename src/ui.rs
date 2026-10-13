use crate::ansi::parse_ansi_to_text;
use crate::app::App;
use omega_session::SessionSource;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
use std::rc::Rc;

pub const MIN_PREVIEW_WIDTH_THRESHOLD: u16 = 100;

pub fn should_render_preview(app: &App, area: Rect) -> bool {
    app.preview_enabled && area.width >= MIN_PREVIEW_WIDTH_THRESHOLD
}

pub fn compute_horizontal_chunks(app: &App, area: Rect) -> Rc<[Rect]> {
    if should_render_preview(app, area) {
        let preview_pct = app.preview_width.clamp(10, 90);
        let list_pct = 100 - preview_pct;
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(list_pct),
                Constraint::Percentage(preview_pct),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    }
}

pub fn resolve_border_type(border_str: &str) -> BorderType {
    match border_str {
        "line" | "plain" => BorderType::Plain,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input
            Constraint::Min(5),    // Session List + Preview
            Constraint::Length(1), // Footer Hint
        ])
        .split(frame.area());

    // 1. Search Box
    let search_bar = Paragraph::new(format!("> {}", app.query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Search / Switch Session (Prefixes: :t tmux, :g git, @ alias) "),
        )
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(search_bar, chunks[0]);

    // 2. Middle area: Horizontal split (List + Preview)
    let middle_chunks = compute_horizontal_chunks(app, chunks[1]);

    let list_items: Vec<ListItem> = if app.filtered.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No matching workspaces found",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        app.filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let item = &app.items[idx];
                let is_selected = i == app.selected;

                let badge = match item.source {
                    SessionSource::Tmux => {
                        Span::styled(" [tmux]  ", Style::default().fg(Color::Green))
                    }
                    SessionSource::Git => {
                        Span::styled(" [git]   ", Style::default().fg(Color::Blue))
                    }
                    SessionSource::Alias => {
                        Span::styled(" [alias] ", Style::default().fg(Color::Magenta))
                    }
                    SessionSource::Config => {
                        Span::styled(" [config]", Style::default().fg(Color::Cyan))
                    }
                    SessionSource::Zoxide => {
                        Span::styled(" [zoxide]", Style::default().fg(Color::Yellow))
                    }
                    SessionSource::Custom => {
                        let label = item.custom_source.as_deref().unwrap_or("custom");
                        Span::styled(format!(" [{label}] "), Style::default().fg(Color::Cyan))
                    }
                };

                let icon_span = if let Some(icon) = &item.icon {
                    Span::raw(format!("{} ", icon))
                } else {
                    Span::raw("")
                };

                let prefix = if is_selected { "> " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    badge,
                    icon_span,
                    Span::styled(&item.name, style),
                    Span::styled(
                        format!("  ({})", item.path.display()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    let border_type = resolve_border_type(&app.preview_border);
    let list_title = format!(" Workspaces ({}/{}) ", app.filtered.len(), app.items.len());

    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .title(list_title),
    );
    frame.render_widget(list_widget, middle_chunks[0]);

    // Render Preview Pane if active
    if middle_chunks.len() > 1 {
        let preview_title = if let Some(item) = app.selected_item() {
            format!(" Preview: {} ", item.name)
        } else {
            " Preview ".to_string()
        };
        let preview_text = parse_ansi_to_text(&app.preview_content);
        let preview_widget = Paragraph::new(preview_text)
            .scroll((app.preview_scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(border_type)
                    .title(preview_title),
            );
        frame.render_widget(preview_widget, middle_chunks[1]);
    }

    // 3. Footer
    let footer = Paragraph::new(
        " <Enter> Connect | <Up>/<Down> Navigate | <C-u>/<C-d> Scroll Preview | <C-p> Toggle Preview | <Esc> Exit",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}
