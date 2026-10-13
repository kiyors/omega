# Architecture Overview

This document serves as the living architectural specification for **Omega**, a high-performance terminal session and workspace navigator written in Rust. It equips agents and human engineers with a comprehensive understanding of system boundaries, data flows, and design invariants.

Update this document whenever architectural changes, new crate boundaries, or structural additions occur.

---

## 1. Project Structure

Omega is designed around a modular Rust workspace architecture. Domain logic and subsystems are partitioned into discrete, decoupled crates within `crates/`, while the root crate (`src/`) serves as the application orchestrator and TUI shell.

```text
omega/
├── crates/                    # Subsystem crates (workspace members)
│   ├── config/                # Configuration parsing & user preferences
│   ├── search/                # Fuzzy filtering & ranking engine (Skim / Nucleo)
│   ├── tmux/                  # Low-level tmux client, layout parsing, actions & mock harnesses
│   ├── session/               # Pure domain session discovery, naming, dedup & preview generation
│   └── daemon/                # Background IPC daemon, candidate caching worker, snapshot store & focus restore
├── src/                       # Application orchestrator & interactive TUI
│   ├── main.rs                # Process initialization & terminal lifecycle
│   ├── app.rs                 # Central application state machine & action dispatch
│   ├── ansi.rs                # Zero-allocation ANSI SGR escape code parser to Ratatui Text
│   ├── cli.rs                 # Clap CLI parsing & command definitions
│   ├── commands.rs            # Headless CLI command execution handlers (connect, clone, etc.)
│   ├── event.rs               # Event polling loop (crossterm keyboard/tick events)
│   └── ui.rs                  # Two-pane Ratatui rendering engine & responsive layout composition
├── tests/                     # Integration test suites & end-to-end scenarios
├── nix/                       # Nix packaging (package.nix) & development shell (devshell.nix)
├── Formula/                   # Homebrew tap formula (omega.rb)
├── docs/                      # Authoritative knowledge base
│   ├── adr/                   # Architecture Decision Records
│   └── agents/                # Operational records & agent configurations
│       ├── records/           # Historical logs (features/, fixes/, adrs/)
│       ├── issue-tracker.md   # Issue tracking standards (GitHub / gh CLI)
│       ├── triage-labels.md   # Canonical triage label taxonomy
│       └── domain.md          # Domain documentation consumption rules
├── Cargo.toml                 # Package / Workspace manifest
├── rust-toolchain.toml        # Locked Rust toolchain channel (1.99.0)
├── flake.nix                  # Minimal root Nix flake delegating to nix/
├── AGENTS.md                  # Master agent operating directives
└── ARCHITECTURE.md            # This living document
```

---

## 2. High-Level System Diagram

Omega operates as an event-driven terminal application that reads active environment states, filters candidate sessions via fuzzy matching, and dispatches terminal handoffs:

```text
+-----------------------------------------------------------------------+
|                             Terminal User                             |
+-----------------------------------------------------------------------+
                                  │  Keypresses & Ticks
                                  ▼
+───────────────────────────────────────────────────────────────────────+
|                           Omega TUI Shell                             |
|                                                                       |
|   ┌───────────────────┐    Draw Frame    ┌────────────────────────┐   |
|   │   Event Handler   │ ───────────────► │    UI Renderer (ui)    │   |
|   │    (crossterm)    │                  │       (ratatui)        │   |
|   └─────────┬─────────┘                  └───────────▲────────────┘   |
|             │                                        │                |
|             ▼ Dispatch Event                         │ State Snapshot |
|   ┌──────────────────────────────────────────────────┴────────────┐   |
|   │                       App State Machine                       │   |
|   │           (query, selected index, filtered indices)           │   |
|   └─────────────────────────┬──────────────────▲──────────────────┘   |
+─────────────────────────────┼──────────────────┼──────────────────────+
                              │ Query            │ Filtered Matches
                              ▼                  │
+────────────────────────────────────────────────┴──────────────────────+
|                           Core Subsystems                             |
|                                                                       |
|   ┌───────────────────┐  ┌──────────────────┐  ┌──────────────────┐   |
|   │   Search Engine   │  │ Session Engine   │  │   Daemon & IPC   │   |
|   │  (crates/search)  │  │ (crates/session) │  │ (crates/daemon)  │   |
|   └───────────────────┘  └────────┬─────────┘  └────────┬─────────┘   |
+───────────────────────────────────┼─────────────────────┼─────────────+
                                    │                     │
                                    │ Tmux Client Calls   │ State Sync
                                    ▼                     ▼
+───────────────────────────────────────────────────────────────────────+
|                           Tmux Subsystem                              |
|                           (crates/tmux)                               |
+───────────────────────────────────┬───────────────────────────────────+
                                    │
                                    ▼ Multiplexer Exec / OS Handoff
+───────────────────────────────────────────────────────────────────────+
|                         Operating System & CLI                        |
|                                                                       |
|     [ tmux CLI ]           [ git / worktrees ]        [ File System ] |
+───────────────────────────────────────────────────────────────────────+
```

---

## 3. Core Components

### 3.1. Application Orchestrator (`src/`)

- **Primary Responsibility**: Owns the terminal lifecycle, process signal handling, high-level routing, and interactive two-pane TUI shell.
- **Key Modules**:
  - `main.rs`: Installs panic hooks (`color-eyre`), configures raw mode, drives the main event loop, and coordinates the asynchronous debounced preview worker thread.
  - `app.rs`: Holds active state (`items`, `filtered`, `query`, `selected`, `preview_enabled`, `preview_scroll`, `preview_content`). Computes query updates and preview scrolling.
  - `ansi.rs`: Zero-allocation parser translating ANSI SGR color/style escape codes into styled Ratatui `Text` spans.
  - `cli.rs`: Clap CLI parser declaring subcommands (`connect`, `list`, `kill`, `last`, `root`, `mkdir`, `clone`, `save`, `restore`, `daemon`, `doctor`, `completions`).
  - `commands.rs`: Headless CLI command execution handlers (`connect`, `clone`, `save`, `restore`, `daemon`, `doctor`, `completions`, etc.).
  - `doctor.rs`: System dependency diagnostic engine validating `tmux`, `git`, `zoxide`, daemon socket health, and `init.lua` validity.
  - `event.rs`: Asynchronous/polling event channel delivering tick intervals and keypresses.
  - `ui.rs`: Two-pane Ratatui layout engine: candidate list on the left, live debounced preview on the right (55% width, auto-collapsing below 100 columns).

### 3.2. Tmux Multiplexer Subsystem (`crates/tmux` / `omega-tmux`)

- **Primary Responsibility**: Owns the low-level Tmux command abstraction, raw tmux output parsing, native layout management, action dispatch (`connect`, `kill`, `last`), and test mock client implementations.
- **Key Types & Functions**:
  - `TmuxClient`: Core trait abstracting tmux interactions (`create_session`, `create_window`, `split_pane`, `apply_layout`, `select_window`, `select_pane`, `switch_client`, `attach_session`, `kill_session`, `capture_pane`, `capture_state`).
  - `RealTmuxClient`: Zero-cost production multiplexer client invoking `std::process::Command` without shell escaping vulnerabilities. Includes resilient `tiled` layout fallback.
  - `MockTmuxClient`: Deterministic mock implementation recording session creation, window splitting, layout application, and active selections for unit testing.
  - `parse_session_state`: Parser transforming tab-separated `list-sessions`, `list-windows`, and `list-panes` streams into structured hierarchical state models (`TmuxSessionState`, `TmuxWindowState`, `TmuxPaneState`).
  - `TmuxActionRunner`: Coordinates high-level connection handoffs, switching inside tmux vs. attaching outside tmux.

### 3.3. Session Discovery & Domain Engine (`crates/session` / `omega-session`)

- **Primary Responsibility**: Discovers navigable destinations across tmux, git repositories, linked worktrees, zoxide, and configuration aliases; provides canonical naming, deduplication, and context-aware preview text. 100% OS-agnostic (zero C/POSIX dependencies).
- **Key Modules**:
  - `discovery.rs`: Multi-source candidate discovery and recursive `.git` worktree pointer/admin inspection.
  - `naming.rs`: Deterministic tmux session name resolution, character sanitization, and `<repo>--<sanitized-branch>` worktree normalization.
  - `collector.rs`: High-performance deduplication (`SessionCollector`) merging candidates across sources, prioritizing active tmux sessions while inheriting user aliases and icons.
  - `preview.rs`: Context-aware preview provider (`PreviewGenerator`) extracting live `tmux capture-pane`, `git status -s` + recent commit log, or directory tree listings.
- **Isolation Invariant**: Operates independently of platform-specific Unix IPC or signal handling.

### 3.4. Persistence & Cache Daemon Subsystem (`crates/daemon` / `omega-daemon`)

- **Primary Responsibility**: Manages snapshot serialization, rolling retention, focus restoration, and the Unix domain socket background cache daemon.
- **Key Modules**:
  - `snapshot.rs`: Structured JSON snapshot serialization/deserialization (`Snapshot`), atomic rolling history retention (keeps last 10 auto-saves with atomic `latest.json` pointer), named snapshot persistence, and focus-preserving session reconstruction (`SnapshotManager`).
  - `daemon.rs`: Lightweight Unix domain socket background cache daemon (`DaemonServer`) with a decoupled background scanning thread for sub-millisecond socket responses, client transport (`DaemonClient`), signal handlers, and cross-platform non-Unix fallback stubs for clean compilation across targets.

### 3.5. Fuzzy Search Engine (`crates/search` / `omega-search`)

- **Primary Responsibility**: High-throughput scoring and indexing of session candidates against user keystrokes.
- **Performance Boundary**: Zero heap allocation in hot search paths. Uses borrowed string slices and cached index lists (`Vec<usize>`).
- **Technologies**: `fuzzy-matcher` (Skim / Nucleo algorithm).

### 3.6. Configuration Subsystem (`crates/config` / `omega-config`)

- **Primary Responsibility**: Hosts the embedded Lua 5.4 engine (`mlua = "0.12"`), parses declarative `omega.setup` configuration, manages module path resolution (`lua/`), auto-loads plugins (`plugins/`), and exposes lifecycle event and naming hooks.
- **Technologies**: `mlua` 0.12 (Lua 5.4, vendored, serde), Serde.

---

## 4. Data Stores & State

### 4.1. In-Memory Session Graph

- **Type**: Transient in-memory store.
- **Purpose**: Holds candidate sessions collected during startup or background re-scans.
- **Key Representation**:
  - `items: Vec<SessionItem>`: Ground truth list of available sessions.
  - `filtered: Vec<usize>`: Dense vector of index pointers into `items`, sorted by score.

### 4.2. Persistent User Configuration

- **Type**: Local file storage (`~/.config/omega/init.lua`, modular `lua/`, and `plugins/`, with JSON fallback).
- **Purpose**: Persists declarative settings, keybindings, icons, path substitutions, aliases with startup layouts, and directory wildcard rules.

### 4.3. Session Snapshots & Persistence Store

- **Type**: JSON filesystem storage (`$XDG_DATA_HOME/omega/snapshots/`, defaulting to `~/.local/share/omega/snapshots/`).
- **Purpose**: Persists layout fidelity, window configurations, working directories, and active commands across reboots. Retains rolling 10 auto-saves (`snapshot-YYYY-MM-DD-HHMMSS*.json`) via an atomic `latest.json` pointer alongside user-named snapshot tags.

### 4.4. Background IPC Domain Socket Cache

- **Type**: Unix domain socket (`$XDG_RUNTIME_DIR/omega.sock` or `$TMPDIR/omega-<uid>.sock`).
- **Purpose**: Background daemon serving pre-scanned session candidates in < 5ms over line-delimited JSON IPC, with transparent direct-scan fallback when stopped.

---

## 5. External Integrations / APIs

### 5.1. Tmux Multiplexer

- **Purpose**: Query existing sessions (`tmux list-sessions -F ...`), create sessions (`tmux new-session -ds <name> -c <path>`), and switch/attach (`tmux switch-client` / `attach-session`).
- **Integration Method**: Structured execution via `std::process::Command` with explicit arguments (no raw shell string execution).

### 5.2. Git & Worktrees

- **Purpose**: Detect repository roots, worktree checkouts, and bare repository layouts.
- **Integration Method**: Direct filesystem inspection (`.git` directories, `.git` pointer files) complemented by `git worktree list --porcelain` where necessary.

---

## 6. Deployment & Infrastructure

- **Toolchain**: Rust stable (Edition 2024), Cargo, Nix Flake (`flake.nix`).
- **Build Optimizations**:
  - Link-Time Optimization (`lto = true`).
  - Single Codegen Unit (`codegen-units = 1`).
  - Size and speed optimization (`opt-level = "s"` or `"3"`).
  - Symbol stripping (`strip = true`).
- **Distribution**: Standalone binary compiled for Linux and macOS.

---

## 7. Security Considerations

- **Process Spawning**: Always pass arguments as discrete slices to `std::process::Command`. Never interpolate user query input into a shell (`sh -c`).
- **Path Sanitization**: Validate target session paths against symlink loops and directory traversal before handing off execution.
- **Environment Isolation**: Strip untrusted environment variables before launching nested terminal processes.

---

## 8. Development & Testing Environment

- **Testing Philosophy**: **Test-Driven Development (TDD)**. Write failing tests first (_red_), implement minimal code (_green_), then optimize (_refactor_).
- **Test Invariant**: Existing tests are immutable contracts. Never modify or loosen an existing test to accommodate broken code; only update tests when specifications explicitly change.
- **Tooling**:
  - Build: `cargo build`
  - Tests: `cargo test`
  - Linter: `cargo clippy --all-targets -- -D warnings`
  - Formatter: `cargo fmt --check`

---

## 9. Future Considerations & Roadmap

1. **Formal Workspace Split**: Completed. Partitioned into workspace member crates `crates/session`, `crates/search`, and `crates/config`.
2. **Background Async Scanner**: Offload filesystem walks to background worker threads or Rayon iterators to ensure zero frame drops during initial launch.
3. **Daemon / Persistent Cache**: Completed in Phase 3. Background Unix domain socket cache daemon with 15-minute rolling snapshot persistence, layout fidelity reconstruction, and non-destructive restore.
4. **Extensibility & Polish**: Completed in Phase 4. Embedded Lua 5.4 configuration engine (`init.lua`), plugins and hooks, startup window/pane layouts, wildcard matching, `omega doctor`, shell completions, Homebrew formula, and Nix flake package outputs.

---

## 10. Project Identification

- **Project Name**: Omega
- **Repository**: `kiyors/omega`
- **Primary Contact**: Gaurav (`gaurav23617@gmail.com`)
- **Date of Last Update**: 2026-10-13

---

## 11. Glossary / Acronyms

- **TUI**: Terminal User Interface.
- **Ratatui**: Rust library for building terminal user interfaces, maintained community fork of tui-rs.
- **Crossterm**: Cross-platform terminal manipulation library providing event input and raw mode management.
- **Worktree**: Linked working tree in Git allowing multiple branches checked out simultaneously in separate directories.
- **LTO**: Link-Time Optimization, whole-program optimization performed at link phase.
- **TDD**: Test-Driven Development (Red-Green-Refactor).
- **Zero-Copy**: Data processing technique avoiding heap allocations by referencing existing buffers via borrowed slices.
