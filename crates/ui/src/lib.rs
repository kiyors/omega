pub mod ansi;
pub mod app;
pub mod layout;
pub mod render;
pub mod theme;

pub use ansi::parse_ansi_to_text;
pub use app::App;
pub use layout::{
    MIN_POPUP_TERMINAL_HEIGHT, MIN_POPUP_TERMINAL_WIDTH, MIN_PREVIEW_WIDTH_THRESHOLD,
    compute_centered_rect, compute_popup_area, resolve_border_type, resolve_borders,
};
pub use render::{format_windows_peek, render, resolve_color};
pub use theme::{ResolvedTheme, parse_color};

use ratatui::layout::Rect;
use std::rc::Rc;

#[must_use]
pub fn should_render_preview(app: &App, area: Rect) -> bool {
    layout::should_render_preview(app.preview_enabled, area)
}

#[must_use]
pub fn compute_horizontal_chunks(app: &App, area: Rect) -> Rc<[Rect]> {
    layout::compute_horizontal_chunks(app.preview_enabled, app.preview_width, area)
}
