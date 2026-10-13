# Omega: Architecture Specification & Development Roadmap

Omega is a high-performance terminal session and workspace navigator written in Rust. It consolidates tmux session management, directory jumping, workspace layouts, state persistence, and interactive fuzzy filtering into a single, cohesive binary with Lua-based configuration.

---

## 1. Vision & Core Objectives

### The Problem: Configuration Sprawl

Modern terminal power-user workflows frequently require stitching together multiple disjoint tools and plugins:

- Shell aliases and functions in `.zshrc` / `.bashrc`
- Session managers and multiplexer keybindings in `.tmux.conf`
- Tmux plugin managers (TPM) running plugins like `tmux-resurrect` and `tmux-continuum`
- Workspace layout tools such as `tmuxinator` or `tmuxp`
- External fuzzy finders (`fzf`, `gum`, `sk`) with custom wrapper scripts

This produces fragile shell configurations, cross-tool synchronization latency, and high maintenance overhead.

### The Omega Solution

Omega delivers an integrated solution:

1. **Single Self-Contained Binary**: Combines workspace discovery, interactive terminal UI (Ratatui), fuzzy scoring, layout generation, and state restoration into one Rust executable.
2. **Zero-Config Defaults with Lua Extensibility**: Operates immediately out-of-the-box with sensible conventions, while providing deep, Neovim-style Lua 5.4 configuration (`init.lua`) and plugin APIs.
3. **Hybrid CLI and TUI Engine**: Exposes a full headless CLI for scripting and window manager shortcuts, alongside an interactive, two-pane terminal picker with live terminal and git previews.
4. **Multiplexer State Persistence**: Natively snapshots and resurrects tmux session geometries, pane hierarchies, and working directories across reboots, completely eliminating external persistence plugins.
5. **Instantaneous Response Times**: Employs an optional lightweight background Unix socket daemon maintaining a hot in-memory candidate cache, guaranteeing sub-millisecond query latency on massive codebases.

---

## 2. System Architecture & Boundaries

```text
+---------------------------------------------------------------------------------+
|                                    User Input                                   |
|               CLI Subcommands (omega ...)  |  Interactive TUI                   |
+---------------------------------------------------------------------------------+
                                       │
                                       ▼
+─────────────────────────────────────────────────────────────────────────────────+
|                                 Omega Core App                                  |
|                                                                                 |
|   ┌───────────────────────────┐         ┌───────────────────────────────────┐   |
|   │     CLI Command Parser    │         │       Ratatui TUI Shell           │   |
|   │          (clap)           │         │  (2-pane: fuzzy list + preview)   │   |
|   └─────────────┬─────────────┘         └─────────────────┬─────────────────┘   |
|                 │                                         │                     |
|                 └────────────────────┬────────────────────┘                     |
|                                      ▼                                          |
|   ┌─────────────────────────────────────────────────────────────────────────┐   |
|   │                       Candidate Discovery & Dedup                       │   |
|   │    Canonical Path Resolution | Hierarchy: tmux > alias > config > git   │   |
|   └──────────────────────────────────┬──────────────────────────────────────┘   |
|                                      │                                          |
|         ┌────────────────────────────┼────────────────────────────┐             |
|         ▼                            ▼                            ▼             |
|   ┌───────────┐                ┌───────────┐                ┌───────────┐       |
|   │ Lua Engine│                │Daemon IPC │                │Persistence│       |
|   │  (mlua)   │                │(Unix Sock)│                │(JSON Store│       |
|   └─────┬─────┘                └─────┬─────┘                └─────┬─────┘       |
+─────────┼────────────────────────────┼────────────────────────────┼─────────────+
          │                            │                            │
          ▼                            ▼                            ▼
+─────────────────────────────────────────────────────────────────────────────────+
|                              Operating System & Host                            |
|        [ tmux CLI ]           [ Git Repos & Worktrees ]        [ Filesystem ]   |
+─────────────────────────────────────────────────────────────────────────────────+
```

### Component Boundaries

- **Orchestrator (`src/`)**: Owns process initialization, terminal lifecycle, signal handling, and CLI command dispatch.
- **Session Discovery (`crates/session`)**: Identifies candidates across tmux, git repositories, linked worktrees, zoxide frecency, and user configuration. Operates independently of any UI library.
- **Fuzzy Search Engine (`crates/search`)**: Zero-heap-allocation fuzzy matcher utilizing borrowed string slices and Nucleo/Skim algorithms.
- **Configuration Subsystem (`crates/config`)**: Manages the embedded Lua 5.4 runtime (`mlua`), parses `init.lua`, handles Serde deserialization, and dispatches lifecycle events.

---

## 3. Session Sources & Deduplication

Omega aggregates workspace targets across five distinct sources:

| Source               | Identifier | Description                                      | Priority Tier |
| -------------------- | ---------- | ------------------------------------------------ | ------------- |
| **Active Tmux**      | `tmux`     | Currently running tmux sessions and windows      | 1 (Highest)   |
| **Aliases**          | `alias`    | Configured named bookmarks with startup behavior | 2             |
| **Configured Roots** | `config`   | Explicitly scanned directories and project roots | 3             |
| **Git Repositories** | `git`      | Git repository roots and linked Git worktrees    | 4             |
| **Frecency**         | `zoxide`   | Recently and frequently accessed directories     | 5 (Lowest)    |

### Deduplication Policy

- **Identity Key**: All candidates backed by filesystem directories are resolved to their **canonical filesystem path** (`canonicalize()`), resolving symbolic links and relative segments. Non-directory tmux sessions fall back to their session name.
- **Shadowing & Metadata Inheritance**: When the same canonical path appears across multiple sources (e.g. an active tmux session that is also in zoxide and has an alias), the higher-priority source wins. However, the resulting candidate retains any associated alias tags, custom icons, and search tokens.
- **Attached Session Handling**: When launched from inside an active tmux session, the currently attached session is excluded from the interactive picker by default (`hide_attached = true`), automatically placing the selection cursor on the previous session for 1-keystroke bouncing.

### Interactive Source Sigils

In the interactive picker, users can filter candidates across all sources or isolate specific providers using prefix sigils:

- `@<query>`: Restricts search strictly to configured **aliases**.
- `:t <query>`: Restricts search strictly to **active tmux sessions**.
- `:w <query>`: Restricts search strictly to **active tmux windows** across sessions.
- `:g <query>`: Restricts search strictly to **git repositories and worktrees**.

---

## 4. Session Naming & Path Resolution

To prevent session name collisions and enforce tmux-compliant naming, Omega passes all target paths through an evaluation pipeline:

```text
Target Path / Candidate
   │
   ├── 1. Explicit Alias (if connection originated from an alias)
   │
   ├── 2. Custom Lua Naming Hook (omega.session_name(path, git_context))
   │
   ├── 3. Regex Substitutions (name_substitutions configured in Lua)
   │
   ├── 4. Built-in Strategy:
   │      - Git Worktree: <repo>--<sanitized-branch>
   │      - Git Repository: Repository root folder name
   │      - Plain Directory: Last dir_length segments (default: 1)
   │
   ├── 5. Tmux Sanitization:
   │      Replace illegal characters ('.' and ':') with '_' or '-'
   │
   └── 6. Collision Resolution:
          If session name is in use by another directory, append '-2', '-3', etc.
```

---

## 5. State Persistence (Save & Restore)

Omega replaces `tmux-resurrect` and `tmux-continuum` with a native snapshot and resurrection engine.

### Snapshot Architecture

- **Format**: Structured, human-readable JSON stored in `$XDG_DATA_HOME/omega/snapshots/` (default: `~/.local/share/omega/snapshots/`).
- **Geometric Fidelity**: Window hierarchies and pane split proportions are captured via native checksummed tmux layout strings (`#{window_layout}`) rather than approximations.
- **Context Preservation**: Each pane records its active working directory (`#{pane_current_path}`) and process directory context.
- **History Rotation**: Auto-saves are recorded with timestamped filenames (`snapshot-YYYY-MM-DD-HHMMSS.json`). Omega maintains a rolling window of the **last 10 auto-saves** alongside an atomic pointer symlink `latest.json`. Explicitly named snapshots (`omega save --name <tag>`) are exempt from rotation and preserved indefinitely.

### Restoration Rules

- **Non-Destructive by Default**: `omega restore` detects sessions already running in the live tmux server and skips them to protect uncommitted in-memory work. An `--overwrite` / `-f` flag is available to force recreation.
- **Selective Restoration**: Running `omega restore [session-name]` resurrects only the specified session from the snapshot without disturbing other running or missing sessions.
- **Lifecycle Triggers**: Snapshots are generated via:
  1. Background periodic timer (daemon-driven, default: 15 minutes).
  2. Tmux server exit hooks (clean shutdown / detach).
  3. Manual CLI invocation (`omega save`).

---

## 6. Configuration & Lua API Design

Omega utilizes an embedded **Lua 5.4** runtime (`mlua = "0.12"` with `vendored`, `lua54`, and `serde` features).

### File System Layout

```text
~/.config/omega/
├── init.lua           # Primary entrypoint configuration
├── lua/               # User and custom modules (auto-added to package.path)
│   └── my_config.lua
└── plugins/           # Auto-loaded plugin scripts (*.lua executed on launch)
    └── statusline.lua
```

### Declarative API (`omega.setup`)

A single nested table handles complete declarative configuration:

```lua
-- ~/.config/omega/init.lua
local omega = require("omega")

omega.setup({
  -- Source ordering & grouping
  sort_order = {
    "tmux",
    "alias",
    { "config", "git", "zoxide" }, -- Interleaved by frecency score
  },

  -- Behavior
  hide_attached = true,
  dir_length = 1,

  -- Interactive TUI settings
  tui = {
    preview = true,
    preview_width = 55, -- Percentage (10 to 90)
    preview_border = "rounded", -- "rounded", "line", "double", "none"
    theme = "catppuccin-mocha", -- Built-in preset ("catppuccin-mocha", "tokyonight", etc.) or custom theme table
  },

  -- Custom Nerd Font or Emoji icons
  icons = {
    tmux = "",
    git = "",
    zoxide = "",
    alias = "󰌹",
    config = "",
  },

  -- Regex path substitutions for session naming
  name_substitutions = {
    { find = "^/Users/[^/]+/personal/", replace = "" },
    { find = "projects/", replace = "p-" },
  },

  -- Named bookmarks with startup layouts
  aliases = {
    {
      name = "dotfiles",
      path = "~/.dotfiles",
      icon = "󱌣",
      windows = {
        { name = "config", command = "nvim" },
      },
    },
    {
      name = "backend",
      path = "~/projects/api",
      windows = {
        { name = "editor", command = "nvim" },
        {
          name = "server",
          layout = "main-vertical",
          panes = {
            { command = "cargo watch -x run" },
            { command = "docker compose logs -f" },
          },
        },
      },
    },
  },

  -- Wildcard rules matching directory trees
  wildcards = {
    {
      pattern = "~/projects/rust/*",
      startup_command = "cargo check",
      icon = "🦀",
    },
    {
      pattern = "~/work/**",
      icon = "💼",
    },
  },

  -- Keybindings
  keymaps = {
    ["<C-j>"] = "select_next",
    ["<C-k>"] = "select_prev",
    ["<C-u>"] = "preview_scroll_up",
    ["<C-d>"] = "preview_scroll_down",
    ["<C-p>"] = "toggle_preview",
    ["<C-x>"] = "kill_session",
    ["<CR>"]  = "connect",
  },
})
```

### Programmatic & Plugin APIs

Power users and plugin authors can attach callbacks and extend sources imperatively:

```lua
-- Custom naming hook
omega.session_name(function(path, git_context)
  if git_context then
    return git_context.repo .. "@" .. git_context.branch
  end
  return nil -- Fall back to default pipeline
end)

-- Theme configuration
omega.theme("catppuccin-mocha")

-- Lifecycle event hooks
omega.on("session_created", function(session)
  print("Initialized session: " .. session.name)
end)

omega.on("session_switched", function(session)
  -- e.g. notify external window managers or log activity
end)

-- Custom dynamic session source registration
omega.register_source("docker", function()
  -- Return dynamic table of candidate targets
  return {
    { name = "docker:redis", path = "/tmp/docker/redis", icon = "󰡨" },
    { name = "docker:postgres", path = "/tmp/docker/postgres", icon = "󰡨" },
  }
end)
```

---

## 7. Command-Line Interface (CLI) Inventory

Omega exposes a comprehensive set of subcommands for direct shell execution, keybinding integration, and pipeline automation:

| Command         | Arguments / Flags                                              | Description                                                           |
| --------------- | -------------------------------------------------------------- | --------------------------------------------------------------------- |
| `omega`         | `[query]`                                                      | Launches the interactive two-pane TUI picker                          |
| `omega window`  | `[query]`                                                      | Launches interactive picker filtering across active tmux windows      |
| `omega scratch` | `[--name <tag>]`                                               | Toggles a persistent floating scratchpad session inside a tmux popup  |
| `omega connect` | `<name\|path\|alias>` `[-s\|--switch]` `[-c\|--command <cmd>]` | Smart connects: switches client if inside tmux, attaches if outside   |
| `omega list`    | `[--tmux] [--git] [--zoxide] [--json] [--format <fmt>]`        | Lists candidate targets with optional JSON or templated formatting    |
| `omega kill`    | `<name>` `[-f\|--force]`                                       | Terminates an active tmux session or removes a frecency path          |
| `omega rename`  | `[old-name] <new-name>`                                        | Renames an active session                                             |
| `omega last`    | —                                                              | Bounces client to the previously attached session                     |
| `omega root`    | —                                                              | Prints root path of current workspace or Git worktree                 |
| `omega clone`   | `<repo-url>` `[--dir <name>]`                                  | Clones a repository and immediately opens a session inside it         |
| `omega mkdir`   | `<path>` `[-c\|--command <cmd>]`                               | Atomic `mkdir -p` followed by immediate session creation              |
| `omega save`    | `[--name <tag>]`                                               | Captures a snapshot of all active tmux sessions and layouts           |
| `omega restore` | `[session-name] [--name <tag>] [-f\|--overwrite]`              | Resurrects sessions from the latest or specified snapshot             |
| `omega daemon`  | `start \| stop \| status`                                      | Manages the background Unix domain socket cache daemon                |
| `omega init`    | `<zsh\|bash\|fish>`                                            | Emits clean shell integration functions, ZLE widgets, and keybindings |
| `omega doctor`  | —                                                              | Diagnoses system dependencies (tmux, git, zoxide, socket, config)     |

---

## 8. Phased Development Roadmap

The development of Omega is structured into five sequential, verifiable milestones. Each phase delivers a functional, tested deliverable before moving forward.

```text
Phase 1: Foundation (Core Engine & CLI)
   │  (tmux, git, zoxide discovery, deduplication, CLI commands, single-pane TUI)
   ▼
Phase 2: Intelligence (TUI & Workspaces)
   │  (2-pane Ratatui TUI, live debounced preview, worktrees, formatting, sigils)
   ▼
Phase 3: Persistence & Daemon (Speed & Resiliency)
   │  (save/restore, native layout strings, rolling history, Unix socket daemon)
   ▼
Phase 4: Extensibility & Polish (Lua & Ecosystem)
   │  (mlua 5.4 engine, init.lua, plugin hooks, omega doctor, shell completions)
   ▼
Phase 5: Multiplexer Mastery & Workflow Consolidation
      (native window picker, persistent scratchpad, shell & tmux init, theme engine)
```

---

### Phase 1: Foundation (Core Engine & CLI)

_Goal: Build the fundamental workspace discovery and session switching engine with standard CLI commands and basic interactive selection._

#### Capabilities & Deliverables

1. **Multi-Source Discovery Engine**:
   - `tmux` active session and window listing (`tmux list-sessions`, `tmux list-windows`).
   - `git` repository root discovery via direct filesystem inspection (`.git` directories/pointers).
   - `zoxide` frecency discovery reading output of `zoxide query --list --score`.
   - `config` static directory scanning (`walkdir`) and static alias mapping.
2. **Canonical Deduplication**:
   - Real-path canonicalization (`std::fs::canonicalize`).
   - Priority resolution hierarchy: `tmux` > `alias` > `config` > `git` > `zoxide`.
3. **Core CLI Subcommands**:
   - `omega connect <target>`: Multiplexer client detection (auto-switch inside tmux, attach outside).
   - `omega list`: Clean text listing of available workspaces.
   - `omega kill <target>`: Session termination via `tmux kill-session`.
   - `omega last`: Instant bounce to second most recently attached session.
   - `omega root`: Top-level directory printout.
   - `omega mkdir <path>`: Atomic folder creation and connection.
4. **Basic TUI Picker**:
   - Single-pane interactive fuzzy finder using `ratatui` and `crossterm`.
   - Fast scoring via `fuzzy-matcher` (Skim algorithm).

#### Verification Criteria

- `cargo test -p omega-session` passes with unit tests verifying deduplication between tmux sessions, symlinks, and zoxide paths.
- Running `omega connect` inside tmux switches client; running outside attaches to terminal.
- Benchmark: Candidate aggregation for 1,000 directories completes in $< 30\text{ms}$.

---

### Phase 2: Intelligence (TUI & Workspaces)

_Goal: Elevate the interactive user experience with a responsive split preview pane, rich git worktree handling, and flexible output formatting._

#### Capabilities & Deliverables

1. **Interactive Two-Pane TUI**:
   - Split-screen layout: candidate list on the left, live preview pane on the right.
   - Configurable preview width (default 55%) with responsive auto-collapse below terminal width thresholds.
   - Asynchronous, debounced preview worker thread preventing UI frame drops during fast cursor scrolling.
2. **Context-Aware Previews**:
   - Active tmux sessions: Live terminal capture via `tmux capture-pane -e -p -t <session>`.
   - Git repositories / worktrees: Formatted `git status -s` and recent commit log.
   - Plain directories: Compact directory tree listing.
3. **First-Class Git Worktree Navigation**:
   - Native discovery of `.git` worktree administrative metadata and linked worktree roots.
   - Automatic session naming following the `<repo>--<branch>` normalization standard.
4. **Convenience & Integration Tooling**:
   - `omega clone <repo>`: Clones repository and immediately attaches.
   - CLI scriptability: `omega list --json` and `omega list --format "{name} | {path}"`.
   - Interactive search sigils (`@` for aliases, `:t` for tmux, `:g` for git).

#### Verification Criteria

- Two-pane Ratatui UI maintains constant 60 FPS while scrolling rapidly across sessions.
- Live tmux pane output renders ANSI colors accurately in the preview box.
- Linked Git worktrees are recognized and assigned `<repo>--<branch>` names automatically.

---

### Phase 3: Persistence & Daemon (Speed & Resiliency)

_Goal: Implement native session save/restore architecture and provide sub-millisecond query responses via a background Unix domain socket daemon._

#### Capabilities & Deliverables

1. **Session Save & Restore System**:
   - Structured JSON snapshot schema in `$XDG_DATA_HOME/omega/snapshots/`.
   - Capture of native tmux layout strings (`#{window_layout}`) and per-pane current paths (`#{pane_current_path}`).
   - Rolling history retention: maintains the last 10 auto-saves with atomic `latest.json` pointer; persistent named tags via `omega save --name <tag>`.
   - `omega restore`: Non-destructive session recreation by default; optional `--overwrite` flag.
   - Selective session restore: `omega restore <session-name>`.
2. **Background Cache Daemon (`omega daemon`)**:
   - Lightweight standalone daemon communicating via Unix domain socket (`$XDG_RUNTIME_DIR/omega.sock`).
   - Keeps in-memory hot candidate cache updated via background filesystem and tmux polling.
   - Fallback design: CLI and TUI operate seamlessly via direct synchronous scan if the daemon is stopped.
3. **Automatic Persistence Lifecycle**:
   - Periodic 15-minute background auto-save timer managed by the daemon.
   - Graceful termination hooks snapshotting state on server exit.

#### Verification Criteria

- `omega save` followed by killing all tmux sessions and running `omega restore` reconstructs the exact window splits, layouts, and pane directories.
- `omega list` with the daemon running responds over the Unix socket in $< 5\text{ms}$.
- Stopping the daemon causes Omega to fall back cleanly to synchronous direct scans without errors.

---

### Phase 4: Extensibility & Polish (Lua & Ecosystem)

_Goal: Embed the complete Lua 5.4 configuration engine, implement user plugins and hooks, and deliver packaging distributions._

#### Capabilities & Deliverables

1. **Embedded Lua 5.4 Engine**:
   - Integrated `mlua = "0.12"` with `["lua54", "vendored", "serde"]`.
   - Declarative `omega.setup({ ... })` API parsing user options, keymaps, aliases, and wildcards.
   - Native startup window and pane layouts in Lua replacing tmuxinator/tmuxp.
2. **Modular Architecture & Plugin System**:
   - Standard package path configuration: `~/.config/omega/init.lua`, `lua/`, and `plugins/`.
   - Lifecycle event hook dispatch: `omega.on("session_created", ...)`.
   - Custom candidate source registration: `omega.register_source(...)`.
   - Custom naming hook: `omega.session_name(...)`.
3. **Diagnostics & Polish**:
   - `omega doctor`: Health check validating tmux installation, socket connectivity, git versions, and Lua config validity.
   - Shell completions generated for Bash, Zsh, and Fish via `clap_complete`.
4. **Distribution Packaging**:
   - Nix Flake (`flake.nix`) build verification.
   - Homebrew tap formula for macOS.
   - Pre-compiled GitHub release binaries with Link-Time Optimization (`lto = true`) and stripped symbols.

#### Verification Criteria

- `omega` loads user configuration from `init.lua`, correctly registering custom aliases and wildcard startup commands.
- `omega doctor` accurately flags missing tools or syntax errors in `init.lua`.
- Homebrew formula and Nix flake compile and package the binary cleanly.

---

### Phase 5: Multiplexer Mastery & Workflow Consolidation

_Goal: Eliminate external multiplexer glue scripts, floating window plugins, and shell redirection workarounds by providing native window switching, floating scratchpad sessions, seamless shell/tmux initialization, and built-in semantic theming._

#### Capabilities & Deliverables

1. **Native Tmux Window Navigation (`omega window` & `:w` Sigil)**:
   - Aggregates active tmux windows across all running sessions into candidate items using `tmux list-windows -a`.
   - Filters window candidates directly via `:w <query>` interactive prefix sigil or dedicated `omega window` CLI subcommand.
   - Renders live terminal preview pane showing target window pane content with ANSI escape sequence decoding.
   - Switches client focus directly to the target window via `tmux select-window -t <session>:<window>` or `tmux switch-client`.
   - Replaces external window discovery scripts and `sesh window`.
2. **Persistent Floating Scratchpad (`omega scratch`)**:
   - Toggles a dedicated, persistent terminal scratchpad session inside a centered tmux popup (`display-popup -E -w 80% -h 80%`).
   - Attaches to the active scratch session if already running; auto-creates it on first invoke with the user shell or configured command.
   - Preserves running terminal processes and background jobs indefinitely across popup dismissals.
   - Replaces third-party floating pane plugins (such as `tmux-floax`).
3. **Shell & Multiplexer Integrations (`omega init`)**:
   - `omega init <zsh|bash|fish>`: Emits self-contained shell integration scripts with clean TTY allocation (`exec < /dev/tty`), providing interactive Alt+s keybindings and ZLE widgets without manual shell wrappers.
   - Generates canonical tmux keybinding configurations:
     - `bind-key -n M-s display-popup -E -w 85% -h 80% "omega"`
     - `bind-key -n M-w display-popup -E -w 85% -h 80% "omega window"`
     - `bind-key L run-shell "omega last"`
     - `bind-key 9 run-shell "omega root"`
   - Consolidates repository root queries (`omega root`) and session bouncing (`omega last`), replacing external helper scripts (`find_git_root.py`, `rename_session.sh`).
4. **Embedded Semantic Theming & Catppuccin Presets**:
   - Built-in color palette system configured in Lua via `omega.theme("catppuccin-mocha")`, supporting all official Catppuccin flavors (`latte`, `frappe`, `macchiato`, `mocha`) and standard terminal presets.
   - Maps semantic palette tokens (crust, mantle, base, surface, text, subtext, mauve, lavender, blue, sapphire, green, peach, red) into Ratatui border, header, item, and highlight styles.
   - Provides programmatic Lua theme definition API for user-defined custom color schemes.

#### Verification Criteria

- `omega window` aggregates and displays all active windows across all tmux sessions in $< 15\text{ms}$.
- Selecting a window in `omega window` switches tmux client focus directly to the exact target window.
- `omega scratch` spawns or toggles the persistent scratch session within a tmux popup; running processes survive popup dismissal and resume on next toggle.
- `omega init zsh` emits valid Zsh code that evaluates cleanly via `eval "$(omega init zsh)"` and binds interactive selection without terminal corruption.
- Lua invocation `omega.theme("catppuccin-mocha")` applies matching Ratatui styles with 100% unit test coverage in `omega-config` and `omega-ui`.

---

## 9. Explicit Out-of-Scope Boundaries

To maintain high performance, strict focus, and zero bloat, the following capabilities are explicitly ruled out of scope:

- **Remote SSH Multiplexer Management**: Omega manages local tmux instances only. Remote host multiplexing is left to SSH wrappers or plugins.
- **Alternative Multiplexers (Zellij, GNU Screen)**: Omega is optimized specifically for tmux architecture and APIs. Multiplexer abstraction traits may exist internally, but only tmux is supported.
- **Built-in Terminal Emulator**: Omega runs inside existing terminal emulators (Alacritty, Ghostty, Kitty, WezTerm); it does not emulate terminals.
- **External Multiplexer Config Parsers**: No legacy support or parsing for `tmuxinator` YAML or `tmuxp` JSON; Omega's native Lua tables supersede them.
- **macOS Browser Tab URL Snooping**: AppleScript integration to query browser tabs is excluded as fragile and platform-locked.
- **GitHub Issue/PR Worktree Provisioning**: Automatic creation of branches and worktrees from GitHub GraphQL queries belongs in dedicated Git tools (`gh`), not in Omega's session navigator.
- **Centralized Plugin Registry**: Omega supports local Lua plugins via `plugins/`, but will not host a package manager or centralized cloud registry.
