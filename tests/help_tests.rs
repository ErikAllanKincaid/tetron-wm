use tetron_wm::help::{help_sections, render_help};

#[test]
fn help_lists_the_tiling_shortcuts() {
    let all: Vec<(&str, &str)> = help_sections().iter().flat_map(|s| s.rows.iter().copied()).collect();
    assert!(all.iter().any(|(k, d)| *k == "t" && d.contains("tile")));
    assert!(all.iter().any(|(k, _)| *k == "1–9"));
    assert!(help_sections().iter().any(|s| s.title == "Session"));
}

#[test]
fn help_explains_select_mode_for_copying_out_of_apps() {
    let sec = help_sections().iter().find(|s| s.title == "Clipboard").expect("a Clipboard section");
    assert!(sec.rows.iter().any(|(k, d)| *k == "x / y" && d.contains("select")));
    assert!(sec.rows.iter().any(|(k, _)| *k == "C"));
    let tip = sec.notes.join(" ");
    assert!(tip.contains("w3m") && tip.contains("Ctrl+Space x"), "tip: {tip}");
}

#[test]
fn help_renders_a_centered_layer() {
    let layers = render_help(80, 30, 0);
    assert_eq!(layers.len(), 1);
    // Centered: origin is left of mid-screen, with positive width.
    assert!(layers[0].origin.x > 0);
}

#[test]
fn close_button_is_on_the_title_bar_inside_the_box() {
    use tetron_wm::help::close_button_rect;
    let layers = render_help(100, 40, 0);
    let l = &layers[0];
    let r = close_button_rect(100, 40);
    assert_eq!(r.y, l.origin.y, "on the title row");
    assert!(r.x >= l.origin.x && r.x + r.w <= l.origin.x + l.buf.width(), "inside the box");
    // The glyph is drawn where the hit rect says.
    let drawn: String = (0..r.w).map(|i| l.buf.get(r.x - l.origin.x + i, 0).unwrap().ch).collect();
    assert!(drawn.contains('\u{2715}'), "{drawn:?}");
}
