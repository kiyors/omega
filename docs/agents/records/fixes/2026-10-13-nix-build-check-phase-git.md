# Fix Record: Nix Build Check Phase Missing Git Dependency

- **Date**: 2026-10-13
- **Component**: `nix/package.nix`, `tests/clone_test.rs`, `crates/session/tests/preview_test.rs`
- **Issue**: `nix build` failed with exit code 101 during `checkPhase` when running `tests/clone_test.rs`.

---

## 1. Root Cause Analysis

Nix derivations execute phases inside a sandboxed environment where `$PATH` only contains inputs explicitly provided to the derivation.
In `nix/package.nix`, `rustPlatform.buildRustPackage` declared:
```nix
nativeBuildInputs = [ pkg-config ];
```
Without `git` specified in `nativeCheckInputs` or `nativeBuildInputs`, the sandboxed test runner lacked `git` on `$PATH`.
Consequently:
- `tests/clone_test.rs` panicked with `git init failed: Os { code: 2, kind: NotFound, message: "No such file or directory" }`.
- Similarly, `crates/session/tests/preview_test.rs` assumed `git` binary availability unconditionally.

---

## 2. Reproduction

Running `nix build` on macOS/Linux resulted in:
```text
failures:
---- test_clone_command_execution stdout ----
thread 'test_clone_command_execution' panicked at tests/clone_test.rs:53:10:
git init failed: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

---

## 3. Solution

A two-layer fix was implemented:

1. **Nix Derivation Inputs (`nix/package.nix`)**:
   - Added `git` to the derivation parameter list (`{ lib, rustPlatform, stdenv, pkg-config, libiconv, git }:`)
   - Added `nativeCheckInputs = [ git ];` so the test sandbox includes `git` during `cargo test` execution.

2. **Integration Test Environment Resilience (`tests/clone_test.rs` & `crates/session/tests/preview_test.rs`)**:
   - Added a `git_available()` helper (`git --version`) mirroring the pattern used in `tests/persistence_e2e_test.rs` (`tmux_available()`).
   - Gracefully skipped tests when git is absent in the environment instead of panicking.

---

## 4. Verification

- `cargo test --workspace`: All 42 tests passed.
- `cargo clippy --all-targets -- -D warnings`: Clean.
- `cargo fmt --check`: Clean.
- `nix build`: Succeeded completely with output `./result/bin/omega`.
