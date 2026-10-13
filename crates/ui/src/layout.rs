use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{BorderType, Borders},
};
use std::rc::Rc;

pub const MIN_PREVIEW_WIDTH_THRESHOLD: u16 = 100;
pub const MIN_POPUP_TERMINAL_WIDTH: u16 = 50;
pub const MIN_POPUP_TERMINAL_HEIGHT: u16 = 12;

/// Computes a centered popup rectangle within a given container rectangle.
///
/// If the terminal is smaller than `MIN_POPUP_TERMINAL_WIDTH` or `MIN_POPUP_TERMINAL_HEIGHT`,
/// it falls back to the full `area` so UI elements remain readable.
#[must_use]
pub fn compute_popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    if area.width <= MIN_POPUP_TERMINAL_WIDTH || area.height <= MIN_POPUP_TERMINAL_HEIGHT {
        return area;
    }

    let px = percent_x.clamp(20, 100);
    let py = percent_y.clamp(20, 100);

    let popup_width = (area.width.saturating_mul(px)) / 100;
    let popup_height = (area.height.saturating_mul(py)) / 100;

    let x = area
        .x
        .saturating_add((area.width.saturating_sub(popup_width)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(popup_height)) / 2);

    Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    }
}

#[must_use]
pub fn should_render_preview(preview_enabled: bool, area: Rect) -> bool {
    preview_enabled && area.width >= MIN_PREVIEW_WIDTH_THRESHOLD
}

#[must_use]
pub fn compute_horizontal_chunks(
    preview_enabled: bool,
    preview_width: u16,
    area: Rect,
) -> Rc<[Rect]> {
    if should_render_preview(preview_enabled, area) {
        let preview_pct = preview_width.clamp(10, 90);
        let list_pct = 100_u16.saturating_sub(preview_pct);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(list_pct),
                Constraint::Percentage(preview_pct),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    }
}

#[must_use]
pub fn resolve_borders(border_str: &str) -> (Borders, BorderType) {
    match border_str {
        "none" => (Borders::NONE, BorderType::Plain),
        "line" | "plain" => (Borders::ALL, BorderType::Plain),
        "double" => (Borders::ALL, BorderType::Double),
        "thick" => (Borders::ALL, BorderType::Thick),
        _ => (Borders::ALL, BorderType::Rounded),
    }
}

#[must_use]
pub fn resolve_border_type(border_str: &str) -> BorderType {
    resolve_borders(border_str).1
}

#[must_use]
pub fn compute_centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x.saturating_add((area.width.saturating_sub(w)) / 2);
    let y = area.y.saturating_add((area.height.saturating_sub(h)) / 2);
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
