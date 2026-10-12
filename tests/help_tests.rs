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
    assert!(sec.rows.iter().any(|(k, d)| *k == "x" && d.contains("select")));
    assert!(sec.rows.iter().any(|(k, _)| *k == "v"));
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
