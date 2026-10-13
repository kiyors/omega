use omega_ui::layout::{
    compute_horizontal_chunks, compute_popup_area, resolve_border_type, resolve_borders,
    should_render_preview,
};
use ratatui::layout::Rect;
use ratatui::widgets::{BorderType, Borders};

#[test]
fn test_compute_popup_area_centering_and_dimensions() {
    let terminal = Rect::new(0, 0, 100, 50);
    let popup = compute_popup_area(terminal, 80, 60);

    assert_eq!(popup.width, 80);
    assert_eq!(popup.height, 30);
    assert_eq!(popup.x, 10);
    assert_eq!(popup.y, 10);
}

#[test]
fn test_compute_popup_area_fallback_on_small_terminals() {
    // Width <= 50 should fallback to full screen
    let narrow = Rect::new(0, 0, 48, 40);
    assert_eq!(compute_popup_area(narrow, 80, 80), narrow);

    // Height <= 12 should fallback to full screen
    let short = Rect::new(0, 0, 120, 10);
    assert_eq!(compute_popup_area(short, 80, 80), short);
}

#[test]
fn test_compute_popup_area_clamped_percentages() {
    let terminal = Rect::new(0, 0, 100, 50);

    // Over 100% clamps to 100%
    let clamped_max = compute_popup_area(terminal, 150, 150);
    assert_eq!(clamped_max.width, 100);
    assert_eq!(clamped_max.height, 50);
    assert_eq!(clamped_max.x, 0);
    assert_eq!(clamped_max.y, 0);

    // Under 20% clamps to 20%
    let clamped_min = compute_popup_area(terminal, 5, 5);
    assert_eq!(clamped_min.width, 20);
    assert_eq!(clamped_min.height, 10);
    assert_eq!(clamped_min.x, 40);
    assert_eq!(clamped_min.y, 20);
}

#[test]
fn test_resolve_borders_variants() {
    assert_eq!(resolve_borders("none"), (Borders::NONE, BorderType::Plain));
    assert_eq!(resolve_borders("line"), (Borders::ALL, BorderType::Plain));
    assert_eq!(resolve_borders("plain"), (Borders::ALL, BorderType::Plain));
    assert_eq!(
        resolve_borders("double"),
        (Borders::ALL, BorderType::Double)
    );
    assert_eq!(resolve_borders("thick"), (Borders::ALL, BorderType::Thick));
    assert_eq!(
        resolve_borders("rounded"),
        (Borders::ALL, BorderType::Rounded)
    );
    assert_eq!(resolve_border_type("double"), BorderType::Double);
}

#[test]
fn test_preview_split_and_collapse() {
    let narrow = Rect::new(0, 0, 80, 24);
    assert!(!should_render_preview(true, narrow));
    let chunks_narrow = compute_horizontal_chunks(true, 55, narrow);
    assert_eq!(chunks_narrow.len(), 1);
    assert_eq!(chunks_narrow[0].width, 80);

    let wide = Rect::new(0, 0, 140, 24);
    assert!(should_render_preview(true, wide));
    let chunks_wide = compute_horizontal_chunks(true, 50, wide);
    assert_eq!(chunks_wide.len(), 2);
    assert_eq!(chunks_wide[0].width, 70);
    assert_eq!(chunks_wide[1].width, 70);
}
