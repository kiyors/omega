# ADR: Extract Lua Runtime Subsystem into `omega-lua` (`crates/lua`)

- **Status**: Accepted
- **Date**: 2026-10-13
- **Deciders**: Engineering Team

## Context

In Phase 4 of Omega, an embedded Lua 5.4 runtime was introduced via `mlua = { version = "0.12", features = ["vendored", "lua54", "serde", "send"] }` to support declarative configuration (`omega.setup`), user plugins (`plugins/*.lua`), custom naming hooks, and dynamic session sources.

Initially, this runtime was placed inside `crates/config` (`omega-config`). However:

1. `omega-config` contains the canonical data definitions (`Config`, `TuiConfig`, `WildcardRule`, `AliasConfig`, etc.) required by almost every crate across the workspace (`omega-session`, `omega-daemon`, `omega`).
2. Bundling `mlua` into `omega-config` forced every dependent crate to transitively depend on C compilation and Lua FFI headers, increasing compilation times and coupling passive data schemas with the active scripting host.
3. Decoupling configuration data from execution runtimes aligns with standard terminal ecosystem design (e.g. Helix, Neovim, Ghostty).

## Decision

Extract the embedded Lua engine into a dedicated workspace crate: `crates/lua` (`omega-lua`).

- **`crates/config` (`omega-config`)**:
  - Contains strictly pure Rust configuration models, serde serialization, defaults, and static JSON loading (`Config::load_from_path`).
  - Zero C/FFI dependencies; fast, lightweight compilation.
- **`crates/lua` (`omega-lua`)**:
  - Houses the `LuaEngine`, `LuaState`, declarative `omega.setup` bridge, plugin auto-loader (`plugins/*.lua`), lifecycle event dispatchers (`omega.on`), and chainable naming hooks (`omega.session_name`).
  - Depends on `omega-config` to write configuration into typed `Config` instances.
  - Exclusively owns the `mlua` dependency.
- **Root Orchestrator (`src/`)**:
  - Connects `omega-config` and `omega-lua` to provide CLI and interactive TUI execution.

## Consequences

### Positive

- **Fast Build Times**: Crates needing only configuration structs (`omega-session`, `omega-daemon`) build without compiling vendored C Lua 5.4.
- **Clear Architectural Separation**: Passive configuration data models are completely isolated from active script runtime state.
- **Resilience & Fault Isolation**: Scripting host errors or API updates are confined to `crates/lua`.
- **Extensibility**: Makes optional feature-flagging (`--no-default-features` for minimal C-free builds) straightforward in the future.

### Neutral / Negative

- Added one workspace crate entry (`crates/lua`) to `Cargo.toml`.
