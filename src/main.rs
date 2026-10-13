use clap::Parser;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use omega::app::App;
use omega::cli::Cli;
use omega::commands::{execute_command_with_engine, harvest_and_sort_candidates, smart_connect};
use omega::event::{Event, EventHandler};
use omega::ui;
use omega_session::{PreviewGenerator, SessionCandidate, SessionSource};
use omega_tmux::{RealTmuxClient, TmuxClient};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let (config, lua_engine) = omega_lua::load_config_and_engine();

    let mut tmux_client = RealTmuxClient;

    if let Some(cmd) = cli.command {
        return execute_command_with_engine(cmd, &mut tmux_client, &config, lua_engine.as_ref());
    }

    // Launch interactive TUI picker
    let sessions = harvest_and_sort_candidates(&config, lua_engine.as_ref());

    let initial_query = if cli.query.is_empty() {
        None
    } else {
        Some(cli.query.join(" "))
    };

    let mut app = App::new(sessions, initial_query);
    app.preview_enabled = config.tui.preview;
    app.preview_width = config.tui.preview_width;
    app.preview_border = config.tui.preview_border.clone();

    // Asynchronous debounced preview worker channel
    let (req_tx, req_rx) = std::sync::mpsc::channel::<Option<SessionCandidate>>();
    let (res_tx, res_rx) = std::sync::mpsc::channel::<(String, std::path::PathBuf, String)>();

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
                let _ = res_tx.send((cand.name, cand.path, preview));
            }
        }
    });

    let mut terminal = ratatui::init();
    let events = EventHandler::new(Duration::from_millis(50));
    let mut last_requested_target: Option<(String, std::path::PathBuf)> = None;

    let chosen_session = loop {
        // Drain any incoming preview updates
        while let Ok((target_name, target_path, preview)) = res_rx.try_recv() {
            if let Some(cur) = app.selected_item()
                && cur.name == target_name
                && cur.path == target_path
            {
                app.preview_content = preview;
            }
        }

        // Trigger preview request when selected candidate changes
        let current_target = app
            .selected_item()
            .map(|c| (c.name.as_str(), c.path.as_path()));
        let last_target_borrowed = last_requested_target
            .as_ref()
            .map(|(n, p)| (n.as_str(), p.as_path()));

        if current_target != last_target_borrowed {
            last_requested_target = app
                .selected_item()
                .map(|c| (c.name.clone(), c.path.clone()));
            app.preview_content.clear(); // Clear stale preview immediately
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
                // Check custom keymaps
                let key_str = match key.code {
                    KeyCode::Enter => Some("<CR>".to_owned()),
                    KeyCode::Esc => Some("<Esc>".to_owned()),
                    KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Some(format!("<C-{c}>"))
                    }
                    _ => None,
                };
                let kill_selected = |app: &mut App, tmux: &mut RealTmuxClient| {
                    if let Some(selected) = app.selected_item() {
                        if selected.source == SessionSource::Tmux {
                            let _ = tmux.kill_session(&selected.name);
                        }
                        app.remove_selected();
                    }
                };

                if let Some(k) = key_str
                    && let Some(action) = config.keymaps.get(&k)
                {
                    match action.as_str() {
                        "connect" => break app.selected_item().cloned(),
                        "kill_session" => {
                            kill_selected(&mut app, &mut tmux_client);
                            continue;
                        }
                        "select_next" => {
                            app.select_next();
                            continue;
                        }
                        "select_prev" => {
                            app.select_prev();
                            continue;
                        }
                        "preview_scroll_up" => {
                            app.scroll_preview_up();
                            continue;
                        }
                        "preview_scroll_down" => {
                            app.scroll_preview_down();
                            continue;
                        }
                        "toggle_preview" => {
                            app.toggle_preview();
                            continue;
                        }
                        _ => {}
                    }
                }

                // Default Navigation & Preview Keymaps
                match key.code {
                    KeyCode::Enter => break app.selected_item().cloned(),
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
                    KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        kill_selected(&mut app, &mut tmux_client);
                    }
                    KeyCode::Backspace => {
                        app.query.pop();
                        app.refilter();
                    }
                    KeyCode::Char(c)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
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
        smart_connect(
            &mut tmux_client,
            &target.name,
            &target.path,
            None,
            &config,
            lua_engine.as_ref(),
        )?;
    }

    Ok(())
}
