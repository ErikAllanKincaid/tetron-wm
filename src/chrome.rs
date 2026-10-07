use crate::buffer::CellBuffer;
use crate::compositor::Layer;
use crate::geometry::{Point, Rect};
use crate::window::WindowId;

// ── Theme constants ────────────────────────────────────────────────────────────

/// Label drawn for the top-left launcher button (opens the app launcher).
/// Short brand word, not the full binary name. TODO: make configurable
/// (config.toml); when it is, derive MODE_X/ASSIST_X/APP_X from its width
/// rather than hardcoding them (see `menubar_brand_region`, which already does).
const GO_LABEL: &str = " tetron ";

/// Column where the view-mode toggle is drawn (just right of the brand button).
const MODE_X: i32 = 9; // GO_LABEL is 8 cells wide, + a 1-cell gap

/// Column where the assistant (✦) button is drawn (right of the mode toggle).
const ASSIST_X: i32 = 13;

/// Label for the assistant button (opens the AI chat panel).
const ASSIST_LABEL: &str = " \u{2726} "; // ✦

/// Column of the "+" new-shell button (reuses the old focused-app slot).
const NEW_SHELL_X: i32 = 17;

/// Column where the taskbar pills begin (after the "+" button + a 1-cell gap).
/// `NEW_SHELL_LABEL` (" + ") is 3 cells wide, + a 1-cell gap.
const TASKBAR_X: i32 = 21;

/// Menubar view-mode toggle glyphs (shows the CURRENT mode; click to switch).
const MODE_DESKTOP: &str = " \u{229E} "; // ⊞  windowed desktop
const MODE_SIMPLE: &str = " \u{25A6} ";  // ▦  full-screen single app

// ── Public types ───────────────────────────────────────────────────────────────

/// What a dock pill activates.
#[derive(Clone, Debug)]
pub enum DockKind {
    /// A single window — clicking focuses it.
    Single(WindowId),
    /// A group of windows of the same app — clicking expands a chooser.
    Group(String, Vec<WindowId>), // (app_key, window ids)
}

/// One pill in the dock.
#[derive(Clone, Debug)]
pub struct DockItem {
    pub kind: DockKind,
    /// Text shown after the badge (single → its label; group → app key).
    pub label: String,
    /// Number of windows (>1 ⇒ a group; shown as a count).
    pub count: usize,
    /// Badge letter + color.
    pub badge_letter: char,
    pub badge_color: crate::cell::Rgba,
    /// Whether any window in this pill is focused.
    pub focused: bool,
    /// Whether any window in this pill has an unseen bell notification.
    pub attention: bool,
}

// ── Public render functions ────────────────────────────────────────────────────

/// Build a compositor [`Layer`] for the top menubar row (the v2 taskbar).
///
/// The layer is 1 row tall, `width` columns wide, positioned at `(0, 0)`.
/// Left to right: the brand button, the view-mode toggle, the assistant button,
/// a "+" new-shell button, then the open-window taskbar pills; the status tray
/// occupies the right edge. Pills shrink to badge-only and finally collapse into
/// a "…+N" overflow marker when they would collide with the tray.
pub fn render_menubar(width: i32, items: &[DockItem], segments: &[crate::tray::Segment], simple: bool) -> Layer {
    let t = crate::theme::current();
    let mut buf = CellBuffer::new(width, 1);
    buf.fill(crate::cell::Cell { ch: ' ', fg: t.text, bg: t.menubar_bg, attrs: Default::default() });
    buf.write_str(0, 0, GO_LABEL, t.accent, t.active_bg);
    let mode = if simple { MODE_SIMPLE } else { MODE_DESKTOP };
    buf.write_str(MODE_X, 0, mode, t.accent, t.active_bg);
    buf.write_str(ASSIST_X, 0, ASSIST_LABEL, t.accent, t.active_bg);
    // The "+" new-shell button, just right of the assistant.
    buf.write_str(NEW_SHELL_X, 0, NEW_SHELL_LABEL, crate::cell::Rgba::rgb(255, 255, 255), t.accent);
    // Status-tray segments occupy the right side, out to the screen edge (the
    // power button moved into the launcher's system section).
    let tray_left = menubar_tray_left(width, segments);
    for s in segments {
        buf.write_str(s.rect.x, 0, &s.text, t.text, t.menubar_bg);
    }
    // Taskbar pills fill the gap between the "+" button and the tray.
    let (pills, marker) = taskbar_layout(TASKBAR_X, tray_left - 1, items);
    for (idx, r, badge_x, label_text) in &pills {
        let item = &items[*idx];
        let bg = if item.focused { t.active_bg } else { t.menubar_bg };
        buf.write_str(r.x, 0, label_text, t.text, bg);
        // Bell-notification dot at the pill's right edge.
        if item.attention {
            buf.set(r.x + r.w - 1, 0, crate::cell::Cell { ch: '•', fg: t.accent, bg, attrs: Default::default() });
        }
        // Overwrite the badge cell with the badge color.
        buf.set(*badge_x, 0, crate::cell::Cell {
            ch: item.badge_letter,
            fg: crate::cell::Rgba::rgb(255, 255, 255),
            bg: item.badge_color,
            attrs: Default::default(),
        });
    }
    if let Some((r, m)) = marker {
        buf.write_str(r.x, 0, &m, t.dim, t.menubar_bg);
    }
    Layer { z: 1000, origin: Point::new(0, 0), buf, opacity: 1.0, scissor: None }
}

/// Screen-space hit region for the menubar brand ("tetron") button, used to open
/// the launcher dropdown. Top-left of the menubar.
pub fn menubar_brand_region() -> Rect {
    Rect::new(0, 0, GO_LABEL.chars().count() as i32, 1)
}

/// Screen-space hit region for the menubar view-mode toggle (just right of the brand).
pub fn menubar_mode_region() -> Rect {
    Rect::new(MODE_X, 0, MODE_DESKTOP.chars().count() as i32, 1)
}

/// Screen-space hit region for the assistant (✦) button — opens the AI chat panel.
pub fn menubar_assistant_region() -> Rect {
    Rect::new(ASSIST_X, 0, ASSIST_LABEL.chars().count() as i32, 1)
}

/// The "new shell" quick-launch button in the menubar (" + ").
const NEW_SHELL_LABEL: &str = " + ";

/// Screen-space hit region for the menubar's "+" (new shell) button.
pub fn menubar_new_shell_region() -> Rect {
    Rect::new(NEW_SHELL_X, 0, NEW_SHELL_LABEL.chars().count() as i32, 1)
}

/// Leftmost column of the status tray (or `width` when the tray is empty). The
/// taskbar pills fill the space up to one cell before this.
pub fn menubar_tray_left(width: i32, segments: &[crate::tray::Segment]) -> i32 {
    segments.iter().map(|s| s.rect.x).min().unwrap_or(width)
}

/// Return `(pill_index, Rect)` hit regions in *screen* coordinates (row 0).
///
/// The caller uses these to translate a taskbar click into a pill index, then
/// looks up `items[pill_index].kind` to decide what to do. The "…+N" overflow
/// marker is not clickable (taskbar scroll is a future addition).
pub fn menubar_taskbar_regions(width: i32, items: &[DockItem], segments: &[crate::tray::Segment]) -> Vec<(usize, Rect)> {
    let tray_left = menubar_tray_left(width, segments);
    let (pills, _marker) = taskbar_layout(TASKBAR_X, tray_left - 1, items);
    pills.into_iter().map(|(idx, r, _, _)| (idx, r)).collect()
}

/// Render a small bordered popup BELOW the taskbar for a window-group chooser.
///
/// Returns `(layers, row_rects)` where each entry in `row_rects` is the
/// screen-space rect of one window row (for hit-testing).
pub fn render_dock_popup(
    width: i32,
    height: i32,
    pill_x: i32,
    pill_w: i32,
    rows: &[(WindowId, char, crate::cell::Rgba, String)], // (id, badge_letter, badge_color, label)
    focused_id: Option<WindowId>,
) -> (Vec<Layer>, Vec<(WindowId, Rect)>) {
    let t = crate::theme::current();
    let n = rows.len() as i32;
    let box_h = (n + 2).min((height - 1).max(2)); // border rows, clamped to screen
    let max_label_w = rows.iter().map(|(_, _, _, l)| l.chars().count()).max().unwrap_or(4) as i32;
    let box_w = (max_label_w + 5).max(12).min(width); // badge + space + label + borders + padding
    // Anchor left edge at pill_x but clamp to screen
    let bx = pill_x.min(width - box_w).max(0);
    // Drop down from the menubar/taskbar row (row 0).
    let by = 1;
    let rect = Rect::new(bx, by, box_w, box_h);

    let mut buf = CellBuffer::new(rect.w, rect.h);
    buf.fill(crate::cell::Cell { ch: ' ', fg: t.text, bg: t.window_bg, attrs: Default::default() });

    // Border
    let b = |ch: char| crate::cell::Cell { ch, fg: t.border, bg: t.window_bg, attrs: Default::default() };
    for x in 0..rect.w {
        buf.set(x, 0, b('─'));
        buf.set(x, rect.h - 1, b('─'));
    }
    for y in 0..rect.h {
        buf.set(0, y, b('│'));
        buf.set(rect.w - 1, y, b('│'));
    }
    buf.set(0, 0, b('╭'));
    buf.set(rect.w - 1, 0, b('╮'));
    buf.set(0, rect.h - 1, b('╰'));
    buf.set(rect.w - 1, rect.h - 1, b('╯'));

    let mut row_rects = Vec::new();
    for (ri, (win_id, badge_letter, badge_color, label)) in rows.iter().enumerate() {
        let y = 1 + ri as i32;
        let is_focused = Some(*win_id) == focused_id;
        let row_bg = if is_focused { t.active_bg } else { t.window_bg };
        // Fill row background inside borders
        for x in 1..rect.w - 1 {
            buf.set(x, y, crate::cell::Cell { ch: ' ', fg: t.text, bg: row_bg, attrs: Default::default() });
        }
        // Badge cell
        buf.set(1, y, crate::cell::Cell {
            ch: *badge_letter,
            fg: crate::cell::Rgba::rgb(255, 255, 255),
            bg: *badge_color,
            attrs: Default::default(),
        });
        // Space + label
        let avail = (rect.w - 4).max(0) as usize;
        let lbl: String = label.chars().take(avail).collect();
        buf.write_str(3, y, &lbl, t.text, row_bg);

        let screen_row = Rect::new(rect.x, rect.y + y, rect.w, 1);
        row_rects.push((*win_id, screen_row));
    }

    let _ = (pill_w, focused_id); // suppress unused warnings
    let layer = Layer { z: 5200, origin: Point::new(rect.x, rect.y), buf, opacity: 1.0, scissor: None };
    (vec![layer], row_rects)
}

// ── Private helpers ────────────────────────────────────────────────────────────

/// A laid-out taskbar pill: `(item_index, local_rect, badge_x, pill_string)`.
type PillLayout = (usize, Rect, i32, String);

/// Superscript digit suffix for group counts, with a leading space (used in the
/// full-label pill, e.g. `" ²"`).
fn count_suffix(n: usize) -> String {
    let s = count_badge_suffix(n);
    if s.is_empty() { String::new() } else { format!(" {s}") }
}

/// Compact group-count suffix with no leading space (used in the badge-only
/// pill, e.g. `"²"` or `"·12"`).
fn count_badge_suffix(n: usize) -> String {
    const SUP: [char; 10] = ['⁰','¹','²','³','⁴','⁵','⁶','⁷','⁸','⁹'];
    if n <= 1 { String::new() }
    else if n <= 9 { SUP[n].to_string() }
    else { format!("\u{00B7}{n}") }
}

/// The "…+N" overflow marker shown when even badge-only pills do not all fit.
fn marker_text(hidden: usize) -> String {
    format!(" \u{2026}+{hidden} ")
}

/// Lay out `texts` (as `(item_index, pill_string)`) left to right from `start_x`
/// with a 1-cell gap between pills. Returns `Some(layout)` only if everything
/// fits within `avail` columns, where each entry is
/// `(item_index, local_rect, badge_x, pill_string)` and `badge_x` is the pill's
/// badge cell (second char, after the leading space).
fn lay_pills(start_x: i32, avail: i32, texts: &[(usize, String)]) -> Option<Vec<PillLayout>> {
    if texts.is_empty() { return Some(Vec::new()); }
    let total: i32 = texts.iter().map(|(_, s)| s.chars().count() as i32).sum::<i32>()
        + texts.len() as i32 - 1; // 1-cell gaps between pills
    if total > avail { return None; }
    let mut out = Vec::new();
    let mut x = start_x;
    for (idx, s) in texts {
        let w = s.chars().count() as i32;
        out.push((*idx, Rect::new(x, 0, w, 1), x + 1, s.clone()));
        x += w + 1;
    }
    Some(out)
}

/// Compute the taskbar-pill layout for the span `[start_x, end_x)`.
///
/// Three progressively tighter modes, first that fits wins:
/// 1. full `" B label[²] "` pills,
/// 2. badge-only `" B[²] "` pills,
/// 3. as many badge-only pills as fit plus a trailing `" …+N "` overflow marker.
///
/// Returns `(pills, overflow_marker)` where `pills` is the `lay_pills` layout and
/// the marker (when present) is its `(local_rect, text)` — it is drawn but not
/// clickable.
///
// TODO(taskbar-scroll): make the "…+N" marker a scroll affordance (wheel over
// the taskbar, or click to cycle) so a very long window list stays reachable
// rather than silently hidden behind the badge-only overflow.
fn taskbar_layout(start_x: i32, end_x: i32, items: &[DockItem]) -> (Vec<PillLayout>, Option<(Rect, String)>) {
    let avail = (end_x - start_x).max(0);
    if avail <= 0 || items.is_empty() { return (Vec::new(), None); }

    // 1) Full labels.
    let full: Vec<(usize, String)> = items.iter().enumerate()
        .map(|(i, it)| (i, format!(" {} {}{} ", it.badge_letter, it.label, count_suffix(it.count))))
        .collect();
    if let Some(l) = lay_pills(start_x, avail, &full) { return (l, None); }

    // 2) Badge-only.
    let badges: Vec<(usize, String)> = items.iter().enumerate()
        .map(|(i, it)| (i, format!(" {}{} ", it.badge_letter, count_badge_suffix(it.count))))
        .collect();
    if let Some(l) = lay_pills(start_x, avail, &badges) { return (l, None); }

    // 3) Badge-only + "…+N" marker: place the most badges that leave room for a
    //    marker covering the rest.
    for k in (1..badges.len()).rev() {
        let placed = &badges[..k];
        let hidden = badges.len() - k;
        let marker = marker_text(hidden);
        let placed_w: i32 = placed.iter().map(|(_, s)| s.chars().count() as i32).sum::<i32>()
            + placed.len() as i32 - 1; // gaps between the placed badges
        let total = placed_w + 1 + marker.chars().count() as i32; // gap + marker
        if total <= avail {
            let l = lay_pills(start_x, avail, placed).unwrap();
            let mx = l.last().map(|(_, r, _, _)| r.x + r.w + 1).unwrap_or(start_x);
            let mw = marker.chars().count() as i32;
            return (l, Some((Rect::new(mx, 0, mw, 1), marker)));
        }
    }

    // Not even one badge + marker fits: show the marker for all items.
    let marker = marker_text(badges.len());
    let mw = (marker.chars().count() as i32).min(avail);
    (Vec::new(), Some((Rect::new(start_x, 0, mw, 1), marker)))
}
