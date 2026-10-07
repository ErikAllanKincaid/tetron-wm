use tetron_wm::chrome::{render_menubar, menubar_taskbar_regions, menubar_new_shell_region, DockItem, DockKind, menubar_mode_region};
use tetron_wm::cell::Rgba;
use tetron_wm::geometry::Point;
use tetron_wm::window::WindowId;

fn badge_color() -> Rgba { Rgba::rgb(70, 130, 230) }

fn single(id: u64, label: &str, letter: char, focused: bool) -> DockItem {
    DockItem {
        kind: DockKind::Single(WindowId(id)),
        label: label.into(),
        count: 1,
        badge_letter: letter,
        badge_color: badge_color(),
        focused,
        attention: false,
    }
}

fn row_text(layer: &tetron_wm::compositor::Layer, width: i32) -> String {
    (0..width).map(|x| layer.buf.get(x, 0).unwrap().ch).collect()
}

#[test]
fn menubar_layer_spans_top_row_and_shows_brand_and_new_shell() {
    // 40 cols: realistic width where the brand + mode + assistant + "+" all fit.
    let layer = render_menubar(40, &[], &[], false);
    assert_eq!(layer.origin, Point::new(0, 0));
    assert_eq!(layer.buf.height(), 1);
    let row = row_text(&layer, 40);
    assert!(row.contains("tetron")); // left brand button (opens launcher)
    assert!(row.contains('+'));      // the "+" new-shell button
}

#[test]
fn new_shell_region_sits_after_the_brand_buttons() {
    let r = menubar_new_shell_region();
    assert_eq!(r.y, 0);
    assert!(r.x >= 17, "the + button is right of brand/mode/assistant, got x={}", r.x);
    assert_eq!(r.w, 3); // " + "
}

#[test]
fn taskbar_single_pill_renders_badge_letter() {
    let items = vec![single(1, "btop", 'B', false)];
    let layer = render_menubar(80, &items, &[], false);
    let row = row_text(&layer, 80);
    assert!(row.contains('B'), "badge letter 'B' should appear in the taskbar: {row:?}");
    assert!(row.contains("btop"), "the pill shows the window label when it fits: {row:?}");
}

#[test]
fn taskbar_group_pill_renders_count_glyph() {
    let items = vec![DockItem {
        kind: DockKind::Group("Claude".into(), vec![WindowId(1), WindowId(2)]),
        label: "Claude".into(),
        count: 2,
        badge_letter: 'C',
        badge_color: badge_color(),
        focused: false,
        attention: false,
    }];
    let layer = render_menubar(80, &items, &[], false);
    let row = row_text(&layer, 80);
    // Group of 2 → superscript ² should appear.
    assert!(row.contains('\u{00B2}'), "group pill should show ² for count=2: {row:?}");
}

#[test]
fn taskbar_hit_regions_map_clicks_to_pills_on_row_zero() {
    let items = vec![single(1, "btop", 'B', true), single(2, "lazygit", 'L', false)];
    let regions = menubar_taskbar_regions(80, &items, &[]);
    assert_eq!(regions[0].0, 0);
    assert_eq!(regions[1].0, 1);
    // All taskbar regions live on the top row.
    assert!(regions.iter().all(|(_, r)| r.y == 0));
    // The first pill starts right of the "+" button (TASKBAR_X).
    assert!(regions[0].1.x >= 21);
    // A click inside the first region hits pill 0.
    let first = regions[0].1;
    assert!(first.contains(Point::new(first.x, 0)));
}

#[test]
fn taskbar_overflows_to_badge_only_then_marker_in_narrow_width() {
    // Eight long-labelled windows in a narrow bar cannot show full labels, nor
    // even all badges — so a trailing "…+N" marker must appear.
    let items: Vec<DockItem> = (0..8)
        .map(|i| single(i as u64 + 1, "some-long-window-name", (b'A' + i) as char, false))
        .collect();
    let layer = render_menubar(40, &items, &[], false);
    let row = row_text(&layer, 40);
    assert!(row.contains('\u{2026}'), "narrow taskbar should show the … overflow marker: {row:?}");
    assert!(row.contains('+'), "the overflow marker counts the hidden pills: {row:?}");
}

#[test]
fn menubar_shows_mode_toggle_glyph() {
    let desktop = row_text(&render_menubar(40, &[], &[], false), 40);
    assert!(desktop.contains('\u{229E}'), "desktop mode shows ⊞, got {desktop:?}");
    let simple = row_text(&render_menubar(40, &[], &[], true), 40);
    assert!(simple.contains('\u{25A6}'), "simple mode shows ▦, got {simple:?}");
    // region sits just right of the brand
    let r = menubar_mode_region();
    assert_eq!(r.y, 0);
    assert!(r.x >= 7);
}

#[test]
fn menubar_has_no_power_button_and_tray_reaches_edge() {
    use tetron_wm::tray::{Segment, SegmentKind};
    let width = 40;
    // A single tray segment placed flush to the right edge.
    let seg = Segment { kind: SegmentKind::Clock, text: "09:41".into(), rect: tetron_wm::geometry::Rect::new(width - 5, 0, 5, 1) };
    let layer = render_menubar(width, &[], &[seg], false);
    let row = row_text(&layer, width);
    // No host-name power button any more (moved into the launcher).
    assert!(!row.contains("devbox"));
    // The tray segment is drawn out to the right edge.
    assert!(row.ends_with("09:41"), "tray should reach the edge, got: {row:?}");
}
