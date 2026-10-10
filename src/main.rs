mod app;
mod event;
mod ui;

use app::{App, SessionItem, SessionType};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use event::{Event, EventHandler};
use std::process::Command;
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Gather initial sessions (Tmux + Local directories)
    let sessions = discover_sessions();
    let mut app = App::new(sessions);

    let mut terminal = ratatui::init();
    let events = EventHandler::new(Duration::from_millis(250));

    let chosen_session = loop {
        terminal.draw(|f| ui::render(f, &app))?;

        match events.next()? {
            Event::Key(key) => {
                // Exit
                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break None;
                }
                // Enter / Select
                if key.code == KeyCode::Enter {
                    break app.selected_item().cloned();
                }
                // Navigation
                match key.code {
                    KeyCode::Up => app.select_prev(),
                    KeyCode::Down => app.select_next(),
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.select_prev()
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.select_next()
                    }
                    KeyCode::Backspace => {
                        app.query.pop();
                        app.refilter();
                    }
                    KeyCode::Char(c) => {
                        app.query.push(c);
                        app.refilter();
                    }
                    _ => {}
                }
            }
            Event::Tick => {}
            _ => {}
        }
    };

    // Restore terminal before switching tmux
    ratatui::restore();

    if let Some(target) = chosen_session {
        switch_or_attach(&target)?;
    }

    Ok(())
}

fn discover_sessions() -> Vec<SessionItem> {
    let mut items = Vec::new();

    // Query active tmux sessions
    if let Ok(output) = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}:#{session_path}"])
        .output()
    {
        if output.status.success() {
            let str_out = String::from_utf8_lossy(&output.stdout);
            for line in str_out.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if let [name, path] = parts.as_slice() {
                    items.push(SessionItem {
                        name: name.to_string(),
                        path: path.to_string(),
                        kind: SessionType::Tmux,
                    });
                }
            }
        }
    }
    items
}

fn switch_or_attach(item: &SessionItem) -> Result<()> {
    let in_tmux = std::env::var("TMUX").is_ok();

    if in_tmux {
        Command::new("tmux")
            .args(["switch-client", "-t", &item.name])
            .status()?;
    } else {
        Command::new("tmux")
            .args(["attach-session", "-t", &item.name])
            .status()?;
    }
    Ok(())
}
