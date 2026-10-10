use crate::app::{App, SessionType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input
            Constraint::Min(5),    // Session List
            Constraint::Length(1), // Footer Hint
        ])
        .split(frame.area());

    // 1. Search Box
    let search_bar = Paragraph::new(format!("> {}", app.query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search / Switch Session "),
        )
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(search_bar, chunks[0]);

    // 2. Filtered List
    let list_items: Vec<ListItem> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let item = &app.items[idx];
            let is_selected = i == app.selected;

            let badge = match item.kind {
                SessionType::Tmux => Span::styled(" [tmux] ", Style::default().fg(Color::Green)),
                SessionType::Git => Span::styled(" [git]  ", Style::default().fg(Color::Blue)),
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
                Span::styled(&item.name, style),
                Span::styled(
                    format!("  ({})", item.path),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list_widget =
        List::new(list_items).block(Block::default().borders(Borders::ALL).title(" Sessions "));
    frame.render_widget(list_widget, chunks[1]);

    // 3. Footer
    let footer = Paragraph::new(" <Enter> Switch  |  <C-n>/<C-p> Move  |  <Esc> Exit")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}
