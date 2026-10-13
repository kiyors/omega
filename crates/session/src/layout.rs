use omega_config::{WildcardRule, WindowLayoutConfig};
use omega_tmux::TmuxClient;
use std::io;
use std::path::Path;

/// Matches a filesystem path against a slice of wildcard rules.
/// Supports `~` home expansion, `*` (single segment wildcard), and `**` (recursive wildcard).
pub fn match_wildcard<'a>(path: &Path, rules: &'a [WildcardRule]) -> Option<&'a WildcardRule> {
    let path_str = path.to_string_lossy();
    let home = std::env::var("HOME").ok();

    rules.iter().find(|rule| {
        let pattern = if let Some(stripped) = rule.pattern.strip_prefix("~/") {
            if let Some(h) = &home {
                format!("{}/{}", h, stripped)
            } else {
                rule.pattern.clone()
            }
        } else {
            rule.pattern.clone()
        };

        glob_match(&pattern, &path_str)
    })
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    // Check recursive wildcard suffix (e.g. `/work/**`)
    if pat_parts.last() == Some(&"**") {
        let prefix_len = pat_parts.len().saturating_sub(1);
        if path_parts.len() < prefix_len {
            return false;
        }
        return pat_parts[..prefix_len]
            .iter()
            .zip(path_parts[..prefix_len].iter())
            .all(|(&p, &t)| p == "*" || p == t);
    }

    if pat_parts.len() != path_parts.len() {
        return false;
    }

    pat_parts
        .iter()
        .zip(path_parts.iter())
        .all(|(&p, &t)| p == "*" || p == t)
}

/// Provisions a session with structured window and pane layouts using the given tmux client.
pub fn provision_session_layout<T: TmuxClient>(
    tmux: &mut T,
    session_name: &str,
    session_path: &Path,
    windows: &[WindowLayoutConfig],
) -> io::Result<()> {
    if windows.is_empty() {
        tmux.create_session(session_name, session_name, session_path, None)?;
        return Ok(());
    }

    for (i, win) in windows.iter().enumerate() {
        let (window_id, first_pane_id) = if i == 0 {
            let ids = tmux.create_session(session_name, &win.name, session_path, None)?;
            (ids.window_id, ids.pane_id)
        } else {
            let ids = tmux.create_window(session_name, &win.name, session_path)?;
            (ids.window_id, ids.pane_id)
        };

        if win.panes.is_empty() {
            if let Some(cmd) = &win.command {
                tmux.send_keys(&first_pane_id, cmd)?;
            }
        } else {
            if let Some(first_pane) = win.panes.first()
                && let Some(cmd) = &first_pane.command
            {
                tmux.send_keys(&first_pane_id, cmd)?;
            }

            for pane in win.panes.iter().skip(1) {
                let pane_path = pane.path.as_ref().map(Path::new).unwrap_or(session_path);
                let pane_id = tmux.split_pane(&window_id, pane_path)?;
                if let Some(cmd) = &pane.command {
                    tmux.send_keys(&pane_id, cmd)?;
                }
            }
        }

        if let Some(layout) = &win.layout {
            tmux.apply_layout(&window_id, layout)?;
        }
    }

    Ok(())
}
