use std::collections::BTreeMap;
use std::fs;
use tetron_wm::config::AppEntry;
use tetron_wm::desktop::{DesktopAction, DesktopIcons, IconSource};
use tetron_wm::geometry::Point;

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("tetron-wm-dt-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn pins() -> Vec<AppEntry> {
    vec![AppEntry { name: "Files".into(), command: "@files".into(), args: vec![], category: None, requires_cwd: None, cwd: None, cli: None, warn: None }]
}

#[test]
fn merges_folder_entries_and_pins() {
    let d = tmp("merge");
    fs::write(d.join("notes.md"), b"x").unwrap();
    fs::create_dir(d.join("proj")).unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&pins(), &BTreeMap::new());
    dt.layout(100, 30); // assign cells
    // 2 folder items + 1 pin = 3 icons
    assert_eq!(dt.icons().len(), 3);
    assert!(dt.icons().iter().any(|i| i.label == "Files" && matches!(i.source, IconSource::Pinned)));
    assert!(dt.icons().iter().any(|i| i.label == "proj" && matches!(i.source, IconSource::Folder)));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn saved_position_wins_else_first_free_cell() {
    let d = tmp("pos");
    fs::write(d.join("a"), b"").unwrap();
    fs::write(d.join("b"), b"").unwrap();
    let mut pos = BTreeMap::new();
    pos.insert(d.join("b").to_string_lossy().to_string(), (2u16, 1u16));
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &pos);
    dt.layout(100, 30);
    let b = dt.icons().iter().find(|i| i.label == "b").unwrap();
    assert_eq!(b.cell, (2, 1));
    // a has no saved position → first free cell (0,0)
    let a = dt.icons().iter().find(|i| i.label == "a").unwrap();
    assert_eq!(a.cell, (0, 0));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn hit_test_and_select_then_double_click_opens() {
    let d = tmp("hit");
    fs::create_dir(d.join("proj")).unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &BTreeMap::new());
    dt.layout(100, 30);
    // proj is at cell (0,0); derive a point inside its tile (layout is right-aligned).
    let tr = dt.tile_rect((0, 0));
    let p = Point::new(tr.x + 1, tr.y + 1);
    assert_eq!(dt.icon_at(p), Some(0));
    assert!(dt.icon_at(Point::new(tr.x - 5, 20)).is_none()); // empty desktop, left of the tile
    dt.click(p, false);
    assert_eq!(dt.selection(), vec![0]);
    dt.double_click(p);
    assert_eq!(dt.take_action(), Some(DesktopAction::Open(d.join("proj"))));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn double_click_pin_runs_command() {
    let d = tmp("pin");
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&pins(), &BTreeMap::new());
    dt.layout(100, 30);
    let idx = dt.icons().iter().position(|i| i.label == "Files").unwrap();
    // place a click on that icon's tile
    let tr = dt.tile_rect(dt.icons()[idx].cell);
    let p = Point::new(tr.x + 1, tr.y + 1);
    dt.double_click(p);
    assert_eq!(dt.take_action(), Some(DesktopAction::Run { command: "@files".into(), args: vec![] }));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn right_click_opens_context_and_menu_targets() {
    use tetron_wm::desktop::DesktopOverlay;
    let d = tmp("ctx");
    fs::write(d.join("a.txt"), b"x").unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &BTreeMap::new());
    dt.layout(100, 30);
    let tr = dt.tile_rect((0, 0));
    dt.right_click(Point::new(tr.x + 1, tr.y + 1)); // on the icon
    assert!(matches!(dt.overlay(), Some(DesktopOverlay::Menu { idx: Some(0), .. })));
    dt.right_click(Point::new(tr.x - 5, 20)); // empty desktop
    assert!(matches!(dt.overlay(), Some(DesktopOverlay::Menu { idx: None, .. })));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn desktop_menu_keys_hover_and_stays_on_screen() {
    use tetron_wm::contextmenu::MenuAction;
    let d = tmp("ctxkeys");
    fs::write(d.join("a.txt"), b"x").unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &BTreeMap::new());
    dt.layout(100, 30);
    // Opened at the bottom-right corner: the box is moved back on screen, and
    // its first row is where a click lands on "Open".
    dt.right_click(Point::new(10, 29));
    assert!(dt.menu_open());
    let r = dt.overlay_rect().unwrap();
    assert!(r.x + r.w <= 100 && r.y + r.h <= 30 && r.y >= 1, "{r:?}");
    assert_eq!(dt.menu_item_at(Point::new(r.x + 2, r.y + 1)), Some(MenuAction::NewFolder));
    dt.menu_down();
    assert_eq!(dt.menu_selected(), Some(MenuAction::CleanUp));
    dt.menu_hover(Point::new(r.x + 2, r.y + 1));
    assert_eq!(dt.menu_selected(), Some(MenuAction::NewFolder), "hover highlights");
    // The keyboard opens the selected icon's menu at its tile.
    let tr = dt.tile_rect((0, 0));
    dt.click(Point::new(tr.x + 1, tr.y + 1), false);
    dt.open_menu_keyboard();
    assert!(matches!(dt.overlay(), Some(tetron_wm::desktop::DesktopOverlay::Menu { idx: Some(0), .. })));
    assert_eq!(dt.menu_selected(), Some(MenuAction::Open));
    // The menu is opaque: no glyph below shows through.
    let buf = dt.overlay_buffer(100, 30).unwrap();
    let r = dt.overlay_rect().unwrap();
    assert_eq!(buf.get(r.x, r.y).unwrap().bg.a, 255);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn rename_overlay_edits_and_commit_requests_fs() {
    let d = tmp("ren");
    fs::write(d.join("old.txt"), b"x").unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &BTreeMap::new());
    dt.layout(100, 30);
    dt.begin_rename(0);
    for _ in 0.."old.txt".len() {
        dt.overlay_backspace();
    }
    for c in "new.txt".chars() {
        dt.overlay_char(c);
    }
    // commit performs the rename via the model's fs (StdFs on temp dir)
    dt.overlay_commit();
    assert!(d.join("new.txt").exists());
    assert!(!d.join("old.txt").exists());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn drag_snaps_to_target_cell_and_reports_position() {
    let d = tmp("drag");
    fs::create_dir(d.join("proj")).unwrap();
    let mut dt = DesktopIcons::new(d.clone());
    dt.reload(&[], &BTreeMap::new());
    dt.layout(100, 30);
    // proj starts at (0,0); grab it and drop at a point inside cell (2,1)
    let from = dt.tile_rect((0, 0));
    dt.begin_drag(Point::new(from.x + 1, from.y + 1));
    let to = dt.tile_rect((2, 1));
    let drop = Point::new(to.x + 1, to.y + 1);
    let moved = dt.end_drag(drop);
    assert!(moved); // a move happened
    let key = dt.icon_key(0).unwrap();
    assert_eq!(dt.icons()[0].cell, (2, 1));
    // the model exposes the position to persist
    assert_eq!(dt.position_of(&key), Some((2, 1)));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn shrinking_layout_keeps_tiles_and_placements_on_screen() {
    // An icon saved deep into a wide grid must not land at negative x after the
    // screen shrinks (e.g. a transient size mid fullscreen-animation) — that
    // would draw it over the windows at the left edge, and its image placement
    // would be unaddressable by the client.
    let d = tmp("shrink");
    fs::write(d.join("a"), b"").unwrap();
    let mut pos = BTreeMap::new();
    pos.insert(d.join("a").to_string_lossy().to_string(), (5u16, 3u16));
    let mut dt = DesktopIcons::new(d.clone());
    dt.layout(200, 60); // wide grid: column 5 is valid
    dt.reload(&[], &pos);
    assert_eq!(dt.icons()[0].cell, (5, 3));
    assert!(dt.tile_rect((5, 3)).x >= 0);

    dt.layout(40, 12); // narrow grid: column 5 would be x = 40-1-6*14 < 0 unclamped
    let r = dt.tile_rect((5, 3));
    assert!(r.x >= 0, "tile clamped into the grid; got x={}", r.x);
    assert!(r.y >= 1, "tile clamped below the menubar; got y={}", r.y);

    let mut role_icons = std::collections::HashMap::new();
    role_icons.insert(dt.icons()[0].role, 42u64);
    let placements = dt.icon_placements(&role_icons, |_| true);
    assert_eq!(placements.len(), 1);
    assert!(placements[0].rect.x >= 0 && placements[0].rect.y >= 0,
        "placement stays addressable: {:?}", placements[0].rect);
    let _ = fs::remove_dir_all(&d);
}
