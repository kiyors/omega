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
