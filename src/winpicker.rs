//! The window picker: a centered, numbered list of every open window (minimized
//! ones included) for keyboard users. Leader then `f` opens it; arrows / Tab /
//! j k move the highlight, Enter or a digit 1-9 focuses that window, Esc closes.
//! A click on a row picks it, a click outside dismisses.
//!
//! Like [`crate::confirmclose`], the widget is pure UI state + geometry. It never
//! touches the window manager: picking reports the [`WindowId`] and the session
//! focuses (and restores) it.

use crate::buffer::CellBuffer;
use crate::cell::Cell;
use crate::compositor::Layer;
use crate::geometry::{Point, Rect};
use crate::window::WindowId;

/// One row of the picker: the window, its label, and whether it is minimized.
#[derive(Clone, Debug, PartialEq)]
pub struct PickRow {
    pub id: WindowId,
    pub label: String,
    pub minimized: bool,
}

/// Picker state. `rows` empty means closed.
#[derive(Default)]
pub struct WinPicker {
    rows: Vec<PickRow>,
    sel: usize,
}

/// Footer hint. Plain ASCII so it reads the same on every console.
const HINT: &str = "Enter/1-9 pick  Esc close";

// ── Geometry (shared by render + hit-test so they can never drift) ──────────────

/// The centered box: one row per window, plus title, footer and border rows.
/// Wide enough for the longest label and the hint, clamped to the screen.
fn box_rect(rows: &[PickRow], w: i32, h: i32) -> Rect {
    let longest = rows.iter().map(|r| r.label.chars().count()).max().unwrap_or(0) as i32;
    // "  9  " prefix (5) + label + "  (min)" suffix (7) + side borders (2).
    let want_w = (longest + 14).max(HINT.len() as i32 + 4).max(30);
    let box_w = want_w.min((w - 2).max(10));
    let box_h = (rows.len() as i32 + 3).min((h - 1).max(4));
    Rect::new((w - box_w) / 2, ((h - box_h) / 2).max(0), box_w, box_h)
}

/// How many rows fit between the title row and the footer row.
fn visible_rows(rows: &[PickRow], w: i32, h: i32) -> usize {
    (box_rect(rows, w, h).h - 3).max(0) as usize
}

/// First row index shown, so the selection is always on screen.
fn scroll_top(rows: &[PickRow], sel: usize, w: i32, h: i32) -> usize {
    let vis = visible_rows(rows, w, h).max(1);
    sel.saturating_sub(vis - 1)
}

/// Screen rect of list row `i` (an index into `rows`), if it is on screen.
fn row_rect(rows: &[PickRow], sel: usize, i: usize, w: i32, h: i32) -> Option<Rect> {
    let b = box_rect(rows, w, h);
    let top = scroll_top(rows, sel, w, h);
    if i < top || i >= top + visible_rows(rows, w, h) {
        return None;
    }
    Some(Rect::new(b.x + 1, b.y + 1 + (i - top) as i32, b.w - 2, 1))
}

impl WinPicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the picker is showing (modal while true).
    pub fn is_open(&self) -> bool {
        !self.rows.is_empty()
    }

    /// Open with `rows`, highlighting row `sel` (clamped). Empty `rows` keeps
    /// it closed: there is nothing to pick.
    pub fn open(&mut self, rows: Vec<PickRow>, sel: usize) {
        self.sel = sel.min(rows.len().saturating_sub(1));
        self.rows = rows;
    }

    pub fn close(&mut self) {
        self.rows.clear();
        self.sel = 0;
    }

    /// The highlighted row index (for tests and rendering).
    pub fn selected(&self) -> usize {
        self.sel
    }

    /// Move the highlight by `d` rows, wrapping at both ends.
    pub fn step(&mut self, d: i32) {
        let n = self.rows.len() as i32;
        if n > 0 {
            self.sel = (self.sel as i32 + d).rem_euclid(n) as usize;
        }
    }

    /// Enter: close and report the highlighted window.
    pub fn confirm(&mut self) -> Option<WindowId> {
        let id = self.rows.get(self.sel).map(|r| r.id);
        self.close();
        id
    }

    /// Digit `n` (1-based): close and report row `n`. Out of range does nothing,
    /// so a stray digit can not dismiss the list.
    pub fn pick_number(&mut self, n: u8) -> Option<WindowId> {
        let i = (n as usize).checked_sub(1)?;
        let id = self.rows.get(i)?.id;
        self.close();
        Some(id)
    }

    /// Route a click while open: a row picks that window, anywhere outside the
    /// box dismisses, inside the box (title, footer) is consumed.
    pub fn on_click(&mut self, p: Point, w: i32, h: i32) -> Option<WindowId> {
        if !self.is_open() {
            return None;
        }
        let hit = (0..self.rows.len()).find(|&i| row_rect(&self.rows, self.sel, i, w, h).is_some_and(|r| r.contains(p)));
        if let Some(i) = hit {
            let id = self.rows[i].id;
            self.close();
            return Some(id);
        }
        if !box_rect(&self.rows, w, h).contains(p) {
            self.close();
        }
        None
    }

    /// Compositor layer for the picker (empty when closed). Same chrome as the
    /// confirm-close dialog: a title bar in the focused-title colour, a border,
    /// and the selection in `active_bg`.
    pub fn render(&self, w: i32, h: i32) -> Vec<Layer> {
        if !self.is_open() {
            return Vec::new();
        }
        let t = crate::theme::current();
        let b = box_rect(&self.rows, w, h);
        let mut buf = CellBuffer::new(b.w, b.h);
        buf.fill(Cell { ch: ' ', fg: t.text, bg: t.window_bg, attrs: Default::default() });
        // Title bar.
        for x in 0..b.w {
            buf.set(x, 0, Cell { ch: ' ', fg: t.title_fg, bg: t.title_focus, attrs: Default::default() });
        }
        buf.write_str(2, 0, " Windows ", t.title_fg, t.title_focus);
        // Border.
        let edge = |ch: char| Cell { ch, fg: t.border, bg: t.window_bg, attrs: Default::default() };
        for y in 1..b.h {
            buf.set(0, y, edge('│'));
            buf.set(b.w - 1, y, edge('│'));
        }
        for x in 0..b.w {
            buf.set(x, b.h - 1, edge('─'));
        }
        buf.set(0, b.h - 1, edge('╰'));
        buf.set(b.w - 1, b.h - 1, edge('╯'));
        // Rows: "  N  label  (min)", N only for the first nine (the digit keys).
        let inner = (b.w - 2).max(0) as usize;
        for (i, row) in self.rows.iter().enumerate() {
            let Some(r) = row_rect(&self.rows, self.sel, i, w, h) else { continue };
            let num = if i < 9 { format!("  {}  ", i + 1) } else { "     ".to_string() };
            let tag = if row.minimized { "  (min)" } else { "" };
            let room = inner.saturating_sub(num.len() + tag.len());
            let label: String = if row.label.chars().count() > room {
                let mut s: String = row.label.chars().take(room.saturating_sub(1)).collect();
                s.push('…');
                s
            } else {
                row.label.clone()
            };
            let line = format!("{num}{label}{tag}");
            let line = format!("{line:<inner$}");
            let (fg, bg) = if i == self.sel { (t.text, t.active_bg) } else { (t.text, t.window_bg) };
            buf.write_str(1, r.y - b.y, &line, fg, bg);
        }
        // Footer hint, centered in the row above the bottom border.
        let hint: String = HINT.chars().take(inner).collect();
        let hx = 1 + (inner.saturating_sub(hint.len()) / 2) as i32;
        buf.write_str(hx, b.h - 2, &hint, t.dim, t.window_bg);
        vec![Layer { z: 6000, origin: Point::new(b.x, b.y), buf, opacity: 1.0, scissor: None }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(n: u64) -> Vec<PickRow> {
        (1..=n).map(|i| PickRow { id: WindowId(i), label: format!("win {i}"), minimized: i == n }).collect()
    }

    #[test]
    fn empty_list_stays_closed() {
        let mut p = WinPicker::new();
        p.open(Vec::new(), 0);
        assert!(!p.is_open());
        assert!(p.render(80, 24).is_empty());
    }

    #[test]
    fn step_wraps_and_enter_reports() {
        let mut p = WinPicker::new();
        p.open(rows(3), 0);
        p.step(-1);
        assert_eq!(p.selected(), 2);
        p.step(1);
        assert_eq!(p.selected(), 0);
        p.step(1);
        assert_eq!(p.confirm(), Some(WindowId(2)));
        assert!(!p.is_open());
    }

    #[test]
    fn digit_picks_and_out_of_range_is_ignored() {
        let mut p = WinPicker::new();
        p.open(rows(3), 0);
        assert_eq!(p.pick_number(7), None);
        assert!(p.is_open());
        assert_eq!(p.pick_number(3), Some(WindowId(3)));
        assert!(!p.is_open());
    }

    #[test]
    fn click_row_picks_and_outside_dismisses() {
        let (w, h) = (100, 30);
        let mut p = WinPicker::new();
        p.open(rows(4), 0);
        let r = row_rect(&p.rows, p.sel, 1, w, h).unwrap();
        assert_eq!(p.on_click(Point::new(r.x + 3, r.y), w, h), Some(WindowId(2)));
        p.open(rows(4), 0);
        assert_eq!(p.on_click(Point::new(0, 0), w, h), None);
        assert!(!p.is_open());
    }

    #[test]
    fn long_list_scrolls_to_keep_selection_visible() {
        let (w, h) = (80, 10);
        let mut p = WinPicker::new();
        p.open(rows(20), 19);
        assert!(row_rect(&p.rows, p.sel, 19, w, h).is_some());
        assert!(row_rect(&p.rows, p.sel, 0, w, h).is_none());
    }
}
