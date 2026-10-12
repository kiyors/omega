# Feature Record: Phase 1 Foundation Engine & CLI

- **Date**: 2026-10-12
- **Author**: Agent Pair Programmer
- **Scope**: Core Engine, Crate Architecture, CLI Subcommands, Single-Pane TUI Picker

---

## 1. What Was Built

Delivered Phase 1 (Foundation) as specified in `roadmap.md` and `ARCHITECTURE.md`:
1. **Workspace & Crate Partitioning**:
   - `crates/config` (`omega-config`): Static root directories, alias definitions, and user settings.
   - `crates/search` (`omega-search`): Zero-copy fuzzy matching engine using `fuzzy-matcher` Skim algorithm.
   - `crates/session` (`omega-session`): Multi-source candidate discovery (`tmux`, `git`, `zoxide`, `config`, `alias`), canonical path deduplication, naming pipeline with tmux sanitization/collision resolution, and multiplexer client abstraction.
   - `src/` (`omega`): Central orchestrator parsing CLI commands (`clap`) and driving the single-pane interactive Ratatui picker with interactive search sigils (`:t `, `:g `, `@`).
2. **Canonical Deduplication**:
   - Priority hierarchy: `tmux` > `alias` > `config` > `git` > `zoxide`.
   - Real-path canonicalization (`std::fs::canonicalize`) with fallback to session name.
   - Metadata inheritance preserving alias labels and custom icons on winning candidates.
   - Attached session hiding (`hide_attached = true`).
3. **Core CLI Subcommands**:
   - `omega connect <target> [-c <cmd>]`: Auto-switches inside tmux, attaches outside, creates session if missing.
   - `omega list [--tmux] [--git] [--zoxide] [--json] [--format <fmt>]`: Headless listing.
   - `omega kill <target> [-f]`: Session termination.
   - `omega last`: Quick bounce to previously attached session.
   - `omega root`: Workspace / Git worktree root detection.
   - `omega mkdir <path> [-c <cmd>]`: Atomic directory creation and immediate connection.
4. **Interactive Single-Pane TUI Picker**:
   - Fast interactive filtering, navigation (`<Up>`, `<Down>`, `<C-p>`, `<C-n>`), and connection (`<Enter>`).

---

## 2. Verification & Testing

Tested test-first (TDD) across all pre-agreed seams:
- `crates/session/tests/dedup_test.rs`: Priority hierarchy, symlink resolution, alias metadata inheritance, attached session exclusion.
- `crates/session/tests/discovery_sources_test.rs`: Parsers for `tmux list-sessions`, `zoxide query --list --score`, filesystem `.git` detection (directories and worktree file pointers), and static config scanning.
- `crates/session/tests/naming_test.rs`: Path segment extraction (`dir_length`), character sanitization (`.`/`:` → `_`), and collision suffix incrementation (`-2`, `-3`).
- `crates/session/tests/tmux_actions_test.rs`: Multiplexer client detection, automatic session creation, client switching, and detachment.
- `crates/session/tests/bench_test.rs`: 1,000 candidate deduplication verified under 30ms (measured at < 1ms).
- `crates/search/tests/search_test.rs`: Fuzzy scoring, exact match precedence, empty query handling.
- `tests/cli_test.rs`: Clap argument parsing for all subcommands.

Total: 28 unit and integration tests passing. Clean `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.
