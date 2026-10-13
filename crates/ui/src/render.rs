use crate::ansi::parse_ansi_to_text;
use crate::app::App;
use crate::layout::{
    compute_centered_rect, compute_horizontal_chunks, compute_popup_area, resolve_borders,
};
use crate::theme::{ResolvedTheme, parse_color};
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
    parse_color(color_name)
}

#[must_use]
pub fn format_windows_peek(names: &[String], max_width: usize) -> Option<String> {
    if names.is_empty() || max_width < 4 {
        return None;
    }

    let prefix = "  ";
    let mut shown = Vec::new();
    let mut used_len = prefix.len();

    for (i, name) in names.iter().enumerate() {
        let entry_len = if shown.is_empty() {
            name.len()
        } else {
            1 + name.len()
        };
        let remaining = names.len() - i - 1;
        let elision_len = if remaining > 0 {
            3 + remaining.to_string().len() + 1
        } else {
            0
        };

        if used_len + entry_len + elision_len <= max_width {
            shown.push(name.as_str());
            used_len += entry_len;
        } else {
            break;
        }
    }

    if shown.is_empty() {
        return None;
    }

    let mut result = String::with_capacity(max_width);
    result.push_str(prefix);
    result.push_str(&shown.join(" "));

    if shown.len() < names.len() {
        let remaining = names.len() - shown.len();
        let elision = format!(" (+{})", remaining);
        if result.len() + elision.len() <= max_width {
            result.push_str(&elision);
        }
    }

    Some(result)
}

pub fn render(frame: &mut Frame, app: &App) {
    let terminal_area = frame.area();
    let mut theme = ResolvedTheme::from_config(&app.tui.theme);

    if app.tui.border_color != "cyan" {
        theme.border = parse_color(&app.tui.border_color);
    }
    if app.tui.highlight_color != "yellow" {
        theme.item_selected_fg = parse_color(&app.tui.highlight_color);
    }

    let (popup_borders, popup_border_type) = resolve_borders(&app.tui.popup_border);

    let active_area = if app.tui.popup {
        let popup = compute_popup_area(terminal_area, app.tui.popup_width, app.tui.popup_height);
        frame.render_widget(Clear, popup);
        if popup_borders != Borders::NONE {
            let outer_block = Block::default()
                .borders(popup_borders)
                .border_type(popup_border_type)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(" Omega ", Style::default().fg(theme.title)));
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
    let search_line = Line::from(vec![
        Span::styled(
            app.tui.search_prompt.as_str(),
            Style::default().fg(theme.search_prompt),
        ),
        Span::styled(app.query.as_str(), Style::default().fg(theme.search_text)),
    ]);
    let search_bar = Paragraph::new(search_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(popup_border_type)
            .title(Span::styled(
                app.tui.search_title.as_str(),
                Style::default().fg(theme.title),
            ))
            .border_style(Style::default().fg(theme.border_focused)),
    );
    frame.render_widget(search_bar, search_chunk);

    // 2. Middle area: Horizontal split (List + Preview)
    let middle_chunks =
        compute_horizontal_chunks(app.preview_enabled, app.preview_width, middle_chunk);
    let list_chunk = middle_chunks.first().copied().unwrap_or(middle_chunk);

    let list_items: Vec<ListItem> = if app.filtered.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No matching workspaces found",
            Style::default().fg(theme.item_path),
        ))]
    } else {
        let mut items = Vec::with_capacity(app.filtered.len() + 4);
        let mut prev_source: Option<SessionSource> = None;

        for (i, &idx) in app.filtered.iter().enumerate() {
            let Some(item) = app.items.get(idx) else {
                continue;
            };

            if app.tui.group_separator
                && app.query.is_empty()
                && let Some(prev) = prev_source
                && prev != item.source
            {
                let sep_width = (list_chunk.width.saturating_sub(4) as usize).max(4);
                let sep_str = "─".repeat(sep_width);
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default().fg(theme.group_separator)),
                    Span::styled(sep_str, Style::default().fg(theme.group_separator)),
                ])));
            }
            prev_source = Some(item.source);

            let is_selected = i == app.selected;
            let mut spans = Vec::with_capacity(7);

            let prefix = if is_selected { "> " } else { "  " };
            spans.push(Span::styled(prefix, Style::default().fg(theme.cursor)));

            if app.tui.show_badges {
                let badge = match item.source {
                    SessionSource::Tmux => {
                        Span::styled(" [tmux]  ", Style::default().fg(theme.badge_tmux))
                    }
                    SessionSource::Window => {
                        Span::styled(" [win]   ", Style::default().fg(theme.badge_tmux))
                    }
                    SessionSource::Git => {
                        Span::styled(" [git]   ", Style::default().fg(theme.badge_git))
                    }
                    SessionSource::Alias => {
                        Span::styled(" [alias] ", Style::default().fg(theme.badge_alias))
                    }
                    SessionSource::Config => {
                        Span::styled(" [config]", Style::default().fg(theme.badge_config))
                    }
                    SessionSource::Zoxide => {
                        Span::styled(" [zoxide]", Style::default().fg(theme.badge_zoxide))
                    }
                    SessionSource::Custom => {
                        let label = item.custom_source.as_deref().unwrap_or("custom");
                        Span::styled(
                            format!(" [{label}] "),
                            Style::default().fg(theme.badge_custom),
                        )
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

            let mut name_style = if is_selected {
                Style::default()
                    .fg(theme.item_selected_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.item_unselected_fg)
            };
            if is_selected && let Some(bg) = theme.item_selected_bg {
                name_style = name_style.bg(bg);
            }
            spans.push(Span::styled(&item.name, name_style));

            if app.tui.show_windows && !item.window_names.is_empty() {
                let budget =
                    (list_chunk.width.saturating_sub(item.name.len() as u16 + 12) as usize).max(8);
                if let Some(win_str) = format_windows_peek(&item.window_names, budget) {
                    spans.push(Span::styled(
                        win_str,
                        Style::default().fg(theme.inline_windows),
                    ));
                }
            }

            if app.tui.show_path {
                spans.push(Span::styled(
                    format!("  ({})", item.path.display()),
                    Style::default().fg(theme.item_path),
                ));
            }

            items.push(ListItem::new(Line::from(spans)));
        }
        items
    };

    let (preview_borders, preview_border_type) = resolve_borders(&app.preview_border);
    let list_title = format!(" Workspaces ({}/{}) ", app.filtered.len(), app.items.len());

    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(popup_border_type)
            .title(Span::styled(list_title, Style::default().fg(theme.title)))
            .border_style(Style::default().fg(theme.border)),
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
                    .title(Span::styled(
                        preview_title,
                        Style::default().fg(theme.preview_title),
                    ))
                    .border_style(Style::default().fg(theme.preview_border)),
            );
        frame.render_widget(preview_widget, preview_chunk);
    }

    // 3. Footer
    if app.tui.show_footer && footer_chunk.height > 0 {
        let footer_spans = vec![
            Span::styled(" <Enter> ", Style::default().fg(theme.footer_key)),
            Span::styled("Connect | ", Style::default().fg(theme.footer_desc)),
            Span::styled("<Up>/<Down> ", Style::default().fg(theme.footer_key)),
            Span::styled("Navigate | ", Style::default().fg(theme.footer_desc)),
            Span::styled("<C-u>/<C-d> ", Style::default().fg(theme.footer_key)),
            Span::styled("Scroll | ", Style::default().fg(theme.footer_desc)),
            Span::styled("<C-p> ", Style::default().fg(theme.footer_key)),
            Span::styled("Preview | ", Style::default().fg(theme.footer_desc)),
            Span::styled("<Esc> ", Style::default().fg(theme.footer_key)),
            Span::styled("Exit", Style::default().fg(theme.footer_desc)),
        ];
        let footer = Paragraph::new(Line::from(footer_spans));
        frame.render_widget(footer, footer_chunk);
    }

    // 4. Centered Confirm Modal (if active)
    if let Some(modal) = &app.confirm_modal {
        let modal_area = compute_centered_rect(terminal_area, 54, 7);
        frame.render_widget(Clear, modal_area);

        let yes_style = if modal.yes {
            Style::default()
                .fg(theme.confirm_button_active_fg)
                .bg(theme.confirm_button_active_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.item_unselected_fg)
        };

        let no_style = if !modal.yes {
            Style::default()
                .fg(theme.confirm_button_active_fg)
                .bg(theme.confirm_button_active_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.item_unselected_fg)
        };

        let content = vec![
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(&modal.prompt, Style::default().fg(theme.title)),
            ]),
            Line::from(vec![
                Span::styled(" Target: ", Style::default().fg(theme.item_path)),
                Span::styled(
                    &modal.target,
                    Style::default()
                        .fg(theme.item_selected_fg)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::raw("        "),
                Span::styled(" [ Yes ] ", yes_style),
                Span::raw("    "),
                Span::styled(" [ No ] ", no_style),
            ]),
        ];

        let modal_block = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(popup_border_type)
                .border_style(Style::default().fg(theme.confirm_border))
                .title(Span::styled(
                    " Confirm Action ",
                    Style::default()
                        .fg(theme.title)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        frame.render_widget(modal_block, modal_area);
    }
}
