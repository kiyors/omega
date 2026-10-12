# Research Report: Lua Runtime & Binding Crates for Omega

**Context**: Investigation for Issue [#5](https://github.com/kiyors/omega/issues/5) — "Research Lua runtime options for Rust embedding"  
**Date**: 2026-10-12  

---

## Executive Summary & Recommendation

**Recommendation: Use `mlua` with features `["lua54", "vendored", "serde"]`.**

- **Why `mlua`:** The Rust-Lua ecosystem has converged completely on `mlua`. Its predecessor `rlua` was formally deprecated and merged into `mlua`, and `hlua` has been abandoned for years. `mlua` is actively maintained, battle-tested in major terminal and CLI applications (notably **WezTerm**), has full async/await support, first-class Serde integration, and solves the fundamental soundness hazards of Lua's C `longjmp` vs Rust stack unwinding.
- **Why Lua 5.4 over LuaJIT:** For Omega (a high-performance terminal session navigator), configuration loading and plugin hooks are IO/event-driven orchestrations, not CPU-bound math loops. Lua 5.4 provides modern language constructs that plugin authors expect (native 64-bit integers, native bitwise operators, `<close>` RAII variables, standard `utf8` library, generational GC), compiles effortlessly on all target architectures (including Apple Silicon without memory allocator hacks), and avoids LuaJIT’s 2GB GC32 memory ceiling and 20-year-old Lua 5.1 syntax limitations.

---

## 1. Detailed Evaluation of Evaluated Crates

### 1.1 `mlua` (The Undisputed Modern Standard)
* **Repository:** [github.com/mlua-rs/mlua](https://github.com/mlua-rs/mlua)
* **Crates.io:** `0.12.2` | **Downloads:** ~7.71M total (~2.18M recent) | **Stars:** 2,896 | **Open Issues:** 59
* **Maintainer:** Aleksandr Khvorov (`khvzak`) and the `mlua-rs` organization.
* **Lua Version Support:** Lua 5.1, 5.2, 5.3, 5.4, 5.5, LuaJIT 2.0/2.1 (including `luajit52`), and Roblox Luau. Also supports WASM (`wasm32-unknown-emscripten`).
* **Safety Guarantees:** 
  - Soundness: Wraps all Lua C API invocations in protected execution (`lua_pcall`) to prevent C `longjmp` from skipping over Rust stack frames (which would bypass `Drop` destructors and cause UB/leaks).
  - Panic Safety: Rust panics generated inside Rust callbacks are caught and transformed into Lua errors. Alternatively, via `LuaOptions`, `mlua` can prevent Lua `pcall` from swallowing Rust panics and safely resume them across the FFI boundary. Internal state remains intact even after a panic.
* **API Ergonomics:** Exceptional.
  - Exposes Rust structs via the `UserData` trait with `add_fields` and `add_methods`.
  - Supports automatic conversion through `IntoLua` and `FromLua`.
  - With the `serde` feature flag, any Rust type implementing `serde::Deserialize` can be deserialized directly from a Lua table using `lua.from_value::<T>(val)`.
  - Removes the awkward scoped `context(|ctx| { ... })` closure pattern required by older `rlua`.
* **Async Support:** First-class. Lua coroutines bridge seamlessly to Rust futures. Supports `lua.create_async_function` and `methods.add_async_method`. Execution can be driven by Tokio, async-std, or smol via `Function::call_async` or `Lua::load().exec_async()`.
* **Build Options:**
  - `features = ["vendored"]`: Compiles Lua or LuaJIT directly from C source via `lua-src` / `luajit-src` in ~1–2 seconds. No external C libraries or system packages needed.
  - System mode: Falls back to `pkg-config` or explicit environment variables (`LUA_LIB`, `LUA_LIB_NAME`, `LUA_LINK`).
* **Notable Users:**
  - **WezTerm** (GPU-accelerated terminal emulator/multiplexer using `mlua` for configuration and user plugins).
  - **Blightmud** (Rust terminal MUD client using `mlua` for extensibility).
  - **bevy_mod_scripting** (game engine scripting engine).
  - Multiple Neovim-adjacent tools and plugins (e.g. `nvim-oxi` ecosystem bridges, `toml-edit.lua`).

---

### 1.2 `rlua` (The Pioneer — Deprecated & Merged)
* **Repository:** [github.com/mlua-rs/rlua](https://github.com/mlua-rs/rlua) (Archived)
* **Crates.io:** `0.20.1` | **Downloads:** ~800k total | **Stars:** 1,757 | **Open Issues:** 5
* **Author:** `kyren` (original author), maintained under `mlua-rs` org until retirement.
* **Lua Version Support:** Historically Lua 5.3 and partial LuaJIT.
* **Safety Guarantees:** Pioneered safe Rust-Lua bindings by enforcing strict scoped lifetimes via `lua.context(|lua_ctx| { ... })`. All handles borrowed `&'lua Context`, preventing Lua values from escaping.
* **API Ergonomics:** Cumbersome due to strict lifetime bounding. You could not store a `LuaTable` or `LuaFunction` in an external Rust struct without complex lifetime gymnastics or using the Lua registry for everything.
* **Async Support:** None.
* **Current Status:** **Officially deprecated and archived.** Version 0.20.1 was released solely as a transitional shim that re-exports `mlua`. The `rlua` project merged into `mlua`. Do not use for new projects.

---

### 1.3 `hlua` (The Outdated / Soundness Risk)
* **Repository:** [github.com/tomaka/hlua](https://github.com/tomaka/hlua)
* **Crates.io:** `0.4.3` | **Downloads:** ~126k total | **Stars:** 522 | **Open Issues:** 50
* **Author:** Pierre Krieger (`tomaka`).
* **Lua Version Support:** Lua 5.2 only.
* **Safety Guarantees:** **Unsound.** Does not properly guard against Lua errors performing `longjmp` across Rust frames with active destructors. Destructors are skipped on error, causing memory leaks and undefined behavior in unsafe blocks relying on `Drop`. Rust panics traversing C frames were also not safely contained.
* **API Ergonomics:** Clunky and restrictive:
  - Registering functions required manual arity wrappers: `hlua::function2(add)`, `hlua::function3(...)`.
  - Holding a `LuaFunction` required borrowing `&mut Lua`, locking the entire context.
  - Storing callbacks for later invocation was unsupported out of the box.
* **Async Support:** None.
* **Current Status:** **Abandoned since ~2017.** Do not use.

---

### 1.4 Other Viable Alternatives

| Crate | Version | Downloads | Description & Suitability for Omega |
| :--- | :--- | :--- | :--- |
| **`piccolo`** | `0.3.3` | ~94k | Pure Rust stackless Lua 5.4 VM by `kyren` using `gc-arena`. Safe, no C FFI. However, it is explicitly **experimental** with frequent breaking API changes, lower performance, and incomplete ecosystem support. Unsuitable for production. |
| **`ezlua`** | `0.5.4` | ~24k | Ergonomic zero-cost Lua 5.4 bindings. Has async and serde support, but **requires Rust Nightly (1.76+)** and has a tiny user base (37 stars). Unsuitable for a stable tool like Omega. |
| **`tealr`** | `0.11.0` | ~41k | **Companion library for `mlua`**, not a separate runtime. Generates type definitions (Teal, EmmyLua / LuaLS) and documentation from `mlua::UserData` implementations. Highly useful for generating autocomplete definitions for Omega users! |

---

## 2. Feature Comparison Matrix

| Criteria | `mlua` | `rlua` | `hlua` | `piccolo` |
| :--- | :--- | :--- | :--- | :--- |
| **Active Maintenance** | **Yes** (Oct 2026 releases) | **No** (Archived) | **No** (Abandoned) | Yes (Experimental) |
| **Lua Versions** | 5.1, 5.2, 5.3, 5.4, 5.5, LuaJIT, Luau | 5.3, LuaJIT (shim to mlua) | 5.2 only | 5.4 subset |
| **Memory / Panic Safety** | **Sound** (pcall guards + panic catching) | Sound | **Unsound** (longjmp leaks drops) | Memory safe (Pure Rust) |
| **Async / Await** | **Yes** (Tokio, async-std, smol) | No | No | Custom `Sequence` model |
| **Serde Integration** | **Yes** (`mlua/serde` built-in) | Partial / third-party | Limited | No |
| **Build Setup** | `vendored` feature (zero external deps) | Manual or vendored | Manual system linking | Pure Rust (`cargo build`) |
| **Exposing Structs** | `UserData` trait (fields, methods, metamethods) | `UserData` trait | Raw `Push` / `PushGuard` | Custom GC wrappers |
| **Callback Storage** | `RegistryKey` (unbounded lifetime) | Requires Registry hacks | Not supported | GC handles |
| **Neovim-style API** | **Trivial** (`lua.from_value` + modules) | Complex | Limited | Complex |

---

## 3. Lua 5.4 vs LuaJIT: Architectural Analysis for Omega

Neovim chose LuaJIT for legacy reasons and raw loop speed in its early rewrite. However, for a terminal workspace navigator like **Omega**, **Lua 5.4 is the vastly superior choice**.

### 3.1 Domain Workload Analysis: Config & Plugins vs Number Crunching
- Omega's Lua runtime performs two tasks:
  1. **Configuration parsing** (runs once on startup): Reading tables and translating them to Rust structs.
  2. **Plugin lifecycle hooks**: Reacting to events like `on_session_create`, `on_session_switch`, or `filter_hook`.
- Lua spends microseconds running user hooks before handing control back to Rust. Fuzzy search (Skim/Nucleo), directory traversal (WalkDir), and TUI rendering (Ratatui) are already written in multi-threaded Rust.
- A JIT compiler provides **zero performance benefit** for config tables and short hook callbacks. A tracing JIT compiler needs thousands of iterations through a hot loop to emit machine code; startup scripts and occasional hook dispatches never trigger trace compilation.

### 3.2 Language Modernity (Lua 5.4 vs Lua 5.1/LuaJIT)
LuaJIT is frozen on Lua 5.1 semantics (released in 2006). Lua 5.4 offers critical features:
1. **Native 64-bit Integers (`integer` vs `float`):**
   - Lua 5.1 represents all numbers as IEEE-754 64-bit doubles (integers above $2^{53}$ lose precision).
   - In Omega, handling process IDs (PIDs), millisecond/nanosecond timestamps, file offsets, and git commit hashes requires exact 64-bit integers. Lua 5.4 provides native 64-bit integers and bitwise operators (`&`, `|`, `~`, `>>`, `<<`) without clunky C library shims (`bit.band`).
2. **Deterministic Cleanup with `<close>` Variables:**
   - Lua 5.4 supports to-be-closed local variables: `local f <close> = io.open(...)`.
   - Plugin authors get RAII-style deterministic resource cleanup (temp files, locks, terminal buffers) even when runtime errors occur.
3. **Native UTF-8 Standard Library (`utf8`):**
   - Terminal workspace navigators constantly deal with UTF-8 glyphs (git branch icons, Nerd Fonts, Unicode file names).
   - Lua 5.1 has no UTF-8 awareness (`string.len("🚀") == 4`). Lua 5.4 provides the built-in `utf8` library (`utf8.len`, `utf8.codes`, `utf8.offset`).
4. **Generational Garbage Collection:**
   - Lua 5.4 introduces generational GC (`collectgarbage("generational")`). Short-lived allocations generated during hook execution are reclaimed immediately, eliminating latency hiccups during interactive terminal navigation.

### 3.3 Portability and Memory Limits
1. **The 2GB Ceiling (GC32) in LuaJIT:**
   - LuaJIT on 64-bit architectures uses 32-bit tagged pointers (NaN boxing), imposing a hard 2GB ceiling on all Lua allocations.
   - On macOS ARM64 (Apple Silicon), this requires custom page allocators or linker workarounds because `MAP_32BIT` is not supported by the Darwin kernel.
2. **Build and Cross-Compilation Ergonomics:**
   - Lua 5.4 is clean, standard ANSI C99. It cross-compiles effortlessly to `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, and musl with standard Cargo cross tooling.
   - LuaJIT requires running host binaries (`minilua`, `buildvm`) during the build to generate dynasm architecture files, making cross-compilation fragile.

---

## 4. Implementation Blueprints for Omega

### Blueprint 1: Neovim-Style `omega.setup({ key = value })` API

Using `mlua` with `serde`:

```rust
use mlua::prelude::*;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct OmegaConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub preview: PreviewConfig,
    #[serde(default)]
    pub keybindings: HashMap<String, String>,
}

fn default_theme() -> String { "default".to_string() }

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PreviewConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_preview_height")]
    pub height: u16,
}

fn default_true() -> bool { true }
fn default_preview_height() -> u16 { 15 }

pub struct LuaEngine {
    lua: Lua,
    pub config: Arc<Mutex<OmegaConfig>>,
}

impl LuaEngine {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let config = Arc::new(Mutex::new(OmegaConfig::default()));

        // Create the `omega` table
        let omega = lua.create_table()?;

        // omega.setup({ ... })
        let cfg_ref = config.clone();
        let setup_fn = lua.create_function(move |lua, value: LuaValue| {
            // Direct Serde deserialization from Lua table into Rust struct
            let parsed_config: OmegaConfig = lua.from_value(value)?;
            let mut cfg = cfg_ref.lock().unwrap();
            *cfg = parsed_config;
            Ok(())
        })?;
        omega.set("setup", setup_fn)?;

        // Expose both as global `omega` and as `require("omega")`
        lua.globals().set("omega", omega.clone())?;
        let package: LuaTable = lua.globals().get("package")?;
        let loaded: LuaTable = package.get("loaded")?;
        loaded.set("omega", omega)?;

        Ok(Self { lua, config })
    }

    pub fn load_user_config(&self, path: &std::path::Path) -> LuaResult<()> {
        if path.exists() {
            let code = std::fs::read_to_string(path).map_err(LuaError::external)?;
            self.lua.load(&code).set_name(path.to_string_lossy())?.exec()?;
        }
        Ok(())
    }
}
```

### Blueprint 2: Registering Lua Callbacks / Plugin Hooks

Using `mlua::RegistryKey` to store closures without lifetime leaks:

```rust
use mlua::prelude::*;
use mlua::RegistryKey;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct HookRegistry {
    // RegistryKey has no 'lua lifetime and can safely outlive the callback scope
    hooks: Arc<Mutex<HashMap<String, Vec<RegistryKey>>>>,
}

impl HookRegistry {
    pub fn register_api(&self, lua: &Lua) -> LuaResult<()> {
        let hooks = self.hooks.clone();
        let on_fn = lua.create_function(move |lua, (event, callback): (String, LuaFunction)| {
            // Store the Lua function in the Lua registry
            let key = lua.create_registry_value(callback)?;
            let mut map = hooks.lock().unwrap();
            map.entry(event).or_default().push(key);
            Ok(())
        })?;

        let omega: LuaTable = lua.globals().get("omega")?;
        omega.set("on", on_fn)?;
        Ok(())
    }

    pub fn dispatch(&self, lua: &Lua, event: &str, payload: LuaValue) -> LuaResult<()> {
        let keys = {
            let map = self.hooks.lock().unwrap();
            map.get(event).cloned().unwrap_or_default()
        };

        for key in keys {
            // Retrieve function from registry and execute
            let callback: LuaFunction = lua.registry_value(&key)?;
            if let Err(err) = callback.call::<()>(payload.clone()) {
                eprintln!("[omega:hook error] event '{event}': {err}");
            }
        }
        Ok(())
    }
}
```

---

## 5. Summary Verdict for Omega

1. **Crate**: **`mlua = "0.12"`**
2. **Features**: `features = ["lua54", "vendored", "serde"]`
3. **Runtime Dialect**: **Lua 5.4**
   - Full 64-bit integer integrity for IDs and timestamps.
   - Clean ANSI C99 compilation across all architectures without assembly shims.
   - No 2GB GC memory limits on 64-bit / macOS ARM64.
   - Generational garbage collection for instant short-lived allocation disposal.
   - Built-in UTF-8 and `<close>` RAII support.
4. **Optional Future Extension**: Integrate **`tealr`** to automatically output EmmyLua / LuaLS type definition files (`omega.lua`), giving end users rich autocompletion and diagnostic warnings in Neovim/VSCode when configuring Omega.
