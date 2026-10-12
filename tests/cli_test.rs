use clap::Parser;
use omega::cli::{Cli, Commands};

#[test]
fn test_cli_parse_connect() {
    let args = vec!["omega", "connect", "backend", "-c", "cargo watch"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Connect {
            target,
            command,
            switch,
        }) => {
            assert_eq!(target, "backend");
            assert_eq!(command.as_deref(), Some("cargo watch"));
            assert!(!switch);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_list_flags() {
    let args = vec!["omega", "list", "--tmux", "--json"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::List {
            tmux,
            git,
            zoxide,
            json,
            format,
        }) => {
            assert!(tmux);
            assert!(!git);
            assert!(!zoxide);
            assert!(json);
            assert_eq!(format, None);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_kill() {
    let args = vec!["omega", "kill", "old-sesh", "-f"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Kill { target, force }) => {
            assert_eq!(target, "old-sesh");
            assert!(force);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_last() {
    let args = vec!["omega", "last"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert_eq!(cli.command, Some(Commands::Last));
}

#[test]
fn test_cli_parse_root() {
    let args = vec!["omega", "root"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert_eq!(cli.command, Some(Commands::Root));
}

#[test]
fn test_cli_parse_mkdir() {
    let args = vec!["omega", "mkdir", "/tmp/new-proj", "-c", "nvim"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Mkdir { path, command }) => {
            assert_eq!(path.to_str().unwrap(), "/tmp/new-proj");
            assert_eq!(command.as_deref(), Some("nvim"));
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_default_query() {
    let args = vec!["omega", "my-search-term"];
    let cli = Cli::try_parse_from(args).unwrap();

    assert_eq!(cli.command, None);
    assert_eq!(cli.query, vec!["my-search-term"]);
}
