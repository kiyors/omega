use omega_config::{WildcardRule, WindowLayoutConfig};
use omega_tmux::TmuxClient;
use std::io;
use std::path::Path;

/// Matches a filesystem path against a slice of wildcard rules.
/// Supports `~` home expansion, `*` (single segment wildcard), and `**` (recursive wildcard).
pub fn match_wildcard<'a>(path: &Path, rules: &'a [WildcardRule]) -> Option<&'a WildcardRule> {
    let home = std::env::var("HOME").ok();
    match_wildcard_with_home(path, rules, home.as_deref())
}

/// Matches a filesystem path against a slice of wildcard rules with a pre-resolved home directory.
pub fn match_wildcard_with_home<'a>(
    path: &Path,
    rules: &'a [WildcardRule],
    home: Option<&str>,
) -> Option<&'a WildcardRule> {
    let path_str = path.to_string_lossy();

    rules.iter().find(|rule| {
        if let Some(stripped_pat) = rule.pattern.strip_prefix("~/") {
            if let Some(home_dir) = home
                && let Some(rest) = path_str.strip_prefix(home_dir)
            {
                let rest = rest.strip_prefix('/').unwrap_or(rest);
                return glob_match(stripped_pat, rest);
            }
            return false;
        }

        glob_match(&rule.pattern, &path_str)
    })
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pat_trimmed = pattern.trim_matches('/');
    let path_trimmed = path.trim_matches('/');

    if let Some(prefix) = pat_trimmed.strip_suffix("/**") {
        let prefix_clean = prefix.trim_matches('/');
        let pat_iter = prefix_clean.split('/');
        let mut path_iter = path_trimmed.split('/');

        for p in pat_iter {
            match path_iter.next() {
                Some(t) if p == "*" || p == t => {}
                _ => return false,
            }
        }
        return true;
    }

    if pat_trimmed == "**" {
        return true;
    }

    let mut pat_iter = pat_trimmed.split('/');
    let mut path_iter = path_trimmed.split('/');

    loop {
        match (pat_iter.next(), path_iter.next()) {
            (Some(p), Some(t)) => {
                if p != "*" && p != t {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
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
