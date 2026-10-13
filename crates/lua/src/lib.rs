pub mod engine;

pub use engine::LuaEngine;
use omega_config::Config;
use std::path::Path;

/// Loads configuration alongside an optional active `LuaEngine` instance.
/// Precedence:
/// 1. `init.lua` or `plugins/` in config directory bootstraps `LuaEngine`.
/// 2. `config.json` in config directory loads pure JSON configuration.
/// 3. Fallback to `(Config::default(), None)`.
pub fn load_config_and_engine() -> (Config, Option<LuaEngine>) {
    if let Some(dir) = Config::config_dir() {
        let has_lua = dir.join("init.lua").exists() || dir.join("plugins").is_dir();
        if has_lua && let Ok(engine) = LuaEngine::from_config_dir(&dir) {
            let cfg = engine.config();
            return (cfg, Some(engine));
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

/// Loads configuration from an explicit file path (`.lua` or `.json`).
pub fn load_from_path(path: &Path) -> std::io::Result<Config> {
    if path.extension().and_then(|s| s.to_str()) == Some("lua") {
        let dir = path.parent().unwrap_or(path);
        let engine =
            LuaEngine::from_config_dir(dir).map_err(|e| std::io::Error::other(format!("{e}")))?;
        Ok(engine.config())
    } else {
        Config::load_from_path(path)
    }
}
