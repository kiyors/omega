# AGENTS.md — Omega Master Agent Guide

System prompt and operational instructions for AI agents working in the **Omega** repository.

> **Related Documents & Pointers**:
>
> - [`ARCHITECTURE.md`](./ARCHITECTURE.md) — System architecture, crate boundaries, data flow, and components.
> - [`GLOSSARY.md`](./GLOSSARY.md) — Single-context domain glossary (see [`docs/agents/domain.md`](./docs/agents/domain.md)).
> - [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md) — Issue tracking conventions via `gh` CLI.
> - [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) — Canonical triage role mapping.
> - [`docs/agents/domain.md`](./docs/agents/domain.md) — Domain docs consumption rules.
> - `docs/agents/records/` — Historical operational records (`features/`, `fixes/`, `adrs/`).

---

## 1. Core Behavioral Directives (The Four Principles)

Derived from standard agent best practices (Karpathy / Claude Code guidelines). Bias toward precision, verification, and caution over haste.

### 1.1 Think Before Coding

**Surface tradeoffs early. State assumptions explicitly. Halt on confusion.**

- **State Assumptions**: Name assumptions explicitly before editing code. If uncertain, ask rather than guess.
- **Surface Ambiguities**: When multiple interpretations exist, present them clearly — do not select one silently.
- **Push Back on Complexity**: When a simpler approach exists, propose it directly.
- **Halt on Confusion**: When instructions, requirements, or existing code paths are unclear, stop immediately and identify the exact confusion.
- **Principled Independence**: If a requested change is brittle, introduces regressions, or violates architectural invariants, explain why and suggest the robust alternative.

### 1.2 Simplicity First

**Deliver the minimal working solution. Avoid speculative additions.**

- **No Speculative Code**: Implement strictly what was requested — no unneeded configurability, speculative hooks, or premature abstractions.
- **No Single-Use Abstractions**: Write concise, direct logic. If 50 lines suffice, do not write 200 lines.
- **Targeted Error Handling**: Handle expected runtime errors; avoid defensive fallbacks for impossible internal states.
- **The Senior Engineer Test**: If a senior engineer would call the implementation over-engineered, simplify it before finalizing.

### 1.3 Surgical Changes

**Touch only what you must. Clean up your own mess.**

- **Strict Diff Boundary**: Confine edits to code directly required by the task. Never reformat or "clean up" adjacent unrelated code.
- **Match Codebase Conventions**: Mirror surrounding Rust idiomatic style, naming, and error-handling patterns.
- **Clean Up Leftovers**: Remove any unused imports, dead functions, or temporary helpers your changes rendered obsolete.
- **Respect Existing Dead Code**: If unrelated dead code is discovered, mention it — do not delete it unprompted.
- **Traceability**: Every changed line must trace directly to a stated goal or requirement.

### 1.4 Goal-Driven Execution

**Define verifiable criteria. Loop until verified.**

- **Verifiable Milestones**: Convert every task into concrete, testable criteria before editing.
- **Plan Step-by-Step**: For non-trivial workflows, follow a verified execution loop:
  ```text
  1. [Step] → verify: [cargo check / test / command]
  2. [Step] → verify: [cargo check / test / command]
  3. [Step] → verify: [cargo check / test / command]
  ```
- **Loop Until Verified**: Run checks, builds, and test suites to confirm correctness before declaring completion.
- **Naive First, Optimize Second**: Establish correctness with tests first. Once verified green, apply performance optimizations.

---

## 2. Test-Driven Development (TDD) & Test Preservation

Testing is the authoritative verification mechanism for Omega.

### 2.1 Test-First Requirement (Red-Green-Refactor)

- **Write Tests First**: For every new feature, subsystem, or bugfix, write a failing unit or integration test first (_red_).
- **Verify Failure**: Confirm the test fails for the expected reason before implementing the solution.
- **Minimal Implementation**: Implement the minimal code required to satisfy the test suite (_green_).
- **Refactor Safely**: Clean and optimize the implementation while keeping all tests passing (_refactor_).
- **Thorough Feature Verification**: Every feature must be checked against unit tests and edge cases before completion.

### 2.2 Test Invariant & Immutability Rule

- **Tests Are Authoritative Contracts**: Existing test files define the system's behavioral commitments.
- **Do Not Loosen Tests for Broken Code**: Never rewrite, weaken, or delete an existing test assertion simply to make broken implementation code pass.
- **Strict Condition for Test Modification**: Test files must only be updated, modified, or rewritten when the underlying product requirements or interface specifications explicitly change. Otherwise, treat existing test suites as immutable ground truth.

---

## 3. Modular Crate & Workspace Architecture

Omega is organized to prevent cross-feature interference through strict architectural boundaries.

```text
omega/
├── crates/         # Isolated domain subsystems (workspace crates)
│   ├── session/    # Discovery and management of tmux/git/local sessions
│   ├── search/     # High-speed scoring and matching engine
│   └── config/     # Settings, serialization, and user preferences
└── src/            # Central orchestrator and interactive TUI shell
```

### 3.1 Architectural Principles

1. **Isolated Subsystems (`crates/*`)**: Place business logic and distinct functional domains in dedicated crates within `crates/`. Each crate manages its own models, errors, and unit tests.
2. **Central Orchestrator (`src/`)**: The root binary crate in `src/` connects and coordinates all subsystems, handling the TUI event loop and terminal lifecycle.
3. **Decoupled Features**: Changes to one feature or crate must not produce side effects in unrelated crates. Interfaces between crates must remain strictly typed and narrow.
4. **Workspace Dependencies**: Crates in `crates/*` are declared as workspace members and dependencies in `Cargo.toml`, making them reusable across binaries, tests, and future tooling.

---

## 4. Documentation Governance & The `/docs/` Rule

Documentation in this repository is an authoritative, living knowledge base.

### 4.1 Always Consult `/docs/` First

- Before proposing designs, modifying architecture, or adding features, agents **must** inspect `ARCHITECTURE.md` and relevant docs in `docs/`.
- Never guess or approximate system conventions; align with documented decisions.

### 4.2 Keep State Synchronized

- **Mandatory Update on Significant Changes**: Whenever you introduce an architectural change, add a crate or module, modify data flow, or introduce a major feature, you **must update `ARCHITECTURE.md` and related docs**.
- Keep `ARCHITECTURE.md` accurate so human developers and future agents always have a reliable map of the system.

### 4.3 Agent Operational Records (`docs/agents/records/`)

When introducing a new feature, resolving a non-trivial bug, or making an architectural decision, record the rationale in `docs/agents/records/`:

- `docs/agents/records/features/YYYY-MM-DD-<slug>.md` — What the feature does, how it is tested, and why it was built.
- `docs/agents/records/fixes/YYYY-MM-DD-<slug>.md` — Root cause analysis, reproduction test, and solution.
- `docs/agents/records/adrs/YYYY-MM-DD-<slug>.md` — Architecture Decision Record detailing context, alternatives considered, and consequences.

**Goal**: Guarantee seamless continuity so any future agent or engineer understands _what_ was done and _why_ it was done without knowledge drift.

---

## 5. Rust Performance & Optimization Guidelines

Omega is a high-responsiveness terminal tool. Adhere strictly to these optimization rules in performance-sensitive paths:

### 5.1 Zero-Cost Iteration: Avoid Intermediate Allocations

- **Rule**: Never call `.collect::<Vec<_>>()` on an iterator chain if another iterator method or aggregator (`.sum()`, `.fold()`, `.any()`, `.find()`) immediately follows.
- **Pattern**:
  ```rust
  // BAD: Intermediate heap allocation
  let total: u64 = items.iter().map(|x| x * x).collect::<Vec<_>>().iter().sum();

  // GOOD: Zero-allocation lazy evaluation
  let total: u64 = items.iter().map(|x| x * x).sum();
  ```

### 5.2 Lifetimes & Zero-Copy: Prefer Borrowed Slices in Hot Paths

- **Rule**: In loops, fuzzy scoring, parsing, and filtering logic, avoid `.to_string()`, `.to_owned()`, and `.clone()`. Use borrowed slices (`&str`, `&[T]`) for working keys and buffers.
- **Rule**: Delay heap allocation to the final return boundary (e.g., only allocate for the final selected items, not every examined candidate).
- **Pattern**:
  ```rust
  // BAD: Allocates 1 String per iteration
  let mut counts: HashMap<String, usize> = HashMap::new();
  for line in lines {
      let id = line.split(',').nth(1).unwrap().to_string();
      // ...
  }

  // GOOD: Borrows from input buffer; zero heap allocations per iteration
  let mut counts: HashMap<&str, usize> = HashMap::new();
  for line in lines {
      let id = line.split(',').nth(1).unwrap();
      // ...
  }
  ```

### 5.3 HashMap Access: Always Use the Entry API

- **Rule**: Do not combine `map.contains_key(...)` with a subsequent `map.insert(...)` or `map.get_mut(...)`. Every call recomputes the hash and traverses table buckets.
- **Pattern**:
  ```rust
  // BAD: Computes hash up to 3 times
  if !map.contains_key(&key) {
      map.insert(key, 0);
  }
  *map.get_mut(&key).unwrap() += 1;

  // GOOD: Computes hash exactly once
  *map.entry(key).or_insert(0) += 1;
  ```

### 5.4 Data Parallelism: Use Rayon for Independent CPU-Bound Loops

- **Rule**: For compute-heavy filesystem scans, batch scoring, or session processing with independent items, leverage Rayon's parallel iterators (`par_iter()`, `into_par_iter()`).
- **Pattern**:
  ```rust
  use rayon::prelude::*;

  let aggregate = data
      .par_iter()
      .fold(HashMap::new, |mut acc, item| {
          *acc.entry(item.key).or_insert(0) += 1;
          acc
      })
      .reduce(HashMap::new, |mut map1, map2| {
          for (k, v) in map2 {
              *map1.entry(k).or_insert(0) += v;
          }
          map1
      });
  ```

### 5.5 Vector Deletion: Use `swap_remove` When Order Does Not Matter

- **Rule**: Never use `Vec::remove(idx)` ($O(N)$ shift) unless preserving index sequence is mandatory. For unordered lists, session pools, or sweeps, use `Vec::swap_remove(idx)` ($O(1)$).
- **Pattern**:
  ```rust
  // BAD: Shifts remaining elements in memory (O(N))
  sessions.remove(i);

  // GOOD: Swaps with last element and pops (O(1))
  sessions.swap_remove(i);
  ```

---

## 6. Tooling, Commands & Quality Standards

Always use standard Cargo workflows:

```bash
# Check compilation without code generation
cargo check

# Run all test suites
cargo test

# Run linter with strict warning enforcement
cargo clippy --all-targets -- -D warnings

# Format codebase
cargo fmt --check
```

---

## 7. Git Workflow & Pre-Flight Standards

### 7.1 Branch & Worktree Isolation

- **Isolated Development**: Always implement new features, enhancements, or non-trivial changes in a dedicated `git worktree` or new branch (`feat/<slug>`, `fix/<slug>`). Never commit feature work directly to `main`.
- **Iterative Refinement**: Develop, test, and improve changes within the isolated branch. Keep feedback loops tight with continuous testing.
- **Descriptive Pull Requests**: Once verified, open a pull request with a detailed description of all changes made. Adhere to the repository's `pr` skill template: structured summary (diagram, diff, or tree), before-and-after evidence, and merge danger / blast radius analysis.

### 7.2 Commit Standards & Hygiene

- **Conventional Type Tags**: Prefix commit titles with standard Conventional Commit types and optional scopes (`feat(session):`, `fix(search):`, `chore:`, `refactor:`, `test:`, `docs:`, `perf:`). Keep titles imperative and concise.
- **Descriptive & Self-Contained Messages**: Commit messages must provide sufficient context to understand what was done and why. For non-trivial changes, include an explanatory body beneath the title so the commit is clear in isolation from `git log` alone.
- **Emoji Prohibition**: Never include emojis in commit titles, commit bodies, branch names, or PR titles.
- **Explicit Git Flags**: Use deliberate, standard git flags (e.g., `git commit -m "<title>" -m "<body>"`). Never use flags like `--no-verify` or blind staging commands that bypass validation.

### 7.3 Pre-Flight Checklist

Before opening a PR or marking any task complete, verify:

- [ ] **Isolated Branch/Worktree**: Developed on a dedicated branch or worktree, not directly on `main`.
- [ ] **Tests First & Passing**: Unit and integration tests were written/verified and pass (`cargo test`).
- [ ] **Tests Intact**: Existing test files were preserved without arbitrary modifications.
- [ ] **Linter & Formatting**: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass cleanly.
- [ ] **Performance Checked**: No unnecessary intermediate `.collect::<Vec<_>>()`, zero-copy slices utilized in hot paths, Entry API used for maps.
- [ ] **Crate Isolation**: Logic placed in appropriate crate or module without leaking cross-feature dependencies.
- [ ] **Docs Synchronized**: `ARCHITECTURE.md` and `docs/agents/records/` updated if architectural changes or major features were added.
- [ ] **Clean Commits**: Conventional Commits adhered to with proper type tags, informative bodies explaining what and why, proper git flags, and zero emojis.
- [ ] **Descriptive PR**: PR opened with detailed summary, evidence, and blast radius analysis (`pr` skill).

---

## 8. Agent Skills

### Issue Tracker & Triage

#### Issue tracker

GitHub issues via `gh` CLI. See `docs/agents/issue-tracker.md`.

#### Triage labels

Canonical triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

#### Domain docs

Single-context (`GLOSSARY.md` and `docs/adr/` at repo root). See `docs/agents/domain.md`.

### Specialized Skills Reference

- **TDD (`tdd`)**: Test-driven development for features and bug fixes (Red-Green-Refactor).
- **Code Review (`code-review`)**: Review changes against repository standards and specifications.
- **Grilling (`grilling` / `grill-me`)**: Stress-test architectural decisions and implementation plans.
- **Domain Modeling (`domain-modeling`)**: Maintain terminology in `GLOSSARY.md` and record decisions in `docs/adr/`.
- **Writing for Agents (`writing-for-agents`)**: Guidelines for authoring skills, `AGENTS.md`, and agent documentation.
- **Pull Requests (`pr`)**: Generate concise, traceable pull request descriptions.
