use clap::CommandFactory;
use clap_complete::Shell;
use omega::cli::{Cli, Commands};

#[test]
fn test_cli_parse_doctor_and_completions() {
    use clap::Parser;

    let cli_doctor = Cli::try_parse_from(["omega", "doctor"]).unwrap();
    assert_eq!(cli_doctor.command, Some(Commands::Doctor));

    let cli_comp_bash = Cli::try_parse_from(["omega", "completions", "bash"]).unwrap();
    assert_eq!(
        cli_comp_bash.command,
        Some(Commands::Completions { shell: Shell::Bash })
    );

    let cli_comp_zsh = Cli::try_parse_from(["omega", "completions", "zsh"]).unwrap();
    assert_eq!(
        cli_comp_zsh.command,
        Some(Commands::Completions { shell: Shell::Zsh })
    );

    let cli_comp_fish = Cli::try_parse_from(["omega", "completions", "fish"]).unwrap();
    assert_eq!(
        cli_comp_fish.command,
        Some(Commands::Completions { shell: Shell::Fish })
    );
}

#[test]
fn test_generate_completions_output() {
    let mut bash_buf = Vec::new();
    let mut cmd = Cli::command();
    clap_complete::generate(Shell::Bash, &mut cmd, "omega", &mut bash_buf);
    let bash_str = String::from_utf8(bash_buf).unwrap();
    assert!(
        bash_str.contains("omega"),
        "bash script should contain binary name"
    );

    let mut zsh_buf = Vec::new();
    let mut cmd = Cli::command();
    clap_complete::generate(Shell::Zsh, &mut cmd, "omega", &mut zsh_buf);
    let zsh_str = String::from_utf8(zsh_buf).unwrap();
    assert!(
        zsh_str.contains("#compdef omega"),
        "zsh script should declare compdef"
    );

    let mut fish_buf = Vec::new();
    let mut cmd = Cli::command();
    clap_complete::generate(Shell::Fish, &mut cmd, "omega", &mut fish_buf);
    let fish_str = String::from_utf8(fish_buf).unwrap();
    assert!(
        fish_str.contains("complete -c omega"),
        "fish script should declare complete"
    );
}
