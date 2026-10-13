pub mod engine;

pub use engine::LuaEngine;
use omega_config::Config;

/// Loads configuration alongside an optional active `LuaEngine` instance.
/// Precedence:
/// 1. `init.lua` or `plugins/` in config directory bootstraps `LuaEngine`.
/// 2. `config.json` in config directory loads pure JSON configuration.
/// 3. Fallback to `(Config::default(), None)`.
pub fn load_config_and_engine() -> (Config, Option<LuaEngine>) {
    if let Some(dir) = Config::config_dir() {
        let has_lua = dir.join("init.lua").exists() || dir.join("plugins").is_dir();
        if has_lua {
            match LuaEngine::from_config_dir(&dir) {
                Ok(engine) => {
                    let cfg = engine.config();
                    return (cfg, Some(engine));
                }
                Err(e) => {
                    eprintln!(
                        "[omega:warn] Failed to load Lua config from {}: {e}",
                        dir.display()
                    );
                }
            }
        }

        let config_json = dir.join("config.json");
        if config_json.exists()
            && let Ok(cfg) = Config::load_from_path(&config_json)
        {
            return (cfg, None);
        }
    }

    (Config::default(), None)
}
