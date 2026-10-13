use mlua::RegistryKey;
use mlua::prelude::*;
use omega_config::{Config, CustomCandidate, GitContext, SessionEventPayload};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type HookMap = HashMap<String, Vec<Arc<RegistryKey>>>;
type SourceList = Vec<(String, Arc<RegistryKey>)>;

struct LuaState {
    config: Config,
    naming_hooks: Vec<Arc<RegistryKey>>,
    hooks: HookMap,
    sources: SourceList,
    diagnostics: Vec<String>,
}

/// Embedded Lua 5.4 runtime host for Omega.
#[derive(Clone)]
pub struct LuaEngine {
    lua: Arc<Mutex<Lua>>,
    state: Arc<Mutex<LuaState>>,
}

impl LuaEngine {
    /// Initializes a new Lua 5.4 runtime with the `omega` global and `package.loaded["omega"]` table.
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let state = Arc::new(Mutex::new(LuaState {
            config: Config::default(),
            naming_hooks: Vec::new(),
            hooks: HashMap::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
        }));

        let omega = lua.create_table()?;

        // omega.version
        omega.set("version", env!("CARGO_PKG_VERSION"))?;

        // omega.log.{info, warn, error}
        let log_tbl = lua.create_table()?;
        let log_info = lua.create_function(|_lua, msg: String| {
            eprintln!("[omega:info] {msg}");
            Ok(())
        })?;
        let log_warn = lua.create_function(|_lua, msg: String| {
            eprintln!("[omega:warn] {msg}");
            Ok(())
        })?;
        let log_error = lua.create_function(|_lua, msg: String| {
            eprintln!("[omega:error] {msg}");
            Ok(())
        })?;
        log_tbl.set("info", log_info)?;
        log_tbl.set("warn", log_warn)?;
        log_tbl.set("error", log_error)?;
        omega.set("log", log_tbl)?;

        // omega.setup({ ... })
        let st_setup = Arc::clone(&state);
        let setup_fn = lua.create_function(move |lua, value: LuaValue| {
            let parsed_config: Config = lua.from_value(value)?;
            if let Ok(mut st) = st_setup.lock() {
                st.config = parsed_config;
            }
            Ok(())
        })?;
        omega.set("setup", setup_fn)?;

        // omega.ui table with setup, set, get APIs
        let ui_tbl = lua.create_table()?;
        let st_ui = Arc::clone(&state);
        let ui_setup_fn = lua.create_function(move |lua, value: LuaValue| {
            if let Ok(mut st) = st_ui.lock() {
                let current_json = serde_json::to_value(&st.config.tui)
                    .map_err(|e| mlua::Error::runtime(format!("{e}")))?;
                let incoming_json: serde_json::Value = lua.from_value(value)?;
                let merged_json = match (current_json, incoming_json) {
                    (serde_json::Value::Object(mut base), serde_json::Value::Object(patch)) => {
                        for (k, v) in patch {
                            base.insert(k, v);
                        }
                        serde_json::Value::Object(base)
                    }
                    (_, other) => other,
                };
                let updated_tui: omega_config::TuiConfig = serde_json::from_value(merged_json)
                    .map_err(|e| mlua::Error::runtime(format!("{e}")))?;
                st.config.tui = updated_tui;
            }
            Ok(())
        })?;

        let st_ui_get = Arc::clone(&state);
        let ui_get_fn = lua.create_function(move |lua, ()| {
            let current = st_ui_get
                .lock()
                .map(|st| st.config.tui.clone())
                .unwrap_or_default();
            lua.to_value(&current)
        })?;
        ui_tbl.set("setup", ui_setup_fn.clone())?;
        ui_tbl.set("set", ui_setup_fn)?;
        ui_tbl.set("get", ui_get_fn)?;
        omega.set("ui", ui_tbl)?;

        // omega.session_name(function) — chainable hooks
        let st_naming = Arc::clone(&state);
        let session_name_fn = lua.create_function(move |lua, func: LuaFunction| {
            let key = Arc::new(lua.create_registry_value(func)?);
            if let Ok(mut st) = st_naming.lock() {
                st.naming_hooks.push(key);
            }
            Ok(())
        })?;
        omega.set("session_name", session_name_fn)?;

        // omega.on(event, function)
        let st_hooks = Arc::clone(&state);
        let on_fn = lua.create_function(move |lua, (event, func): (String, LuaFunction)| {
            let key = Arc::new(lua.create_registry_value(func)?);
            if let Ok(mut st) = st_hooks.lock() {
                st.hooks.entry(event).or_default().push(key);
            }
            Ok(())
        })?;
        omega.set("on", on_fn)?;

        // omega.register_source(name, function)
        let st_sources = Arc::clone(&state);
        let register_src_fn =
            lua.create_function(move |lua, (name, func): (String, LuaFunction)| {
                let key = Arc::new(lua.create_registry_value(func)?);
                if let Ok(mut st) = st_sources.lock() {
                    st.sources.push((name, key));
                }
                Ok(())
            })?;
        omega.set("register_source", register_src_fn)?;

        // Expose to globals and require("omega")
        lua.globals().set("omega", omega.clone())?;
        let package: LuaTable = lua.globals().get("package")?;
        let loaded: LuaTable = package.get("loaded")?;
        loaded.set("omega", omega)?;

        Ok(Self {
            lua: Arc::new(Mutex::new(lua)),
            state,
        })
    }

    /// Bootstraps a Lua engine from a configuration directory:
    /// - Adds `<config_dir>/lua/?.lua` and `<config_dir>/lua/?/init.lua` to `package.path`
    /// - Executes all plugin scripts in `<config_dir>/plugins/*.lua`
    /// - Loads `<config_dir>/init.lua` if it exists
    pub fn from_config_dir(config_dir: &Path) -> LuaResult<Self> {
        let engine = Self::new()?;
        let _ = engine.configure_paths(config_dir);
        let _ = engine.load_plugins(config_dir);
        let init_lua = config_dir.join("init.lua");
        if init_lua.exists() {
            engine.load_file(&init_lua)?;
        }
        Ok(engine)
    }

    /// Access the current parsed configuration snapshot.
    pub fn config(&self) -> Config {
        self.state
            .lock()
            .map(|st| st.config.clone())
            .unwrap_or_default()
    }

    /// Returns recorded diagnostics and non-fatal plugin warnings.
    pub fn diagnostics(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|st| st.diagnostics.clone())
            .unwrap_or_default()
    }

    /// Configures `package.path` so that user modules in `config_dir/lua/` can be required.
    pub fn configure_paths(&self, config_dir: &Path) -> LuaResult<()> {
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        let lua_dir = config_dir.join("lua");
        let path_str = lua_dir.to_string_lossy();
        let additions = format!("{}/?.lua;{}/?/init.lua;", path_str, path_str);

        let package: LuaTable = lua.globals().get("package")?;
        let current_path: String = package.get("path")?;
        let new_path = format!("{}{}", additions, current_path);
        package.set("path", new_path)?;

        Ok(())
    }

    /// Executes all plugin scripts (*.lua) in `config_dir/plugins/`.
    /// Catches and records errors per plugin file so a failing script does not prevent other plugins from running.
    pub fn load_plugins(&self, config_dir: &Path) -> LuaResult<()> {
        let plugins_dir = config_dir.join("plugins");
        if !plugins_dir.is_dir() {
            return Ok(());
        }

        let mut entries: Vec<_> = match fs::read_dir(&plugins_dir) {
            Ok(iter) => iter.filter_map(std::result::Result::ok).collect(),
            Err(_) => return Ok(()),
        };
        // Sort entries deterministically by filename
        entries.sort_by_key(std::fs::DirEntry::file_name);

        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;

        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let code = fs::read_to_string(&path).map_err(LuaError::external)?;
                let res = lua.load(&code).set_name(path.to_string_lossy()).exec();
                if let Err(e) = res {
                    let msg = format!("Plugin error in {}: {e}", path.display());
                    eprintln!("[omega:warn] {msg}");
                    if let Ok(mut st) = self.state.lock() {
                        st.diagnostics.push(msg);
                    }
                }
            }
        }

        Ok(())
    }

    /// Executes a Lua script file.
    pub fn load_file(&self, path: &Path) -> LuaResult<()> {
        let code = fs::read_to_string(path).map_err(LuaError::external)?;
        self.load_string_named(&code, &path.to_string_lossy())
    }

    /// Executes a raw Lua code chunk.
    pub fn load_string(&self, code: &str) -> LuaResult<()> {
        self.load_string_named(code, "chunk")
    }

    /// Executes a named raw Lua code chunk (useful for stacktraces).
    pub fn load_string_named(&self, code: &str, chunk_name: &str) -> LuaResult<()> {
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        lua.load(code).set_name(chunk_name).exec()
    }

    /// Evaluates a global Lua variable by name (used for unit testing assertions).
    pub fn eval_global<T: FromLua>(&self, name: &str) -> LuaResult<T> {
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        lua.globals().get(name)
    }

    /// Invokes custom session name hooks in registration order, returning the first non-empty string.
    pub fn call_session_name_hook(
        &self,
        path: &Path,
        git_context: Option<&GitContext>,
    ) -> Option<String> {
        let keys = {
            let st = self.state.lock().ok()?;
            st.naming_hooks.clone()
        };

        if keys.is_empty() {
            return None;
        }

        let lua = self.lua.lock().ok()?;
        let path_str = path.to_string_lossy();

        for key in keys {
            if let Ok(hook) = lua.registry_value::<LuaFunction>(&key) {
                let res: LuaResult<LuaValue> = if let Some(ctx) = git_context {
                    if let Ok(git_table) = lua.create_table() {
                        let _ = git_table.set("repo", ctx.repo.as_str());
                        let _ = git_table.set("branch", ctx.branch.as_str());
                        hook.call((path_str.as_ref(), git_table))
                    } else {
                        hook.call((path_str.as_ref(), LuaValue::Nil))
                    }
                } else {
                    hook.call((path_str.as_ref(), LuaValue::Nil))
                };

                if let Ok(LuaValue::String(s)) = res
                    && let Ok(name) = s.to_str()
                    && !name.trim().is_empty()
                {
                    return Some(name.to_owned());
                }
            }
        }

        None
    }

    /// Dispatches a lifecycle event to registered event hooks (e.g. `session_created`, `session_switched`).
    pub fn dispatch_hook(&self, event: &str, payload: &SessionEventPayload) -> LuaResult<()> {
        let keys = {
            let st = self
                .state
                .lock()
                .map_err(|e| LuaError::external(format!("{e}")))?;
            st.hooks.get(event).cloned().unwrap_or_default()
        };

        if keys.is_empty() {
            return Ok(());
        }

        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        for key in keys {
            if let Ok(func) = lua.registry_value::<LuaFunction>(&key) {
                let table = lua.create_table()?;
                table.set("name", payload.name.as_str())?;
                table.set("path", payload.path.to_string_lossy().as_ref())?;
                let _ = func.call::<()>(table);
            }
        }

        Ok(())
    }

    /// Evaluates all custom dynamic candidate sources registered via `omega.register_source`.
    pub fn eval_custom_sources(&self) -> LuaResult<Vec<CustomCandidate>> {
        let sources = {
            let st = self
                .state
                .lock()
                .map_err(|e| LuaError::external(format!("{e}")))?;
            st.sources.clone()
        };

        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        let mut results = Vec::new();

        for (source_name, key) in sources {
            if let Ok(func) = lua.registry_value::<LuaFunction>(&key) {
                match func.call::<LuaTable>(()) {
                    Ok(table) => {
                        for row in table.sequence_values::<LuaTable>().flatten() {
                            let name: String = row.get("name").unwrap_or_default();
                            let path_str: String = row.get("path").unwrap_or_default();
                            let icon: Option<String> = row.get("icon").ok();
                            if !name.is_empty() && !path_str.is_empty() {
                                results.push(CustomCandidate {
                                    source_name: source_name.clone(),
                                    name,
                                    path: PathBuf::from(path_str),
                                    icon,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[omega:warn] Source '{source_name}' evaluation error: {e}");
                    }
                }
            }
        }

        Ok(results)
    }
}
