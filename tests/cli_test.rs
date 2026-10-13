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

#[test]
fn test_cli_parse_rename() {
    let args_one = vec!["omega", "rename", "new-name"];
    let cli_one = Cli::try_parse_from(args_one).unwrap();
    match cli_one.command {
        Some(Commands::Rename { first, second }) => {
            assert_eq!(first, "new-name");
            assert_eq!(second, None);
        }
        other => panic!("Unexpected command: {:?}", other),
    }

    let args_two = vec!["omega", "rename", "old-name", "new-name"];
    let cli_two = Cli::try_parse_from(args_two).unwrap();
    match cli_two.command {
        Some(Commands::Rename { first, second }) => {
            assert_eq!(first, "old-name");
            assert_eq!(second.as_deref(), Some("new-name"));
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_window() {
    let args = vec!["omega", "window", "editor"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Window { query }) => {
            assert_eq!(query, vec!["editor"]);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_scratch_default() {
    let args = vec!["omega", "scratch"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Scratch { name }) => {
            assert_eq!(name, None);
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parse_scratch_named() {
    let args = vec!["omega", "scratch", "--name", "notes"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Scratch { name }) => {
            assert_eq!(name.as_deref(), Some("notes"));
        }
        other => panic!("Unexpected command: {:?}", other),
    }
}
