# Feature Record: Phase 2 Intelligence (Two-Pane TUI & Workspaces)

- **Date**: 2026-10-12
- **Author**: Agent Pair Programmer
- **Scope**: Two-Pane TUI Layout, Asynchronous Debounced Preview, Live ANSI Terminal Parsing, Git Worktree Discovery & Normalization, and `omega clone` Subcommand

---

## 1. What Was Built

Delivered Phase 2 (Intelligence) as specified in `roadmap.md` and `ARCHITECTURE.md`:
1. **Interactive Two-Pane Ratatui TUI**:
   - Split-screen layout: candidate list on the left, live preview pane on the right.
   - Configurable preview width (default 55%, configured via `Config.tui.preview_width`).
   - Responsive auto-collapse when terminal width is below 100 columns (`MIN_PREVIEW_WIDTH_THRESHOLD`), collapsing gracefully to single-pane list mode.
   - Preview toggle (`<C-p>`) and preview vertical scrolling (`<C-u>` up, `<C-d>` down), resetting scroll offset whenever candidate selection changes.
2. **Context-Aware Previews & ANSI SGR Parser**:
   - `omega_session::preview::PreviewGenerator`:
     - Active tmux sessions: Live terminal output capture via `tmux capture-pane -e -p -t <session>`.
     - Git repositories / worktrees: Real-time `git status -s -b` and recent commit log (`git log -n 5 --oneline --decorate`).
     - Plain directories: Formatted compact directory tree listing (up to 3 levels, filtered).
   - Zero-allocation ANSI parser (`src/ansi.rs`):
     - Parses ANSI SGR codes (standard 8 colors, bold, dim, italic, underline, reversed, 256 indexed colors, reset) directly into styled Ratatui `Text` / `Line` / `Span` structs.
3. **First-Class Git Worktree Discovery & Normalization**:
   - Recursive inspection of `.git` file pointers and `.git/worktrees/` administrative structures.
   - Worktree session naming standard: `<repo>--<sanitized-branch>`, sanitizing slashes and tmux-illegal characters (`.` and `:`).
   - Collision resolution incrementing suffix (`-2`, `-3`) if multiple sessions share the base name.
4. **Convenience & Integration Tooling**:
   - `omega clone <repo> [--dir <name>]`: Atomic cloning of Git repositories and immediate connection to a corresponding tmux session.
   - Asynchronous debounced preview worker thread: background thread debounces rapid selection changes (50ms) to ensure continuous 60 FPS in interactive TUI.

---

## 2. Verification & Testing

Developed test-first (TDD) across all 4 confirmed seams:
- **Seam 1 (`crates/session/tests/worktree_test.rs`)**:
  - `test_worktree_session_naming_sanitization`: Verifies `<repo>--<branch>` formatting, character replacement, and collision suffix generation.
  - `test_linked_worktree_discovery_and_metadata`: Verifies discovery of linked git worktrees with `.git` pointer files pointing to main repo admin directory.
- **Seam 2 (`crates/session/tests/preview_test.rs` & `tests/ansi_test.rs`)**:
  - `test_tmux_session_preview`: Verifies live tmux pane capture via `MockTmuxClient`.
  - `test_plain_directory_preview`: Verifies directory tree traversal and truncation.
  - `test_git_repository_preview`: Verifies git status and log extraction in initialized repositories.
  - `test_parse_plain_text`, `test_parse_ansi_colors_and_modifiers`, `test_parse_ansi_256_colors`: Verifies ANSI SGR escape parsing to Ratatui `Text`.
- **Seam 3 (`tests/clone_test.rs`)**:
  - `test_cli_parse_clone`: Verifies CLI arguments parsing for `omega clone <repo>` and optional `--dir`.
  - `test_clone_command_execution`: Verifies end-to-end repository cloning and immediate tmux session attachment.
- **Seam 4 (`tests/tui_preview_test.rs`)**:
  - `test_config_tui_defaults`: Verifies `tui.preview = true`, `tui.preview_width = 55`, `tui.preview_border = "rounded"`.
  - `test_app_preview_state_and_scrolling`: Verifies scroll up, scroll down, toggle, and scroll reset on selection change.
  - `test_ui_responsive_collapse_and_split`: Verifies single-pane collapse below 100 columns and 55% split on wide screens.

Total: 39 unit and integration tests passing. `cargo clippy --all-targets -- -D warnings` clean (0 warnings). `cargo fmt --check` clean.
