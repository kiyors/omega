use clap::Parser;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use omega::app::App;
use omega::cli::Cli;
use omega::commands::execute_command;
use omega::event::{Event, EventHandler};
use omega::ui;
use omega_config::Config;
use omega_session::{RealTmuxClient, SessionCollector, TmuxActionRunner};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config = Config::default();
    let mut tmux_client = RealTmuxClient;

    if let Some(cmd) = cli.command {
        return execute_command(cmd, &mut tmux_client, &config);
    }

    // Launch interactive TUI picker
    let collector = SessionCollector::new();
    let sessions = collector.collect_from_system(&config);

    let initial_query = if cli.query.is_empty() {
        None
    } else {
        Some(cli.query.join(" "))
    };

    let mut app = App::new(sessions, initial_query);

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
        let runner = TmuxActionRunner::new();
        runner.connect(&mut tmux_client, &target.name, &target.path, None)?;
    }

    Ok(())
}
