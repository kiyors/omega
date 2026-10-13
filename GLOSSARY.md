# Domain Glossary: Omega

This document defines canonical domain terminology for **Omega**. It is maintained in accordance with `domain-modeling` practices and strictly describes domain concepts, independent of implementation details.

---

### Session Candidate

A navigable target or workspace destination presented to the user. A candidate may correspond to a running tmux session, a Git repository or worktree, a tracked frecency directory, or a configured bookmark/alias.

### Session Source

A discovery subsystem responsible for identifying and enumerating session candidates. The primary sources are:

- `tmux`: Active multiplexer sessions and panes.
- `alias`: Explicitly named shortcuts with pre-configured behavior and startup commands.
- `config`: Explicitly configured root directories and session presets.
- `git`: Git repository roots and linked worktrees.
- `zoxide`: Directory paths ranked by access frecency (frequency and recency).

### Canonical Path Deduplication

The process of collapsing multiple candidate representations pointing to the same underlying filesystem path into a single candidate. When duplicates collide on a canonical path, higher-priority sources (`tmux` > `alias` > `config` > `git` > `zoxide`) shadow lower-priority sources while inheriting alias tags and search metadata.

### Prefix Sigil

A designated symbol entered at the beginning of an interactive search query (e.g., `@` for aliases, `:t` for active tmux sessions, `:g` for git repositories) that restricts filtering exclusively to a specific session source.

### Attached Session

The tmux session currently displayed in the user's active terminal window. In the interactive picker, this session is hidden by default to prioritize switching to previous workspaces.

### Session Snapshot

A serialized point-in-time record of active tmux sessions, containing session names, window hierarchies, exact pane geometric layouts (via native tmux layout strings), and active working directories.

### Snapshot Rotation

A retention mechanism that maintains a rolling history of the most recent auto-saved snapshots alongside an atomic pointer to the latest state, while preserving explicitly named snapshots indefinitely.

### Selective Restore

The operation of restoring one specific named session from a saved snapshot without modifying or recreating other active or missing sessions.

### Cache Daemon

An optional lightweight background process that listens on a Unix domain socket (`$XDG_RUNTIME_DIR/omega.sock`), maintains an in-memory candidate cache refreshed by background filesystem and tmux polling, and manages the periodic auto-save lifecycle. When stopped, Omega falls back to direct synchronous scans transparently.

### Hot Candidate Cache

The daemon-maintained in-memory set of session candidates, refreshed on a fixed background interval, enabling sub-millisecond query responses over the Unix socket for CLI and TUI consumers.

### Auto-Save

A snapshot captured without an explicit tag, written to the rolling retention window (`snapshot-YYYY-MM-DD-HHMMSS.json`) by the daemon's periodic timer (default: 15 minutes) and on graceful daemon termination.

### Direct-Scan Fallback

The transparent degradation path in which candidate discovery runs synchronously against tmux, git, configured roots, and zoxide when the cache daemon is not reachable, guaranteeing identical results without the socket round-trip.

### Naming Pipeline

The staged transformation sequence (`alias` → `Lua hook` → `regex substitutions` → `worktree/directory defaults` → `character sanitization`) that resolves an arbitrary workspace path or target into a valid, deterministic tmux session name.

### Collision Suffix

An incremental numeric suffix (e.g., `-2`, `-3`) automatically appended to a newly generated session name when an active tmux session with the same base name already exists pointing to a different directory.

### Worktree Name Normalization

The standard convention of formatting linked Git worktree session names as `<repo>--<sanitized-branch>`, preventing naming collisions across repositories sharing common branch names while keeping the parent repository explicit.

### CLI Mode

Non-interactive execution of Omega subcommands (e.g., `list`, `connect`, `kill`, `last`, `root`, `save`, `restore`) designed for terminal pipeline scripting and external launcher integrations without launching the interactive TUI.

### Output Templating

A formatting mechanism for headless listing (`omega list --format "<template>"`) that interpolates session attributes (`{name}`, `{path}`, `{source}`, `{active_window}`) into user-defined text layouts without requiring external JSON processors.

### Convenience Actions

Atomic multi-step workflow commands (`last`, `root`, `clone`, `mkdir`) built into the Omega CLI to eliminate shell alias and script boilerplate.

### Configuration Root

The base configuration directory (`$XDG_CONFIG_HOME/omega/` or path configured via `OMEGA_CONFIG`) hosting `init.lua`, modular `lua/` libraries, and an auto-sourced `plugins/` folder.

### Declarative Setup Table

The primary options table passed to `omega.setup({ ... })` specifying global preferences, sort ordering, keybindings, aliases, and directory wildcards.

### Startup Layout

A declarative configuration structure specifying window names, pane subdivisions, layout styles, and initial commands automatically provisioned when a new tmux session is launched.

### Wildcard Session Rule

A path-globbing pattern rule (`*`, `**`) that applies default icons, window layouts, or startup commands to any session created within matching directory trees.
