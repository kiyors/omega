use clap::Parser;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use omega::app::App;
use omega::cli::Cli;
use omega::commands::execute_command;
use omega::event::{Event, EventHandler};
use omega::ui;
use omega_config::Config;
use omega_session::{
    PreviewGenerator, RealTmuxClient, SessionCandidate, SessionCollector, TmuxActionRunner,
};
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
    app.preview_enabled = config.tui.preview;
    app.preview_width = config.tui.preview_width;

    // Asynchronous debounced preview worker channel
    let (req_tx, req_rx) = std::sync::mpsc::channel::<Option<SessionCandidate>>();
    let (res_tx, res_rx) = std::sync::mpsc::channel::<(String, String)>();

    std::thread::spawn(move || {
        let generator = PreviewGenerator::new();
        let tmux = RealTmuxClient;

        while let Ok(mut candidate_opt) = req_rx.recv() {
            // Debounce: sleep 50ms, drain newer rapid navigation requests
            std::thread::sleep(Duration::from_millis(50));
            while let Ok(newer) = req_rx.try_recv() {
                candidate_opt = newer;
            }

            if let Some(cand) = candidate_opt {
                let preview = generator.generate(&cand, &tmux);
                let _ = res_tx.send((cand.name, preview));
            }
        }
    });

    let mut terminal = ratatui::init();
    let events = EventHandler::new(Duration::from_millis(50));
    let mut last_requested_name: Option<String> = None;

    let chosen_session = loop {
        // Drain any incoming preview updates
        while let Ok((target_name, preview)) = res_rx.try_recv() {
            if let Some(cur) = app.selected_item()
                && cur.name == target_name
            {
                app.preview_content = preview;
            }
        }

        // Trigger preview request when selected candidate changes
        let current_name = app.selected_item().map(|c| c.name.clone());
        if current_name != last_requested_name {
            last_requested_name = current_name;
            let _ = req_tx.send(app.selected_item().cloned());
        }

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

                // Navigation & Preview Keymaps
                match key.code {
                    KeyCode::Up => app.select_prev(),
                    KeyCode::Down => app.select_next(),
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.select_prev();
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.select_next();
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.select_next();
                    }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.toggle_preview();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.scroll_preview_up();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.scroll_preview_down();
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
