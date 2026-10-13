use omega_config::LuaEngine;
use tempfile::tempdir;

#[test]
fn test_lua_engine_loads_declarative_setup() {
    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");

    let lua_code = r#"
        local omega = require("omega")
        omega.setup({
            hide_attached = false,
            dir_length = 2,
            tui = {
                preview = false,
                preview_width = 40,
                preview_border = "double",
            },
            icons = {
                tmux = "T",
                git = "G",
            },
            name_substitutions = {
                { find = "projects/", replace = "p-" },
            },
            aliases = {
                {
                    name = "backend",
                    path = "~/projects/api",
                    icon = "B",
                    windows = {
                        {
                            name = "server",
                            layout = "main-vertical",
                            panes = {
                                { command = "cargo run" },
                            },
                        },
                    },
                },
            },
            wildcards = {
                {
                    pattern = "~/projects/rust/*",
                    startup_command = "cargo check",
                    icon = "R",
                },
            },
            keymaps = {
                ["<C-j>"] = "select_next",
            },
        })
    "#;

    std::fs::write(&init_path, lua_code).unwrap();

    let engine = LuaEngine::new().expect("failed to init lua engine");
    engine
        .load_file(&init_path)
        .expect("failed to load init.lua");

    let config = engine.config();
    assert!(!config.hide_attached);
    assert_eq!(config.dir_length, 2);
    assert!(!config.tui.preview);
    assert_eq!(config.tui.preview_width, 40);
    assert_eq!(config.tui.preview_border, "double");
    assert_eq!(config.icons.get("tmux").map(|s| s.as_str()), Some("T"));
    assert_eq!(config.name_substitutions.len(), 1);
    assert_eq!(config.name_substitutions[0].find, "projects/");
    assert_eq!(config.name_substitutions[0].replace, "p-");

    assert_eq!(config.aliases.len(), 1);
    assert_eq!(config.aliases[0].name, "backend");
    assert_eq!(config.aliases[0].windows.len(), 1);
    assert_eq!(config.aliases[0].windows[0].name, "server");
    assert_eq!(
        config.aliases[0].windows[0].layout.as_deref(),
        Some("main-vertical")
    );
    assert_eq!(config.aliases[0].windows[0].panes.len(), 1);
    assert_eq!(
        config.aliases[0].windows[0].panes[0].command.as_deref(),
        Some("cargo run")
    );

    assert_eq!(config.wildcards.len(), 1);
    assert_eq!(config.wildcards[0].pattern, "~/projects/rust/*");
    assert_eq!(
        config.wildcards[0].startup_command.as_deref(),
        Some("cargo check")
    );

    assert_eq!(
        config.keymaps.get("<C-j>").map(|s| s.as_str()),
        Some("select_next")
    );
}

#[test]
fn test_lua_engine_syntax_error_reported() {
    let dir = tempdir().unwrap();
    let init_path = dir.path().join("init.lua");
    std::fs::write(&init_path, "local omega = ;").unwrap();

    let engine = LuaEngine::new().unwrap();
    let result = engine.load_file(&init_path);
    assert!(result.is_err(), "syntax error should fail load_file");
}

#[test]
fn test_lua_engine_package_path_and_plugins() {
    let dir = tempdir().unwrap();
    let lua_dir = dir.path().join("lua");
    let plugins_dir = dir.path().join("plugins");
    std::fs::create_dir_all(&lua_dir).unwrap();
    std::fs::create_dir_all(&plugins_dir).unwrap();

    // Custom module
    let helper_path = lua_dir.join("helper.lua");
    std::fs::write(&helper_path, "return { val = 42 }").unwrap();

    // Plugin
    let plugin_path = plugins_dir.join("my_plugin.lua");
    std::fs::write(
        &plugin_path,
        r#"
        local helper = require("helper")
        _G.PLUGIN_LOADED = helper.val
    "#,
    )
    .unwrap();

    // init.lua
    let init_path = dir.path().join("init.lua");
    std::fs::write(&init_path, "local _ = 1").unwrap();

    let engine = LuaEngine::new().unwrap();
    engine.configure_paths(dir.path()).unwrap();
    engine.load_plugins(dir.path()).unwrap();
    engine.load_file(&init_path).unwrap();

    let plugin_val: i32 = engine.eval_global("PLUGIN_LOADED").unwrap();
    assert_eq!(plugin_val, 42);
}
