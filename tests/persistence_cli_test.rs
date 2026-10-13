use clap::Parser;
use omega::cli::{Cli, Commands, DaemonAction};

#[test]
fn test_cli_parse_save_without_name() {
    let cli = Cli::try_parse_from(["omega", "save"]).expect("parse save");
    match cli.command {
        Some(Commands::Save { name }) => assert_eq!(name, None),
        other => panic!("Unexpected command: {other:?}"),
    }
}

#[test]
fn test_cli_parse_save_with_name() {
    let cli =
        Cli::try_parse_from(["omega", "save", "--name", "standup"]).expect("parse save --name");
    match cli.command {
        Some(Commands::Save { name }) => assert_eq!(name.as_deref(), Some("standup")),
        other => panic!("Unexpected command: {other:?}"),
    }
}

#[test]
fn test_cli_parse_restore_full_flags() {
    let cli = Cli::try_parse_from(["omega", "restore", "omega", "--name", "standup", "-f"])
        .expect("parse restore");
    match cli.command {
        Some(Commands::Restore {
            session,
            name,
            overwrite,
        }) => {
            assert_eq!(session.as_deref(), Some("omega"));
            assert_eq!(name.as_deref(), Some("standup"));
            assert!(overwrite);
        }
        other => panic!("Unexpected command: {other:?}"),
    }
}

#[test]
fn test_cli_parse_restore_defaults() {
    let cli = Cli::try_parse_from(["omega", "restore"]).expect("parse restore");
    match cli.command {
        Some(Commands::Restore {
            session,
            name,
            overwrite,
        }) => {
            assert_eq!(session, None);
            assert_eq!(name, None);
            assert!(!overwrite);
        }
        other => panic!("Unexpected command: {other:?}"),
    }
}

#[test]
fn test_cli_parse_daemon_actions() {
    for (args, expected) in [
        (vec!["omega", "daemon", "start"], DaemonAction::Start),
        (vec!["omega", "daemon", "stop"], DaemonAction::Stop),
        (vec!["omega", "daemon", "status"], DaemonAction::Status),
    ] {
        let cli = Cli::try_parse_from(args).expect("parse daemon action");
        match cli.command {
            Some(Commands::Daemon { action }) => assert_eq!(action, expected),
            other => panic!("Unexpected command: {other:?}"),
        }
    }
}

#[test]
fn test_cli_parse_daemon_serve_is_hidden_but_available() {
    let cli = Cli::try_parse_from(["omega", "daemon", "serve"]).expect("parse daemon serve");
    match cli.command {
        Some(Commands::Daemon { action }) => assert_eq!(action, DaemonAction::Serve),
        other => panic!("Unexpected command: {other:?}"),
    }
}
