use omega::doctor::{DoctorEnv, DoctorStatus, run_doctor};
use std::collections::HashMap;
use std::path::Path;
use tempfile::tempdir;

struct MockDoctorEnv {
    commands: HashMap<String, Option<String>>,
    socket_running: bool,
}

impl DoctorEnv for MockDoctorEnv {
    fn command_output(&self, cmd: &str, _args: &[&str]) -> Option<String> {
        self.commands.get(cmd).cloned().flatten()
    }

    fn check_socket(&self, _socket_path: &Path) -> bool {
        self.socket_running
    }
}

#[test]
fn test_doctor_reports_all_healthy_when_tools_and_valid_config_present() {
    let mut commands = HashMap::new();
    commands.insert("tmux".to_owned(), Some("tmux 3.4".to_owned()));
    commands.insert("git".to_owned(), Some("git version 2.44.0".to_owned()));
    commands.insert("zoxide".to_owned(), Some("zoxide 0.9.4".to_owned()));

    let env = MockDoctorEnv {
        commands,
        socket_running: true,
    };

    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");
    std::fs::write(
        &init_path,
        r#"
        local omega = require("omega")
        omega.setup({ dir_length = 2 })
    "#,
    )
    .unwrap();

    let report = run_doctor(&env, Some(dir.path()));
    assert!(report.all_ok, "all checks should be healthy");
    assert!(
        report
            .items
            .iter()
            .any(|i| i.name == "tmux" && matches!(i.status, DoctorStatus::Ok))
    );
    assert!(
        report
            .items
            .iter()
            .any(|i| i.name == "git" && matches!(i.status, DoctorStatus::Ok))
    );
    assert!(
        report
            .items
            .iter()
            .any(|i| i.name == "zoxide" && matches!(i.status, DoctorStatus::Ok))
    );
    assert!(
        report
            .items
            .iter()
            .any(|i| i.name == "daemon" && matches!(i.status, DoctorStatus::Ok))
    );
    assert!(
        report
            .items
            .iter()
            .any(|i| i.name == "config" && matches!(i.status, DoctorStatus::Ok))
    );
}

#[test]
fn test_doctor_flags_missing_tmux_and_syntax_error_in_config() {
    let mut commands = HashMap::new();
    commands.insert("tmux".to_owned(), None); // Missing tmux
    commands.insert("git".to_owned(), Some("git version 2.44.0".to_owned()));
    commands.insert("zoxide".to_owned(), None); // Missing zoxide (optional/warning)

    let env = MockDoctorEnv {
        commands,
        socket_running: false, // Daemon stopped (warning)
    };

    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");
    std::fs::write(&init_path, "local broken = ;").unwrap();

    let report = run_doctor(&env, Some(dir.path()));
    assert!(!report.all_ok, "should flag errors");

    let tmux_item = report.items.iter().find(|i| i.name == "tmux").unwrap();
    assert!(matches!(tmux_item.status, DoctorStatus::Error));

    let config_item = report.items.iter().find(|i| i.name == "config").unwrap();
    assert!(matches!(config_item.status, DoctorStatus::Error));

    let zoxide_item = report.items.iter().find(|i| i.name == "zoxide").unwrap();
    assert!(matches!(zoxide_item.status, DoctorStatus::Warning));
}

#[test]
fn test_doctor_validates_config_json_when_lua_absent() {
    let mut commands = HashMap::new();
    commands.insert("tmux".to_owned(), Some("tmux 3.4".to_owned()));
    commands.insert("git".to_owned(), Some("git version 2.44.0".to_owned()));

    let env = MockDoctorEnv {
        commands,
        socket_running: true,
    };

    let dir = tempdir().unwrap();
    let json_path = dir.path().join("config.json");
    std::fs::write(
        &json_path,
        r#"{"aliases": [{"name": "api", "path": "/tmp/api"}]}"#,
    )
    .unwrap();

    let report = run_doctor(&env, Some(dir.path()));
    assert!(report.all_ok);

    let config_item = report.items.iter().find(|i| i.name == "config").unwrap();
    assert!(matches!(config_item.status, DoctorStatus::Ok));
    assert!(config_item.message.contains("Valid config.json"));
}
