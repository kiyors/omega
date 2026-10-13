//! Shell and multiplexer integration script generation.

use crate::cli::ShellTarget;

pub const TMUX_KEYBINDINGS: &str = r#"# Omega Tmux Keybindings
bind-key -n M-s display-popup -E -w 85% -h 80% "omega"
bind-key -n M-w display-popup -E -w 85% -h 80% "omega window"
bind-key -n M-o run-shell "omega scratch"
bind-key L run-shell "omega last"
bind-key 9 run-shell "omega root"
"#;

#[must_use]
pub fn generate_init_script(target: ShellTarget, include_tmux: bool) -> String {
    let mut script = String::new();

    match target {
        ShellTarget::Zsh => {
            script.push_str(
                r#"# Omega Zsh Integration
omega-widget() {
  omega </dev/tty
  zle && zle reset-prompt
}
zle -N omega-widget
bindkey '\es' omega-widget
bindkey '^[s' omega-widget

omega-window-widget() {
  omega window </dev/tty
  zle && zle reset-prompt
}
zle -N omega-window-widget
bindkey '\ew' omega-window-widget
bindkey '^[w' omega-window-widget
"#,
            );
        }
        ShellTarget::Bash => {
            script.push_str(
                r#"# Omega Bash Integration
_omega_bind() {
  omega </dev/tty
}
bind -x '"\es": _omega_bind'
bind -x '"\e[s": _omega_bind'

_omega_window_bind() {
  omega window </dev/tty
}
bind -x '"\ew": _omega_window_bind'
bind -x '"\e[w": _omega_window_bind'
"#,
            );
        }
        ShellTarget::Fish => {
            script.push_str(
                r#"# Omega Fish Integration
function __omega_run
  omega </dev/tty
  commandline -f repaint
end
bind \es __omega_run

function __omega_window_run
  omega window </dev/tty
  commandline -f repaint
end
bind \ew __omega_window_run
"#,
            );
        }
        ShellTarget::Tmux => {
            script.push_str(TMUX_KEYBINDINGS);
        }
    }

    if include_tmux && target != ShellTarget::Tmux {
        if !script.is_empty() {
            script.push('\n');
        }
        script.push_str(TMUX_KEYBINDINGS);
    }

    script
}
