use omega::cli::Commands;
use omega::commands::execute_command_with_engine;
use omega_lua::LuaEngine;
use omega_tmux::MockTmuxClient;
use tempfile::tempdir;

#[test]
fn test_connect_provisions_layout_from_lua_config() {
    let mut tmux = MockTmuxClient::new();
    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");

    let lua_code = r#"
        local omega = require("omega")
        omega.setup({
            aliases = {
                {
                    name = "my-project",
                    path = "/tmp/my-project",
                    windows = {
                        {
                            name = "editor",
                            command = "nvim",
                        },
                        {
                            name = "server",
                            layout = "even-horizontal",
                            panes = {
                                { command = "cargo run" },
                            },
                        },
                    },
                },
            },
        })
        omega.on("session_created", function(session)
            _G.HOOK_CREATED = session.name
        end)
    "#;

    std::fs::write(&init_path, lua_code).unwrap();
    let engine = LuaEngine::new().unwrap();
    engine.load_file(&init_path).unwrap();
    let config = engine.config();

    let cmd = Commands::Connect {
        target: "my-project".to_owned(),
        switch: false,
        command: None,
    };

    execute_command_with_engine(cmd, &mut tmux, &config, Some(&engine)).unwrap();

    assert_eq!(tmux.created_sessions.len(), 1);
    assert_eq!(tmux.created_sessions[0].name, "my-project");
    assert_eq!(tmux.created_sessions[0].window_name, "editor");
    assert_eq!(tmux.created_windows.len(), 1);
    assert_eq!(tmux.created_windows[0].1, "server");
    assert_eq!(tmux.applied_layouts.len(), 1);
    assert_eq!(tmux.applied_layouts[0].1, "even-horizontal");

    let hook_val: String = engine.eval_global("HOOK_CREATED").unwrap();
    assert_eq!(hook_val, "my-project");
}

#[test]
fn test_connect_runs_wildcard_startup_command() {
    let mut tmux = MockTmuxClient::new();
    let config = omega_config::Config {
        wildcards: vec![omega_config::WildcardRule {
            pattern: "/tmp/rust/*".to_owned(),
            startup_command: Some("cargo check".to_owned()),
            icon: Some("🦀".to_owned()),
            windows: Vec::new(),
        }],
        ..Default::default()
    };

    let cmd = Commands::Connect {
        target: "/tmp/rust/myapp".to_owned(),
        switch: false,
        command: None,
    };

    execute_command_with_engine(cmd, &mut tmux, &config, None).unwrap();

    assert_eq!(tmux.created_sessions.len(), 1);
    assert_eq!(tmux.sent_keys.len(), 1);
    assert_eq!(tmux.sent_keys[0].1, "cargo check");
}

#[test]
fn test_harvest_and_sort_candidates_with_custom_source_and_naming() {
    use omega::commands::harvest_and_sort_candidates;
    use omega_session::SessionSource;

    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");

    let lua_code = r#"
        local omega = require("omega")
        omega.setup({
            sort_order = { "custom", "tmux" },
        })
        omega.register_source("special", function()
            return {
                { name = "custom:project", path = "/tmp/special-project", icon = "⚡" }
            }
        end)
        omega.session_name(function(path, git_ctx)
            return "hook:name.with:dots"
        end)
    "#;

    std::fs::write(&init_path, lua_code).unwrap();
    let engine = LuaEngine::new().unwrap();
    engine.load_file(&init_path).unwrap();
    let config = engine.config();

    let candidates = harvest_and_sort_candidates(&config, Some(&engine));

    let custom_cand = candidates
        .iter()
        .find(|c| c.source == SessionSource::Custom)
        .expect("custom candidate should be present");

    assert_eq!(custom_cand.name, "custom:project");
    assert_eq!(custom_cand.icon.as_deref(), Some("⚡"));

    // Any non-tmux, non-alias, non-custom candidate evaluated under the hook will be sanitized
    for cand in &candidates {
        if cand.source != SessionSource::Tmux
            && cand.source != SessionSource::Alias
            && cand.source != SessionSource::Custom
        {
            assert_eq!(cand.name, "hook_name_with_dots");
        }
    }
}
