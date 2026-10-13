use clap::Parser;
use omega::cli::{Cli, Commands, ShellTarget};
use omega::init::generate_init_script;
use std::process::Command;

#[test]
fn test_cli_parse_init() {
    let args_zsh = vec!["omega", "init", "zsh"];
    let cli_zsh = Cli::try_parse_from(args_zsh).unwrap();
    match cli_zsh.command {
        Some(Commands::Init { shell, tmux }) => {
            assert_eq!(shell, ShellTarget::Zsh);
            assert!(!tmux);
        }
        other => panic!("Unexpected command: {:?}", other),
    }

    let args_tmux_flag = vec!["omega", "init", "zsh", "--tmux"];
    let cli_tmux_flag = Cli::try_parse_from(args_tmux_flag).unwrap();
    match cli_tmux_flag.command {
        Some(Commands::Init { shell, tmux }) => {
            assert_eq!(shell, ShellTarget::Zsh);
            assert!(tmux);
        }
        other => panic!("Unexpected command: {:?}", other),
    }

    let args_tmux = vec!["omega", "init", "tmux"];
    let cli_tmux = Cli::try_parse_from(args_tmux).unwrap();
    match cli_tmux.command {
        Some(Commands::Init { shell, tmux }) => {
            assert_eq!(shell, ShellTarget::Tmux);
            assert!(!tmux);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_generate_init_script_zsh_syntax_and_eval() {
    let script = generate_init_script(ShellTarget::Zsh, false);
    assert!(script.contains("exec </dev/tty"), "Must allocate clean tty");
    assert!(script.contains("zle -N omega-widget"));
    assert!(script.contains("bindkey"));

    // Verify script evaluates cleanly in zsh without syntax errors
    let status = Command::new("zsh")
        .args(["-c", &format!("eval \"{script}\"")])
        .status();
    if let Ok(st) = status {
        assert!(
            st.success(),
            "zsh eval should succeed without syntax errors"
        );
    }
}

#[test]
fn test_generate_init_script_bash_syntax() {
    let script = generate_init_script(ShellTarget::Bash, false);
    assert!(script.contains("exec </dev/tty"));
    assert!(script.contains("bind -x"));

    let status = Command::new("bash").args(["-n", "-c", &script]).status();
    if let Ok(st) = status {
        assert!(st.success(), "bash -n syntax validation should succeed");
    }
}

#[test]
fn test_generate_init_script_fish() {
    let script = generate_init_script(ShellTarget::Fish, false);
    assert!(script.contains("</dev/tty"));
    assert!(script.contains("bind \\es"));
}

#[test]
fn test_generate_init_script_tmux_keybindings() {
    let script_tmux = generate_init_script(ShellTarget::Tmux, false);
    assert!(script_tmux.contains("bind-key -n M-s display-popup -E -w 85% -h 80% \"omega\""));
    assert!(
        script_tmux.contains("bind-key -n M-w display-popup -E -w 85% -h 80% \"omega window\"")
    );
    assert!(script_tmux.contains("bind-key L run-shell \"omega last\""));
    assert!(script_tmux.contains("bind-key 9 run-shell \"omega root\""));

    let script_zsh_with_tmux = generate_init_script(ShellTarget::Zsh, true);
    assert!(
        script_zsh_with_tmux.contains("bind-key -n M-s display-popup -E -w 85% -h 80% \"omega\"")
    );
    assert!(script_zsh_with_tmux.contains("exec </dev/tty"));
}
