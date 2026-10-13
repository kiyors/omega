# Feature Record: Phase 4 Extensibility & Polish

- **Date**: 2026-10-13
- **Author**: Agent Pair Programmer
- **Scope**: Embedded Lua 5.4 Engine (`mlua`), Declarative `omega.setup`, Plugins and Lifecycle Hooks, Wildcards & Startup Layout Provisioning, Doctor Diagnostics, Shell Completions, Homebrew Formula, and Nix Flake

---

## 1. What Was Built

Delivered Phase 4 (Extensibility & Polish) as specified in `roadmap.md` and `ARCHITECTURE.md`:

1. **Embedded Lua 5.4 Runtime Engine (`crates/config/src/engine.rs`)**:
   - Integrated `mlua = { version = "0.12", features = ["lua54", "vendored", "serde", "send"] }` embedding a self-contained ANSI C99 Lua 5.4 runtime.
   - Declarative `omega.setup({ ... })` API parsing user options directly into Rust configuration models (`hide_attached`, `dir_length`, `tui`, `icons`, `name_substitutions`, `aliases`, `wildcards`, `keymaps`, `sort_order`).
   - Config directory path resolution (`~/.config/omega/init.lua` and `$XDG_CONFIG_HOME/omega/init.lua`).
   - Package path configuration auto-adding `~/.config/omega/lua/` to Lua `package.path` for modular user libraries.
   - Plugin script auto-loading executing all `*.lua` files in `~/.config/omega/plugins/` on application launch.

2. **Extensibility & Plugin Hooks (`crates/config/src/engine.rs`)**:
   - Custom naming hook `omega.session_name(function(path, git_context) ... end)` integrated into the `NamingPipeline`.
   - Lifecycle event hook dispatch `omega.on("session_created", ...)` and `omega.on("session_switched", ...)` notified whenever sessions are provisioned or switched.
   - Custom dynamic source registration `omega.register_source("name", function() return { ... } end)` yielding dynamically queried session candidates in the interactive picker.

3. **Wildcards & Multi-Window Startup Layout Provisioning (`crates/session/src/layout.rs`)**:
   - Glob pattern matcher (`match_wildcard`) supporting `~` home directory expansion, single-segment wildcards (`*`), and deep recursive wildcards (`**`).
   - Declarative window and pane startup layout provisioning (`provision_session_layout`) replacing external tools like `tmuxinator` or `tmuxp`.
   - Automatically handles window creation, split panes, native layout styles (e.g. `main-vertical`, `even-horizontal`), and initial command execution upon session creation.
   - Automatic candidate icon enrichment from matched wildcard rules and configured source icons.

4. **Diagnostics Subcommand (`src/doctor.rs`)**:
   - `omega doctor`: Health check validating `tmux` (presence, version), `git` (presence, version), `zoxide` (presence), background cache daemon socket responsiveness, and `init.lua` syntax/runtime validity.
   - Environment abstraction trait (`DoctorEnv`) enabling deterministic unit testing.

5. **Shell Completions (`src/cli.rs`, `src/commands.rs`)**:
   - `omega completions <bash|zsh|fish>` powered by `clap_complete = "4.6"`, generating native shell autocompletions for all subcommands, arguments, and flags.

6. **Distribution Packaging (`flake.nix`, `Formula/omega.rb`)**:
   - Nix Flake package output `packages.default` with `rustPlatform.buildRustPackage` building the standalone Omega binary.
   - Homebrew tap formula (`Formula/omega.rb`) for macOS with clean dependency and test declarations.

---

## 2. Verification & Testing

Implemented test-first following Red-Green-Refactor across all pre-agreed seams:

- **Seam 1 (`crates/config/tests/lua_config_test.rs`)**:
  - `test_lua_engine_loads_declarative_setup`: Verifies full `omega.setup({ ... })` parsing into typed Rust configuration models.
  - `test_lua_engine_syntax_error_reported`: Verifies syntax errors in Lua scripts produce meaningful `Err` results.
  - `test_lua_engine_package_path_and_plugins`: Verifies custom module resolution in `lua/` and auto-sourcing in `plugins/`.

- **Seam 2 (`crates/config/tests/lua_hooks_test.rs`)**:
  - `test_lua_custom_naming_hook`: Verifies custom naming hook callbacks with and without Git context.
  - `test_lua_lifecycle_event_hook`: Verifies `session_created` and `session_switched` event dispatching.
  - `test_lua_register_custom_source`: Verifies dynamic candidate source evaluation.

- **Seam 3 (`crates/session/tests/layout_and_wildcard_test.rs`, `tests/lua_layout_provision_test.rs`)**:
  - `test_match_wildcard_patterns`: Verifies single-star and double-star glob matching and home directory resolution.
  - `test_provision_session_layout_single_window_panes_and_layout`: Verifies pane splitting, layout strings, and command dispatch.
  - `test_provision_session_layout_multiple_windows`: Verifies multiple window provisioning.
  - `test_connect_provisions_layout_from_lua_config`: Verifies end-to-end `omega connect` layout provisioning and lifecycle hook firing from `init.lua`.

- **Seam 4 (`tests/doctor_test.rs`)**:
  - `test_doctor_reports_all_healthy_when_tools_and_valid_config_present`: Verifies report generation when all tools and valid config are present.
  - `test_doctor_flags_missing_tmux_and_syntax_error_in_config`: Verifies error reporting when tmux is missing or `init.lua` has syntax errors.

- **Seam 5 (`tests/completions_test.rs`)**:
  - `test_cli_parse_doctor_and_completions`: Verifies CLI parser recognizes `doctor` and `completions` subcommands.
  - `test_generate_completions_output`: Verifies completion script output for Bash, Zsh, and Fish.

- **Seam 6 (`tests/packaging_test.rs`)**:
  - `test_homebrew_formula_structure_and_validity`: Verifies `Formula/omega.rb` structure, repository URL, and test assertions.
  - `test_nix_flake_declares_package_build`: Verifies `flake.nix` package outputs and derivation metadata.

- **Quality Gates**:
  - `cargo test --workspace`: All test suites pass.
  - `cargo clippy --all-targets -- -D warnings`: Passes with zero warnings under strict clippy settings.
  - `cargo fmt --check`: Clean formatting across the entire codebase.

---

## 3. Code Review Refinements

Following the two-axis code review against Standards (`AGENTS.md`) and Spec (`roadmap.md`), the following enhancements were implemented:
- **Zero-Allocation Glob Matching**: Refactored `glob_match` and `match_wildcard` in `crates/session/src/layout.rs` using borrowed iterators and prefix stripping, removing all intermediate `Vec` and `String` heap allocations on candidate scoring loops (`AGENTS.md §5.1, §5.2`).
- **Regex Path Substitutions**: Added `regex` engine support to `crates/session/src/naming.rs` allowing regex token matching (e.g. `^/Users/[^/]+/personal/`) and deduplicated the substitution execution logic.
- **Standalone Plugins Auto-Loading**: Implemented `LuaEngine::from_config_dir` and `Config::load_with_engine` ensuring `plugins/*.lua` scripts execute on launch even when `init.lua` is absent.
- **Git Context Propagation**: Implemented `find_git_context` in `crates/session/src/discovery.rs` and populated `GitContext` (repository name and branch) into `omega.session_name` callbacks in CLI and connect commands.
- **Custom Sources in CLI**: Added `eval_custom_sources` integration in `omega connect` and `omega list` so dynamically registered Lua candidates are recognized across CLI subcommands.
- **Domain String Key Projections**: Added `SessionSource::as_str(&self) -> &'static str` to replace inline enum matching.
- **TUI Keymaps & Fallthrough Protection**: Wired `<CR>` (connect) and `<C-x>` / `kill_session` actions in the TUI event loop, while guarding unmapped control key events from inserting into query text.

---

## 4. Subagent Audit & Ecosystem Polish (Nix, Rust 1.99.0, Lua Architecture & Performance)

Following a comprehensive subagent audit spanning standards, spec compliance, performance, and Lua runtime ergonomics:

1. **Rust 1.99.0 Pinning & Modular Nix Architecture**:
   - Locked toolchain to `1.99.0` via `rust-toolchain.toml` with `clippy` and `rustfmt` profiles.
   - Modularized Nix build infrastructure: decoupled `nix/package.nix` (`buildRustPackage` derivation) and `nix/devshell.nix` (developer environment pinned to 1.99.0 via fenix), keeping `flake.nix` minimal, clean, and extensible.
   - Updated integration tests in `tests/packaging_test.rs` to verify the presence of `rust-toolchain.toml` and `nix/package.nix`.

2. **Lua Runtime Architecture & Quality of Life (QoL)**:
   - Consolidated fragmented mutexes in `LuaEngine` into a single `Arc<Mutex<LuaState>>`, eliminating lock contention and ensuring thread-safe access to the single-threaded `mlua::Lua` runtime.
   - Made `omega.session_name` chainable (`Vec<Arc<RegistryKey>>`), cascading through registered naming callbacks until one returns a non-nil name.
   - Isolated plugin error handling: `load_plugins` captures runtime errors per plugin file and populates engine `diagnostics`, allowing other plugins to load unimpeded.
   - Exposed `omega.version` and `omega.log` (`info`, `warn`, `error`) utilities in the Lua global scope.

3. **Performance & Allocation Reductions (`AGENTS.md §5.1, §5.2`)**:
   - Added `match_wildcard_with_home` in `crates/session/src/layout.rs` and hoisted environment variable lookups (`HOME`) out of the candidate loop in `crates/session/src/collector.rs`.
   - Optimized path resolution in `crates/session/src/naming.rs`: replaced full component heap allocation (`collect::<Vec<_>>()`) with `.rev().take(count)`, and precompiled regex substitutions once per resolution pass.

4. **Spec Polish & Edge Cases**:
   - Wildcard startup commands in `smart_connect`: executed via `tmux.send_keys(&created.pane_id, cmd)` rather than as the primary window command, preventing non-interactive or short-lived commands (e.g. `cargo check`) from terminating the session window upon completion.
   - Added `~` home directory expansion to alias path matching in `smart_connect`.
   - Enhanced target path detection in `omega connect <target>` so leading `/`, `~`, or `.` paths are recognized as filesystem targets even before the directory is created.
   - Added `config.json` fallback verification to `omega doctor`.

5. **Declarative Sort Order & Score Interleaving (`SessionCollector::apply_sort_order`)**:
   - Implemented candidate grouping and prioritization based on declarative `config.sort_order`.
   - Supports discrete source tiers (e.g. `"tmux"`, `"alias"`), custom source names (e.g. `"docker"`), and interleaved array groups (e.g. `["config", "git", "zoxide"]`) ranked by descending frecency score.
   - Retains stable sorting order for equal or missing scores, with unlisted sources deferred to the final tier.
   - Thoroughly tested in `crates/session/tests/sort_order_test.rs`.

6. **Terminal Capabilities Diagnostic (`omega doctor`)**:
   - Added environment inspection for `$COLORTERM` and `$TERM` detecting TrueColor (24-bit RGB) and 256-color capabilities.
   - Flags degraded or missing `$TERM` (e.g. `dumb`) with descriptive warning messages and advice.
   - Fully covered by unit tests in `tests/doctor_test.rs`.

