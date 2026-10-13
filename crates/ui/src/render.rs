use crate::ansi::parse_ansi_to_text;
use crate::app::App;
use crate::layout::{compute_horizontal_chunks, compute_popup_area, resolve_borders};
use omega_session::SessionSource;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

#[must_use]
pub fn resolve_color(color_name: &str) -> Color {
    match color_name.trim().to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" | "darkgray" | "dark_gray" => Color::DarkGray,
        "white" => Color::White,
        _ => Color::Cyan,
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let terminal_area = frame.area();
    let border_color = resolve_color(&app.tui.border_color);
    let highlight_color = resolve_color(&app.tui.highlight_color);
    let (popup_borders, popup_border_type) = resolve_borders(&app.tui.popup_border);

    let active_area = if app.tui.popup {
        let popup = compute_popup_area(terminal_area, app.tui.popup_width, app.tui.popup_height);
        frame.render_widget(Clear, popup);
        if popup_borders != Borders::NONE {
            let outer_block = Block::default()
                .borders(popup_borders)
                .border_type(popup_border_type)
                .border_style(Style::default().fg(border_color))
                .title(" Omega ");
            let inner = outer_block.inner(popup);
            frame.render_widget(outer_block, popup);
            inner
        } else {
            popup
        }
    } else {
        terminal_area
    };

    let footer_constraint = if app.tui.show_footer {
        Constraint::Length(1)
    } else {
        Constraint::Length(0)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input
            Constraint::Min(5),    // Session List + Preview
            footer_constraint,     // Footer Hint
        ])
        .split(active_area);

    let search_chunk = chunks.first().copied().unwrap_or(active_area);
    let middle_chunk = chunks.get(1).copied().unwrap_or(active_area);
    let footer_chunk = chunks.get(2).copied().unwrap_or(active_area);

    // 1. Search Box
    let search_text = format!("{}{}", app.tui.search_prompt, app.query);
    let search_bar = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(popup_border_type)
                .title(app.tui.search_title.as_str())
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(search_bar, search_chunk);

    // 2. Middle area: Horizontal split (List + Preview)
    let middle_chunks =
        compute_horizontal_chunks(app.preview_enabled, app.preview_width, middle_chunk);
    let list_chunk = middle_chunks.first().copied().unwrap_or(middle_chunk);

    let list_items: Vec<ListItem> = if app.filtered.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No matching workspaces found",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        app.filtered
            .iter()
            .enumerate()
            .filter_map(|(i, &idx)| {
                let item = app.items.get(idx)?;
                let is_selected = i == app.selected;

                let mut spans = Vec::with_capacity(6);

                let prefix = if is_selected { "> " } else { "  " };
                spans.push(Span::raw(prefix));

                if app.tui.show_badges {
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
                    spans.push(badge);
                }

                if app.tui.show_icons
                    && let Some(icon) = &item.icon
                {
                    spans.push(Span::raw(icon.as_str()));
                    spans.push(Span::raw(" "));
                }

                let name_style = if is_selected {
                    Style::default()
                        .fg(highlight_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(&item.name, name_style));

                if app.tui.show_path {
                    spans.push(Span::styled(
                        format!("  ({})", item.path.display()),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                Some(ListItem::new(Line::from(spans)))
            })
            .collect()
    };

    let (preview_borders, preview_border_type) = resolve_borders(&app.preview_border);
    let list_title = format!(" Workspaces ({}/{}) ", app.filtered.len(), app.items.len());

    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(popup_border_type)
            .title(list_title)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(list_widget, list_chunk);

    // Render Preview Pane if horizontal split is active
    if let Some(&preview_chunk) = middle_chunks.get(1) {
        let preview_title = if let Some(item) = app.selected_item() {
            format!(" Preview: {} ", item.name)
        } else {
            " Preview ".to_owned()
        };
        let preview_text = parse_ansi_to_text(&app.preview_content);
        let preview_widget = Paragraph::new(preview_text)
            .scroll((app.preview_scroll, 0))
            .block(
                Block::default()
                    .borders(preview_borders)
                    .border_type(preview_border_type)
                    .title(preview_title)
                    .border_style(Style::default().fg(border_color)),
            );
        frame.render_widget(preview_widget, preview_chunk);
    }

    // 3. Footer
    if app.tui.show_footer && footer_chunk.height > 0 {
        let footer = Paragraph::new(
            " <Enter> Connect | <Up>/<Down> Navigate | <C-u>/<C-d> Scroll Preview | <C-p> Toggle Preview | <Esc> Exit",
        )
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, footer_chunk);
    }
}
