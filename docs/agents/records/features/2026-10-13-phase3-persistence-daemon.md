# Feature Record: Phase 3 Persistence & Daemon

- **Date**: 2026-10-13
- **Author**: Agent Pair Programmer
- **Scope**: Tmux Session Snapshots, Atomic Rolling Retention, Layout/Pane Restore, Background Unix Domain Socket Cache Daemon, Periodic 15-min Auto-Save, and Persistence CLI Commands

---

## 1. What Was Built

Delivered Phase 3 (Persistence & Daemon) as specified in `roadmap.md` and `ARCHITECTURE.md`:

1. **Structured JSON Snapshot Storage (`crates/session/src/snapshot.rs`)**:
   - Structured JSON schema storing session names, window names, native tmux window layouts (`#{window_layout}`), pane working directories (`#{pane_current_path}`), and active command contexts (`#{pane_current_command}`).
   - Stored under `$XDG_DATA_HOME/omega/snapshots/` (defaulting to `~/.local/share/omega/snapshots/`).
   - Pure timestamp breakdown utility (`break_down_timestamp`) mapping UTC timestamps into human-readable ISO-like filenames (`snapshot-YYYY-MM-DD-HHMMSS.json`).
   - Named snapshots via `omega save --name <tag>` (saved as `<name>.json`), strictly exempt from rotation.
   - Rolling history rotation keeping the last 10 auto-save snapshots and pruning older auto-saves.
   - Atomic pointer `latest.json` updated on every auto-save to guarantee crash-resilient pointer resolution.

2. **Session State Capture & Layout Fidelity (`crates/session/src/tmux.rs`)**:
   - `SnapshotManager::capture` capturing all running tmux sessions, windows, and pane geometries in structured form via `TmuxClient::capture_state`.
   - Native layout application: restoring multi-pane windows executes `tmux split-window -c <path>` followed by `tmux select-layout <layout>`, reproducing exact layout strings.
   - Initial command restoration with shell argument sanitization: sends captured commands (excluding interactive login shells `zsh`, `bash`, `fish`, `sh`) via `tmux send-keys` into restored panes.

3. **Non-Destructive & Selective Restore (`crates/session/src/tmux.rs`)**:
   - Default non-destructive behavior (`omega restore`): detects already-running tmux sessions and skips them without destroying active user work.
   - Selective restore (`omega restore [session]`): restores only the specified session from the snapshot if present.
   - Force recreation (`omega restore --overwrite` / `-f`): terminates conflicting active sessions prior to rebuild.

4. **Background Cache Daemon (`crates/session/src/daemon.rs`)**:
   - Lightweight Unix domain socket IPC server (`DaemonServer`) listening at `$XDG_RUNTIME_DIR/omega.sock` (or `$TMPDIR/omega-<uid>.sock`).
   - Multi-client worker thread model allowing simultaneous connections without blocking server accept loop.
   - Sub-5ms query response (`DaemonRequest::GetCandidates` / `DaemonResponse::Candidates`).
   - Line-delimited JSON IPC protocol (`Ping`, `Pong`, `GetCandidates`, `SaveSnapshot`, `Stop`).
   - Automated 15-minute background auto-save cycle (`AUTOSAVE_INTERVAL`) managed within the daemon accept loop.
   - Transparent fallback helper `fetch_candidates_or_fallback`: seamlessly queries the running daemon over IPC, falling back immediately to synchronous scanning if the daemon is stopped or unreachable.
   - Daemon lifecycle management functions (`daemon_start`, `daemon_stop`, `daemon_status`).

5. **Headless CLI Subcommands & Interactive Integration (`src/cli.rs`, `src/commands.rs`, `src/main.rs`)**:
   - `omega save [--name <tag>]`: Captures active tmux state and persists to snapshot storage.
   - `omega restore [session] [--name <tag>] [--overwrite|-f]`: Restores sessions from named or latest snapshot.
   - `omega daemon <start|stop|status>`: Starts daemon process in background, stops active daemon, or queries runtime status and uptime. Hidden `serve` command drives daemon process loop.
   - Wired `fetch_candidates_or_fallback` into `omega list` and interactive TUI shell, ensuring sub-5ms launch latency when daemon is active.

---

## 2. Verification & Testing

Implemented test-first following Red-Green-Refactor across 5 isolated test seams:

- **Seam 1 (`crates/session/tests/snapshot_test.rs`)**:
  - `test_break_down_timestamp_known_epochs`: Verifies pure UTC timestamp breakdown against known calendar epochs.
  - `test_snapshot_model_serialization_roundtrip`: Verifies serde JSON serialization/deserialization fidelity.
  - `test_load_missing_snapshot_reports_not_found`: Verifies robust error reporting for missing snapshot files.
  - `test_store_save_named_and_load`: Verifies named snapshots save to `<name>.json` and load cleanly.
  - `test_auto_save_filename_follows_spec_format`: Verifies auto-save filename format (`snapshot-YYYY-MM-DD-HHMMSS.json`).
  - `test_store_auto_save_creates_latest_pointer`: Verifies atomic `latest.json` pointer creation.
  - `test_store_list_snapshots_reports_kinds`: Verifies summary categorization into `Named` vs `Auto` snapshots.
  - `test_store_rotation_keeps_last_ten_and_exempts_named`: Verifies 10-save rolling retention prunes oldest auto-saves while preserving named snapshots.
  - `test_latest_pointer_resolves_to_newest_auto_save`: Verifies resolving `None` tag points to the newest auto-save.

- **Seam 2 (`crates/session/tests/snapshot_capture_test.rs`)**:
  - `test_parse_session_state_empty_input`: Verifies handling of empty tmux input.
  - `test_parse_session_state_skips_malformed_lines`: Verifies resilience against malformed lines.
  - `test_parse_session_state_window_without_panes_is_preserved`: Verifies empty window fallback.
  - `test_parse_session_state_full_tree`: Verifies tree parsing across sessions, windows, layouts, paths, commands.
  - `test_mock_capture_state_returns_fixture`: Verifies `MockTmuxClient` state capture fixture generation.
  - `test_capture_maps_state_tree_into_snapshot`: Verifies `SnapshotManager::capture` generates valid snapshots.

- **Seam 3 (`crates/session/tests/restore_test.rs`)**:
  - `test_restore_non_destructive_skips_running_sessions`: Verifies existing sessions are preserved when overwrite is false.
  - `test_restore_overwrite_recreates_running_sessions`: Verifies existing sessions are killed and recreated when overwrite is true.
  - `test_restore_rebuilds_windows_splits_layouts_and_commands`: Verifies window creation, pane splitting, layout application, and command replay.
  - `test_restore_session_without_windows_creates_bare_session`: Verifies fallback for sessions with no window records.
  - `test_selective_restore_single_session`: Verifies restoring a single named session from a multi-session snapshot.

- **Seam 4 (`crates/session/tests/daemon_test.rs`)**:
  - `test_autosave_due_pure_logic`: Verifies 15-minute interval check logic.
  - `test_daemon_ping_and_get_candidates_latency`: Verifies Unix domain socket ping and candidate retrieval latency (< 5ms).
  - `test_fallback_when_daemon_not_running`: Verifies transparent fallback to direct discovery when daemon is stopped.
  - `test_stale_socket_file_falls_back_cleanly`: Verifies unconnectable stale socket files trigger fallback without error.
  - `test_daemon_graceful_stop_unlinks_socket`: Verifies `DaemonRequest::Stop` causes clean socket unlinking and shutdown.
  - `test_fetch_candidates_prefers_daemon_when_available`: Verifies IPC response is preferred over direct scan when available.

- **Seam 5 (`tests/persistence_cli_test.rs`)**:
  - `test_cli_parse_save_without_name`: Verifies `omega save` argument parsing.
  - `test_cli_parse_save_with_name`: Verifies `omega save --name foo` parsing.
  - `test_cli_parse_restore_defaults`: Verifies `omega restore` parsing with defaults.
  - `test_cli_parse_restore_full_flags`: Verifies `omega restore my-session --name backup -f` parsing.
  - `test_cli_parse_daemon_actions`: Verifies `omega daemon start|stop|status` subcommands.
  - `test_cli_parse_daemon_serve_is_hidden_but_available`: Verifies internal `omega daemon serve` subcommand.

Total: 54 unit and integration tests passing. `cargo clippy --all-targets -- -D warnings` clean (0 warnings). `cargo fmt --check` clean.
