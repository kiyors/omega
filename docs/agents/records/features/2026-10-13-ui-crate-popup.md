# Feature Record: UI Workspace Crate Decomposition & Popup Modal Layout

- **Date**: 2026-10-13
- **Author**: Agent Pair Programmer
- **Scope**: Dedicated `crates/ui` (`omega-ui`) workspace crate, centered popup modal window look-and-feel, customizable TUI options in `TuiConfig`, and programmatic `omega.ui` Lua APIs.

---

## 1. What Was Built

1. **Dedicated UI Subsystem Crate (`crates/ui` / `omega-ui`)**:
   - Extracted all UI and Ratatui presentation logic out of `src/` into an isolated workspace crate.
   - Partitioned responsibilities cleanly:
     - `crates/ui/src/popup.rs` / `layout.rs`: Popup geometry math (`compute_popup_area`), preview horizontal split (`compute_horizontal_chunks`), responsive threshold detection, and border resolution.
     - `crates/ui/src/ansi.rs`: Zero-allocation ANSI SGR escape code parser translating raw terminal and git command output into Ratatui `Text` spans.
     - `crates/ui/src/app.rs`: Interactive TUI state machine and reusable scoring buffer (`App`).
     - `crates/ui/src/render.rs`: Ratatui frame rendering engine handling the centered popup card, `Clear` backdrop, search box, workspace list with badges and icons, ANSI preview pane, and footer hints.

2. **Popup Modal Look-and-Feel**:
   - Rather than spanning the entire terminal full screen, Omega now defaults to a centered popup dialog with surrounding margins (`popup = true`, `popup_width = 85%`, `popup_height = 80%`).
   - Uses `ratatui::widgets::Clear` beneath the modal so terminal background noise does not bleed through.
   - Small-terminal graceful fallback: if the terminal width is $\le 50$ columns or height is $\le 12$ rows, Omega falls back to full-screen display to prevent UI truncation.

3. **User Customization APIs (Config & Lua)**:
   - Extended `TuiConfig` in `crates/config` with user-customizable properties and defaults:
     - `popup`: `bool` (default: `true`)
     - `popup_width`: `u16` (default: `85`)
     - `popup_height`: `u16` (default: `80`)
     - `popup_border`: `String` (default: `"rounded"`)
     - `border_color`: `String` (default: `"cyan"`)
     - `highlight_color`: `String` (default: `"yellow"`)
     - `search_prompt`: `String` (default: `"> "`)
     - `search_title`: `String` (default: `" Search / Switch Session (Prefixes: :t tmux, :g git, @ alias) "`)
     - `show_badges`: `bool` (default: `true`)
     - `show_icons`: `bool` (default: `true`)
     - `show_path`: `bool` (default: `true`)
     - `show_footer`: `bool` (default: `true`)
   - Programmatic Lua API in `crates/lua`:
     - Declarative: `omega.setup({ tui = { ... } })`
     - Imperative / Modular: `omega.ui.setup({ ... })` or `omega.ui.set({ ... })` with partial table merges
     - Inspection: `omega.ui.get()` returning current active TUI configurations.

---

## 2. Verification & Testing

Implemented following test-first Red-Green-Refactor across pre-agreed seams:

- **Seam 1 (`crates/ui/tests/popup_layout_test.rs`)**:
  - `test_compute_popup_area_centering_and_dimensions`: Verifies centered positioning and width/height percentage scaling.
  - `test_compute_popup_area_fallback_on_small_terminals`: Confirms small-screen full-screen fallback.
  - `test_compute_popup_area_clamped_percentages`: Confirms clamping bounds (20% to 100%).
  - `test_resolve_borders_variants`: Confirms rounded, line, double, thick, and none border types.
  - `test_preview_split_and_collapse`: Confirms preview split when wide and collapse when narrow.

- **Seam 2 (`crates/ui/tests/ansi_test.rs`)**:
  - `test_parse_plain_text`: Verifies borrowed plain text lines without styling.
  - `test_parse_ansi_colors_and_modifiers`: Verifies SGR color and bold style parsing.

- **Seam 3 (`crates/ui/tests/app_state_test.rs`)**:
  - `test_app_initialization_and_filtering`: Verifies `App::new`, `apply_tui_config`, query filtering, and bounds navigation.
  - `test_app_preview_toggling_and_scrolling`: Verifies preview toggle and scroll up/down without underflow.

- **Seam 4 (`crates/ui/tests/render_test.rs`)**:
  - `test_render_in_popup_mode`: Verifies centered modal buffer emission with margins at row 0 and rendered borders at popup rows via `TestBackend`.
  - `test_render_in_fullscreen_mode`: Verifies full-screen rendering when `popup = false`.

- **Seam 5 (`crates/config/tests/config_test.rs`, `crates/lua/tests/lua_config_test.rs`)**:
  - `test_tui_popup_and_customization_config_defaults_and_override`: Tests serde defaults and custom JSON overrides.
  - `test_lua_ui_api_configuration`: Tests `omega.ui.setup` and `omega.ui.get` in Lua scripts.

- **Quality Gates**:
  - `cargo test --workspace`: All tests pass cleanly across every crate and binary.
  - `cargo clippy --all-targets -- -D warnings`: 0 warnings under strict pedantic lints.
  - `cargo fmt --check`: Clean formatting across the entire workspace.
