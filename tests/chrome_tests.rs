use tetron_wm::chrome::{render_menubar, render_dock, dock_hit_regions, DockItem, DockKind, menubar_mode_region};
use tetron_wm::cell::Rgba;
use tetron_wm::geometry::Point;
use tetron_wm::window::WindowId;

fn badge_color() -> Rgba { Rgba::rgb(70, 130, 230) }

#[test]
fn menubar_layer_spans_top_row_and_shows_brand() {
    // 40 cols: realistic width where the Go button + app name + power button all fit.
    let layer = render_menubar(40, "btop", &[], false);
    assert_eq!(layer.origin, Point::new(0,0));
    assert_eq!(layer.buf.height(), 1);
    let row: String = (0..40).map(|x| layer.buf.get(x,0).unwrap().ch).collect();
    assert!(row.contains("tetron"));  // left brand button (opens launcher)
    assert!(row.contains("btop"));    // focused-app name
}

#[test]
fn dock_layer_is_bottom_row() {
    let items = vec![DockItem {
        kind: DockKind::Single(WindowId(1)),
        label: "btop".into(),
        count: 1,
        badge_letter: 'B',
        badge_color: badge_color(),
        focused: true,
        attention: false,
    }];
    let layer = render_dock(40, 24, &items);
    assert_eq!(layer.origin, Point::new(0, 23));
}

#[test]
fn dock_hit_regions_map_clicks_to_pills() {
    let items = vec![
        DockItem {
            kind: DockKind::Single(WindowId(1)),
            label: "btop".into(),
            count: 1,
            badge_letter: 'B',
            badge_color: badge_color(),
            focused: true,
        attention: false,
        },
        DockItem {
            kind: DockKind::Single(WindowId(2)),
            label: "lazygit".into(),
            count: 1,
            badge_letter: 'L',
            badge_color: badge_color(),
            focused: false,
        attention: false,
        },
    ];
    let regions = dock_hit_regions(40, 24, &items);
    // first region is pill 0, second is pill 1
    assert_eq!(regions[0].0, 0);
    assert_eq!(regions[1].0, 1);
    // a click inside the first region hits pill 0
    let first_r = regions[0].1;
    assert!(first_r.contains(Point::new(first_r.x, 23)));
    // regions are on the bottom row
    assert!(regions.iter().all(|(_, r)| r.y == 23));
}

#[test]
fn dock_single_pill_renders_badge_letter() {
    let items = vec![DockItem {
        kind: DockKind::Single(WindowId(1)),
        label: "btop".into(),
        count: 1,
        badge_letter: 'B',
        badge_color: badge_color(),
        focused: false,
        attention: false,
    }];
    let layer = render_dock(40, 24, &items);
    // The bottom row should contain 'B' (the badge letter)
    let row: String = (0..40).map(|x| layer.buf.get(x, 0).unwrap().ch).collect();
    assert!(row.contains('B'), "badge letter 'B' should appear in dock row: {row:?}");
}

#[test]
fn dock_group_pill_renders_count_glyph() {
    let items = vec![DockItem {
        kind: DockKind::Group("Claude".into(), vec![WindowId(1), WindowId(2)]),
        label: "Claude".into(),
        count: 2,
        badge_letter: 'C',
        badge_color: badge_color(),
        focused: false,
        attention: false,
    }];
    let layer = render_dock(60, 24, &items);
    let row: String = (0..60).map(|x| layer.buf.get(x, 0).unwrap().ch).collect();
    // Group of 2 → superscript ² should appear
    assert!(row.contains('\u{00B2}'), "group pill should show ² for count=2: {row:?}");
}

#[test]
fn menubar_shows_mode_toggle_glyph() {
    let desktop: String = (0..40).map(|x| render_menubar(40, "x", &[], false).buf.get(x, 0).unwrap().ch).collect();
    assert!(desktop.contains('\u{229E}'), "desktop mode shows ⊞, got {desktop:?}");
    let simple: String = (0..40).map(|x| render_menubar(40, "x", &[], true).buf.get(x, 0).unwrap().ch).collect();
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
    // A single tray segment placed flush to the right edge (reserve = 0).
    let seg = Segment { kind: SegmentKind::Clock, text: "09:41".into(), rect: tetron_wm::geometry::Rect::new(width - 5, 0, 5, 1) };
    let layer = render_menubar(width, "btop", &[seg], false);
    let row: String = (0..width).map(|x| layer.buf.get(x, 0).unwrap().ch).collect();
    // No host-name power button any more (moved into the launcher).
    assert!(!row.contains("devbox"));
    // The tray segment is drawn out to the right edge.
    assert!(row.ends_with("09:41"), "tray should reach the edge, got: {row:?}");
}
