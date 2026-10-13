# Feature Record: Phase 5 Multiplexer Mastery & Workflow Consolidation

- **Date**: 2026-10-13
- **Author**: Agent Pair Programmer
- **Scope**: Native Tmux Window Navigation (`omega window`, `:w` Sigil), Persistent Floating Scratchpad (`omega scratch`), Shell & Multiplexer Integrations (`omega init`), and Semantic Theming Verification

---

## 1. What Was Built

Delivered Phase 5 (Multiplexer Mastery & Workflow Consolidation) as specified in `roadmap.md` and `ARCHITECTURE.md`:

1. **Native Tmux Window Navigation (`omega window` & `:w` Sigil)**:
   - Added `SessionSource::Window` supporting prefix sigil `:w` and `:w <query>`.
   - Implemented `parse_tmux_window_candidates` parsing `tmux list-windows -a` streams (`session_name\twindow_name\tpane_current_path\twindow_active`) into `<session>:<window>` candidates.
   - Enforced deduplication identity separation so windows within the same directory do not collide with each other or shadow directory sessions.
   - Live preview pane capturing pane contents via `tmux capture-pane -e -p -t <session>:<window>`.
   - Dedicated `omega window [query]` CLI subcommand launching the interactive TUI pre-filtered across all active tmux windows.
   - Direct tmux client switching via `select_window` and `switch_client` when selected.

2. **Persistent Floating Scratchpad (`omega scratch`)**:
   - Added `omega scratch [--name <tag>]` subcommand defaulting to `"scratch"` session.
   - Inside tmux: displays a centered 80% x 80% popup overlay executing `tmux new-session -A -s <name>`, persisting terminal jobs indefinitely across popup dismissals.
   - Outside tmux: seamlessly attaches or connects to the scratchpad session.

3. **Shell & Multiplexer Integrations (`omega init`)**:
   - Added `omega init <zsh|bash|fish|tmux> [--tmux]` subcommand via `src/init.rs`.
   - Zsh: Generates self-contained ZLE widgets and Alt+s keybindings (`bindkey '\es'` / `bindkey '^[s'`) with clean TTY allocation (`exec </dev/tty`) evaluating cleanly via `eval "$(omega init zsh)"`.
   - Bash: Emits `bind -x` readline functions with `exec </dev/tty`.
   - Fish: Emits `commandline -f repaint` functions with `</dev/tty`.
   - Tmux: Generates the 4 canonical popup and helper keybindings:
     - `bind-key -n M-s display-popup -E -w 85% -h 80% "omega"`
     - `bind-key -n M-w display-popup -E -w 85% -h 80% "omega window"`
     - `bind-key L run-shell "omega last"`
     - `bind-key 9 run-shell "omega root"`

4. **Embedded Semantic Theming & Catppuccin Presets**:
   - Built-in color palette engine supporting 8 presets (`catppuccin-mocha`, `catppuccin-macchiato`, `catppuccin-frappe`, `catppuccin-latte`, `tokyonight`, `nord`, `gruvbox`, `default`) with 20+ granular slot overrides.
   - Programmatic Lua theme API (`omega.theme`, `omega.ui.theme`).
   - Sesh UI adaptations: source-group separator lines, inline window peek with dynamic width budget elision, and interactive deletion confirmation modal dialogs.

---

## 2. Verification & Testing

Implemented test-first following Red-Green-Refactor across all pre-agreed seams:

- **Seam 1 (`crates/session/tests/window_navigation_test.rs`, `crates/session/tests/dedup_test.rs`)**:
   - `test_window_source_sigils_and_attributes`: Verifies `:w` sigil extraction, prefix stripping, and attribute access.
   - `test_parse_tmux_window_candidates`: Verifies tab-separated window parsing into `<session>:<window>` candidates with paths and active flags.
   - `test_benchmark_window_aggregation_1000`: Verifies aggregating 1,000 tmux windows executes in < 15ms (0.00s in practice).

- **Seam 2 (`crates/tmux/tests/tmux_actions_test.rs`, `tests/cli_test.rs`)**:
   - `test_switch_window_inside_tmux`: Verifies `select_window` and `switch_client` are dispatched.
   - `test_switch_window_outside_tmux`: Verifies `attach_session` is dispatched.
   - `test_cli_parse_window`: Verifies `omega window [query]` CLI argument parsing.

- **Seam 3 (`crates/tmux/tests/tmux_actions_test.rs`, `tests/cli_test.rs`)**:
   - `test_scratchpad_inside_tmux`: Verifies `display_popup("80%", "80%", "tmux new-session -A -s scratch")` invocation.
   - `test_scratchpad_outside_tmux`: Verifies connection/attachment when outside tmux.
   - `test_cli_parse_scratch_default` & `test_cli_parse_scratch_named`: Verifies CLI arguments.

- **Seam 4 (`tests/init_integration_test.rs`)**:
   - `test_cli_parse_init`: Verifies shell targets (`zsh`, `bash`, `fish`, `tmux`) and `--tmux` flag.
   - `test_generate_init_script_zsh_syntax_and_eval`: Verifies `eval "$(omega init zsh)"` executes cleanly in `/bin/zsh`.
   - `test_generate_init_script_bash_syntax`: Verifies Bash syntax validation via `bash -n`.
   - `test_generate_init_script_tmux_keybindings`: Verifies canonical popup keybindings.

- **Seam 5 (`crates/session/tests/window_navigation_test.rs`)**:
   - `test_window_preview_captures_pane`: Verifies `capture_pane` for `<session>:<window>` target identifiers.

---

## 3. Post-Audit Optimizations & Robustness Refinements

Following an exhaustive subagent safety audit spanning spec completeness, performance/zero-copy adherence (`AGENTS.md` Section 5), and runtime edge cases:

1. **Scratchpad Toggle & Name Sanitization**:
   - Added `current_session` and `detach_client` to `TmuxClient`.
   - `omega scratch` now detects if the user is already inside the active scratchpad session; if so, it detaches the client (`tmux detach-client`), toggling the popup off cleanly.
   - Introduced `sanitize_session_name` replacing illegal chars (`.`, `:`, spaces, quotes, shell metachars) with `_`, escaping shell arguments safely in `display-popup`.
2. **Window ID Targeting for Dot/Colon-containing Window Names**:
   - Extended `list_windows_raw` and `parse_tmux_window_candidates` with optional 5th column `#{window_id}` (`@id`), stored in `candidate.alias`.
   - Windows with dots (e.g. `main.rs`, `app.py`) switch via `@id`, avoiding tmux's `.` pane separator collision.
3. **Zero-Allocation Hot-Path Optimizations (`AGENTS.md` Section 5)**:
   - `format_windows_peek`: Replaced `.to_string().len()` with integer digit counter and eliminated intermediate `Vec` allocations and `shown.join(" ")`.
   - `parse_color`: Replaced `collect::<Vec<_>>()` with iterator `.next()` calls for zero intermediate allocations.
   - `generate_dir_preview`: Replaced `"  ".repeat(depth)` string allocations with direct buffer pushes.
   - `harvest_and_sort_candidates`: Skipped redundant second `dedup_candidates` and `apply_sort_order` pass when no custom sources/hooks mutate candidates, cutting duplicate `fs::canonicalize` disk queries on CLI launch.
   - TUI Event Loop: Replaced heap `String` format per keypress with stack-allocated byte buffer lookup.
4. **Window Peek Production Activation**:
   - Populated `candidate.window_names` during tmux harvesting so the `format_windows_peek` ribbon renders active window names for tmux sessions.
5. **Terminal RAII Cleanup Guard**:
   - Introduced `TerminalGuard` implementing `Drop` around `ratatui::restore()` to ensure terminal restoration even upon early error returns or panics.
6. **Scoped Shell Invocations**:
   - Replaced global `exec </dev/tty` in Zsh and Bash widgets with scoped command redirection (`omega </dev/tty>` and `omega window </dev/tty>`).
   - Added `bind-key -n M-o run-shell "omega scratch"` to default tmux keybinding template.
