use omega::cli::Commands;
use omega::commands::execute_command_with_engine;
use omega_config::LuaEngine;
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
