use omega_config::{CustomCandidate, GitContext, SessionEventPayload};
use omega_lua::LuaEngine;
use std::path::{Path, PathBuf};

#[test]
fn test_lua_custom_naming_hook() {
    let engine = LuaEngine::new().expect("failed to init engine");
    let script = r#"
        local omega = require("omega")
        omega.session_name(function(path, git_context)
            if git_context then
                return git_context.repo .. "@" .. git_context.branch
            end
            if path == "/special" then
                return "special-session"
            end
            return nil
        end)
    "#;

    engine.load_string(script).expect("failed to load script");

    let git_ctx = GitContext {
        repo: "omega".to_owned(),
        branch: "feat/phase4".to_owned(),
    };
    let name_with_git = engine.call_session_name_hook(Path::new("/code/omega"), Some(&git_ctx));
    assert_eq!(name_with_git.as_deref(), Some("omega@feat/phase4"));

    let name_special = engine.call_session_name_hook(Path::new("/special"), None);
    assert_eq!(name_special.as_deref(), Some("special-session"));

    let name_fallback = engine.call_session_name_hook(Path::new("/other"), None);
    assert_eq!(name_fallback, None);
}

#[test]
fn test_lua_chainable_naming_hooks() {
    let engine = LuaEngine::new().expect("failed to init engine");
    let script = r#"
        local omega = require("omega")
        -- Hook 1 only handles /first
        omega.session_name(function(path, _)
            if path == "/first" then return "one" end
            return nil
        end)
        -- Hook 2 only handles /second
        omega.session_name(function(path, _)
            if path == "/second" then return "two" end
            return nil
        end)
    "#;
    engine.load_string(script).expect("failed to load script");

    assert_eq!(
        engine
            .call_session_name_hook(Path::new("/first"), None)
            .as_deref(),
        Some("one")
    );
    assert_eq!(
        engine
            .call_session_name_hook(Path::new("/second"), None)
            .as_deref(),
        Some("two")
    );
    assert_eq!(
        engine.call_session_name_hook(Path::new("/other"), None),
        None
    );
}

#[test]
fn test_lua_lifecycle_event_hook() {
    let engine = LuaEngine::new().expect("failed to init engine");
    let script = r#"
        local omega = require("omega")
        omega.on("session_created", function(payload)
            _G.LAST_CREATED = payload.name .. ":" .. payload.path
        end)
        omega.on("session_switched", function(payload)
            _G.LAST_SWITCHED = payload.name
        end)
    "#;

    engine.load_string(script).expect("failed to load script");

    engine
        .dispatch_hook(
            "session_created",
            &SessionEventPayload {
                name: "dev-session".to_owned(),
                path: PathBuf::from("/work/dev"),
            },
        )
        .expect("hook failed");

    let created: String = engine.eval_global("LAST_CREATED").unwrap();
    assert_eq!(created, "dev-session:/work/dev");

    engine
        .dispatch_hook(
            "session_switched",
            &SessionEventPayload {
                name: "other-session".to_owned(),
                path: PathBuf::from("/work/other"),
            },
        )
        .expect("hook failed");

    let switched: String = engine.eval_global("LAST_SWITCHED").unwrap();
    assert_eq!(switched, "other-session");
}

#[test]
fn test_lua_register_custom_source() {
    let engine = LuaEngine::new().expect("failed to init engine");
    let script = r#"
        local omega = require("omega")
        omega.register_source("docker", function()
            return {
                { name = "docker:redis", path = "/tmp/docker/redis", icon = "󰡨" },
                { name = "docker:postgres", path = "/tmp/docker/postgres" },
            }
        end)
    "#;

    engine.load_string(script).expect("failed to load script");

    let candidates: Vec<CustomCandidate> = engine
        .eval_custom_sources()
        .expect("failed to evaluate sources");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].source_name, "docker");
    assert_eq!(candidates[0].name, "docker:redis");
    assert_eq!(candidates[0].path, PathBuf::from("/tmp/docker/redis"));
    assert_eq!(candidates[0].icon.as_deref(), Some("󰡨"));

    assert_eq!(candidates[1].source_name, "docker");
    assert_eq!(candidates[1].name, "docker:postgres");
    assert_eq!(candidates[1].path, PathBuf::from("/tmp/docker/postgres"));
    assert_eq!(candidates[1].icon, None);
}
