# Architecture Overview

This document serves as the living architectural specification for **Omega**, a high-performance terminal session and workspace navigator written in Rust. It equips agents and human engineers with a comprehensive understanding of system boundaries, data flows, and design invariants.

Update this document whenever architectural changes, new crate boundaries, or structural additions occur.

---

## 1. Project Structure

Omega is designed around a modular Rust workspace architecture. Domain logic and subsystems are partitioned into discrete, decoupled crates within `crates/`, while the root crate (`src/`) serves as the application orchestrator and TUI shell.

```text
omega/
├── crates/                    # Subsystem crates (workspace members)
│   ├── session/               # Session discovery (tmux, git, local directories)
│   ├── search/                # Fuzzy filtering & ranking engine (Skim / Nucleo)
│   └── config/                # Configuration parsing & user preferences
├── src/                       # Application orchestrator & interactive TUI
│   ├── main.rs                # Process initialization & terminal lifecycle
│   ├── app.rs                 # Central application state machine & action dispatch
│   ├── event.rs               # Event polling loop (crossterm keyboard/tick events)
│   └── ui.rs                  # Ratatui rendering engine & layout composition
├── tests/                     # Integration test suites & end-to-end scenarios
├── docs/                      # Authoritative knowledge base
│   ├── adr/                   # Architecture Decision Records
│   └── agents/                # Operational records & agent configurations
│       ├── records/           # Historical logs (features/, fixes/, adrs/)
│       ├── issue-tracker.md   # Issue tracking standards (GitHub / gh CLI)
│       ├── triage-labels.md   # Canonical triage label taxonomy
│       └── domain.md          # Domain documentation consumption rules
├── Cargo.toml                 # Package / Workspace manifest
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
|   ┌───────────────────────────┐          ┌────────────────────────┐   |
|   │    Fuzzy Search Engine    │          │   Session Discovery    │   |
|   │     (crates/search)       │          │    (crates/session)    │   |
|   └───────────────────────────┘          └───────────┬────────────┘   |
+──────────────────────────────────────────────────────┼────────────────+
                                                       │
                                 System Enquiries      │ Handoff / Exec
                                                       ▼
+───────────────────────────────────────────────────────────────────────+
|                         Operating System & CLI                        |
|                                                                       |
|     [ tmux CLI ]           [ git / worktrees ]        [ File System ] |
+───────────────────────────────────────────────────────────────────────+
```

---

## 3. Core Components

### 3.1. Application Orchestrator (`src/`)
- **Primary Responsibility**: Owns the terminal lifecycle, process signal handling, and high-level routing between subsystems.
- **Key Modules**:
  - `main.rs`: Installs panic hooks (`color-eyre`), configures raw mode, invokes session discovery, and drives the main loop.
  - `app.rs`: Holds active state (`items`, `filtered`, `query`, `selected`). Computes query updates.
  - `event.rs`: Asynchronous/polling event channel delivering tick intervals and keypresses.
  - `ui.rs`: Pure rendering function mapping `App` state to Ratatui widgets (`Block`, `List`, `Paragraph`).

### 3.2. Session Discovery (`crates/session` / Module)
- **Primary Responsibility**: Discovers navigable destinations across tmux sessions, git repositories, worktrees, and configured directory roots.
- **Isolation Invariant**: Operates independently of UI concerns. Exposes typed structs (`SessionItem`, `SessionType`) without importing UI or rendering dependencies.
- **Technologies**: `walkdir`, `dirs`, `std::process::Command` (for tmux interactions).

### 3.3. Fuzzy Search Engine (`crates/search` / Module)
- **Primary Responsibility**: High-throughput scoring and indexing of session candidates against user keystrokes.
- **Performance Boundary**: Zero heap allocation in hot search paths. Uses borrowed string slices and cached index lists (`Vec<usize>`).
- **Technologies**: `fuzzy-matcher` (Skim / Nucleo algorithm).

### 3.4. Configuration Subsystem (`crates/config` / Module)
- **Primary Responsibility**: Serializes and validates user configuration (default directories, search excludes, keybindings).
- **Technologies**: Serde, TOML / JSON.

---

## 4. Data Stores & State

### 4.1. In-Memory Session Graph
- **Type**: Transient in-memory store.
- **Purpose**: Holds candidate sessions collected during startup or background re-scans.
- **Key Representation**:
  - `items: Vec<SessionItem>`: Ground truth list of available sessions.
  - `filtered: Vec<usize>`: Dense vector of index pointers into `items`, sorted by score.

### 4.2. Persistent User Configuration
- **Type**: Local file storage (`~/.config/omega/config.toml` or platform equivalent via `dirs`).
- **Purpose**: Persists custom scan paths, pinned sessions, and preference flags.

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

- **Testing Philosophy**: **Test-Driven Development (TDD)**. Write failing tests first (*red*), implement minimal code (*green*), then optimize (*refactor*).
- **Test Invariant**: Existing tests are immutable contracts. Never modify or loosen an existing test to accommodate broken code; only update tests when specifications explicitly change.
- **Tooling**:
  - Build: `cargo build`
  - Tests: `cargo test`
  - Linter: `cargo clippy --all-targets -- -D warnings`
  - Formatter: `cargo fmt --check`

---

## 9. Future Considerations & Roadmap

1. **Formal Workspace Split**: Extract existing modules in `src/` into standalone workspace crates under `crates/` (`crates/session`, `crates/search`, `crates/config`).
2. **Background Async Scanner**: Offload filesystem walks to background worker threads or Rayon iterators to ensure zero frame drops during initial launch.
3. **Daemon / Persistent Cache**: Support an optional lightweight background cache for instantaneous session resolution on large file trees.

---

## 10. Project Identification

- **Project Name**: Omega
- **Repository**: `kiyors/omega`
- **Primary Contact**: Gaurav (`gaurav23617@gmail.com`)
- **Date of Last Update**: 2026-10-12

---

## 11. Glossary / Acronyms

- **TUI**: Terminal User Interface.
- **Ratatui**: Rust library for building terminal user interfaces, maintained community fork of tui-rs.
- **Crossterm**: Cross-platform terminal manipulation library providing event input and raw mode management.
- **Worktree**: Linked working tree in Git allowing multiple branches checked out simultaneously in separate directories.
- **LTO**: Link-Time Optimization, whole-program optimization performed at link phase.
- **TDD**: Test-Driven Development (Red-Green-Refactor).
- **Zero-Copy**: Data processing technique avoiding heap allocations by referencing existing buffers via borrowed slices.
