use tetron_wm::help::{help_sections, preferred_size, HelpView};

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
fn help_window_has_a_sensible_size_and_renders_its_body() {
    let (w, h) = preferred_size(120, 50);
    assert!((30..=118).contains(&w) && (6..=47).contains(&h), "{w}x{h}");
    // Tiny screens still get a usable window.
    let (w, h) = preferred_size(40, 10);
    assert!(w >= 30 && h >= 6, "{w}x{h}");
    let v = HelpView::new();
    let buf = v.render(w, h);
    let text: String = (0..buf.height()).flat_map(|y| (0..buf.width()).map(move |x| (x, y))).map(|(x, y)| buf.get(x, y).unwrap().ch).collect();
    assert!(text.contains("Leader"), "the body is drawn");
}

#[test]
fn help_view_scrolls_within_bounds_and_pages() {
    let v = HelpView::new();
    v.render(60, 8); // learns it has 8 rows
    assert_eq!(v.scroll(), 0);
    v.scroll_by(i32::MAX);
    let end = v.scroll();
    assert!(end > 0);
    v.scroll_by(5);
    assert_eq!(v.scroll(), end, "clamped at the end");
    v.scroll_by(i32::MIN);
    assert_eq!(v.scroll(), 0);
    assert_eq!(v.page(), 7, "a page is one row less than the window");
}
