use crate::{Config, CustomCandidate, GitContext, SessionEventPayload};
use mlua::RegistryKey;
use mlua::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type HookMap = HashMap<String, Vec<Arc<RegistryKey>>>;
type SourceList = Vec<(String, Arc<RegistryKey>)>;

/// Embedded Lua 5.4 runtime host for Omega.
#[derive(Clone)]
pub struct LuaEngine {
    lua: Arc<Mutex<Lua>>,
    config: Arc<Mutex<Config>>,
    naming_hook: Arc<Mutex<Option<Arc<RegistryKey>>>>,
    hooks: Arc<Mutex<HookMap>>,
    sources: Arc<Mutex<SourceList>>,
}

impl LuaEngine {
    /// Initializes a new Lua 5.4 runtime with the `omega` global and `package.loaded["omega"]` table.
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let config = Arc::new(Mutex::new(Config::default()));
        let naming_hook: Arc<Mutex<Option<Arc<RegistryKey>>>> = Arc::new(Mutex::new(None));
        let hooks: Arc<Mutex<HookMap>> = Arc::new(Mutex::new(HashMap::new()));
        let sources: Arc<Mutex<SourceList>> = Arc::new(Mutex::new(Vec::new()));

        let omega = lua.create_table()?;

        // omega.setup({ ... })
        let cfg_clone = Arc::clone(&config);
        let setup_fn = lua.create_function(move |lua, value: LuaValue| {
            let parsed_config: Config = lua.from_value(value)?;
            if let Ok(mut cfg) = cfg_clone.lock() {
                *cfg = parsed_config;
            }
            Ok(())
        })?;
        omega.set("setup", setup_fn)?;

        // omega.session_name(function)
        let naming_clone = Arc::clone(&naming_hook);
        let session_name_fn = lua.create_function(move |lua, func: LuaFunction| {
            let key = Arc::new(lua.create_registry_value(func)?);
            if let Ok(mut hook) = naming_clone.lock() {
                *hook = Some(key);
            }
            Ok(())
        })?;
        omega.set("session_name", session_name_fn)?;

        // omega.on(event, function)
        let hooks_clone = Arc::clone(&hooks);
        let on_fn = lua.create_function(move |lua, (event, func): (String, LuaFunction)| {
            let key = Arc::new(lua.create_registry_value(func)?);
            if let Ok(mut map) = hooks_clone.lock() {
                map.entry(event).or_default().push(key);
            }
            Ok(())
        })?;
        omega.set("on", on_fn)?;

        // omega.register_source(name, function)
        let sources_clone = Arc::clone(&sources);
        let register_src_fn =
            lua.create_function(move |lua, (name, func): (String, LuaFunction)| {
                let key = Arc::new(lua.create_registry_value(func)?);
                if let Ok(mut src_list) = sources_clone.lock() {
                    src_list.push((name, key));
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
            config,
            naming_hook,
            hooks,
            sources,
        })
    }

    /// Access the current parsed configuration snapshot.
    pub fn config(&self) -> Config {
        self.config
            .lock()
            .map(|c| c.clone())
            .unwrap_or_else(|_| Config::default())
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
                lua.load(&code).set_name(path.to_string_lossy()).exec()?;
            }
        }

        Ok(())
    }

    /// Loads and executes a Lua file at the specified path.
    pub fn load_file(&self, path: &Path) -> LuaResult<()> {
        let code = fs::read_to_string(path).map_err(LuaError::external)?;
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        lua.load(&code).set_name(path.to_string_lossy()).exec()
    }

    /// Loads and executes a Lua code string directly.
    pub fn load_string(&self, code: &str) -> LuaResult<()> {
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        lua.load(code).exec()
    }

    /// Evaluates a global variable and returns its deserialized value.
    pub fn eval_global<T: FromLua>(&self, name: &str) -> LuaResult<T> {
        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        lua.globals().get(name)
    }

    /// Invokes the custom session name hook, if registered.
    pub fn call_session_name_hook(
        &self,
        path: &Path,
        git_context: Option<&GitContext>,
    ) -> Option<String> {
        let key_opt = self.naming_hook.lock().ok()?.as_ref().cloned()?;
        let lua = self.lua.lock().ok()?;
        let hook: LuaFunction = lua.registry_value(&key_opt).ok()?;
        let path_str = path.to_string_lossy();

        let res: LuaValue = if let Some(ctx) = git_context {
            let git_table = lua.create_table().ok()?;
            git_table.set("repo", ctx.repo.as_str()).ok()?;
            git_table.set("branch", ctx.branch.as_str()).ok()?;
            hook.call((path_str.as_ref(), git_table)).ok()?
        } else {
            hook.call((path_str.as_ref(), LuaValue::Nil)).ok()?
        };

        match res {
            LuaValue::String(s) => s.to_str().ok().map(|slice| slice.to_owned()),
            _ => None,
        }
    }

    /// Dispatches a lifecycle event to registered event hooks (e.g. `session_created`, `session_switched`).
    pub fn dispatch_hook(&self, event: &str, payload: &SessionEventPayload) -> LuaResult<()> {
        let keys = {
            let map = self
                .hooks
                .lock()
                .map_err(|e| LuaError::external(format!("{e}")))?;
            map.get(event).cloned().unwrap_or_default()
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
            let list = self
                .sources
                .lock()
                .map_err(|e| LuaError::external(format!("{e}")))?;
            list.clone()
        };

        let lua = self
            .lua
            .lock()
            .map_err(|e| LuaError::external(format!("{e}")))?;
        let mut results = Vec::new();

        for (source_name, key) in sources {
            if let Ok(func) = lua.registry_value::<LuaFunction>(&key)
                && let Ok(table) = func.call::<LuaTable>(())
            {
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
        }

        Ok(results)
    }
}
