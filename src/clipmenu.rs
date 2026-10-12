//! The clipboard history popover opened from the menubar clipboard segment (or
//! leader + `v`). Pure state + rendering + hit-testing: it never touches the
//! clipboard itself, it reports a [`ClipAction`] and the session acts on it.
//!
//! Layout (render and hit-test share [`layout`]):
//! ```text
//! ╭ Clipboard (12) ───────────────────────╮
//! │ / filter█                             │
//! │───────────────────────────────────────│
//! │ ★ sel  some copied text…            ✕ │
//! │ ☆ app  another entry                ✕ │
//! │───────────────────────────────────────│
//! │ preview of the highlighted entry      │
//! │ [Pause] [Clear]  Enter paste · Alt+c … │
//! ╰───────────────────────────────────────╯
//! ```

use crate::buffer::CellBuffer;
use crate::cell::Cell;
use crate::clipboard::ClipHistory;
use crate::compositor::Layer;
use crate::geometry::{Point, Rect};

/// Rows of history shown at once.
const MAX_ROWS: usize = 10;
const BOX_W: i32 = 62;

/// What the user asked for; indices are into the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipAction {
    /// Make it the clipboard and paste it into the focused app.
    Paste(usize),
    /// Make it the clipboard only.
    CopyOnly(usize),
    TogglePin(usize),
    Delete(usize),
    TogglePause,
    Clear,
    Close,
}

/// Keys the client forwards while the popover is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClipKey {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Copy,
    Pin,
    Delete,
    Pause,
    Backspace,
    Esc,
    Char(char),
}

#[derive(Default)]
pub struct ClipMenu {
    open: bool,
    anchor_x: i32,
    /// Position within the filtered list.
    selected: usize,
    filter: String,
}

/// Screen geometry for one frame, shared by render and hit-testing.
pub struct Layout {
    pub bounds: Rect,
    /// `(rect, history index)` for every visible row.
    pub rows: Vec<(Rect, usize)>,
    pub pause_btn: Rect,
    pub clear_btn: Rect,
    /// First filtered position shown.
    pub offset: usize,
    pub total: usize,
}

/// Per-row pin and delete hit zones (x offsets within the row rect).
fn pin_zone(row: Rect) -> Rect { Rect::new(row.x, row.y, 3, 1) }
fn del_zone(row: Rect) -> Rect { Rect::new(row.x + row.w - 3, row.y, 3, 1) }

impl ClipMenu {
    pub fn is_open(&self) -> bool { self.open }

    pub fn open(&mut self, anchor_x: i32) {
        self.open = true;
        self.anchor_x = anchor_x;
        self.selected = 0;
        self.filter.clear();
    }

    pub fn close(&mut self) { self.open = false; }

    pub fn toggle(&mut self, anchor_x: i32) {
        if self.open { self.close() } else { self.open(anchor_x) }
    }

    fn matches(&self, h: &ClipHistory) -> Vec<usize> { h.matching(&self.filter) }

    fn clamp(&mut self, h: &ClipHistory) {
        let n = self.matches(h).len();
        self.selected = self.selected.min(n.saturating_sub(1));
    }

    /// The highlighted entry's history index.
    pub fn selected_index(&self, h: &ClipHistory) -> Option<usize> {
        self.matches(h).get(self.selected).copied()
    }

    /// Apply a key; returns the action to perform, if any.
    pub fn key(&mut self, k: ClipKey, h: &ClipHistory) -> Option<ClipAction> {
        let n = self.matches(h).len();
        match k {
            ClipKey::Up => self.selected = self.selected.saturating_sub(1),
            ClipKey::Down => self.selected = (self.selected + 1).min(n.saturating_sub(1)),
            ClipKey::PageUp => self.selected = self.selected.saturating_sub(MAX_ROWS),
            ClipKey::PageDown => self.selected = (self.selected + MAX_ROWS).min(n.saturating_sub(1)),
            ClipKey::Home => self.selected = 0,
            ClipKey::End => self.selected = n.saturating_sub(1),
            ClipKey::Enter => return self.selected_index(h).map(ClipAction::Paste),
            ClipKey::Copy => return self.selected_index(h).map(ClipAction::CopyOnly),
            ClipKey::Pin => return self.selected_index(h).map(ClipAction::TogglePin),
            ClipKey::Delete => return self.selected_index(h).map(ClipAction::Delete),
            ClipKey::Pause => return Some(ClipAction::TogglePause),
            ClipKey::Backspace => {
                self.filter.pop();
                self.selected = 0;
            }
            ClipKey::Esc => {
                if self.filter.is_empty() {
                    return Some(ClipAction::Close);
                }
                self.filter.clear();
                self.selected = 0;
            }
            ClipKey::Char(c) => {
                if !c.is_control() {
                    self.filter.push(c);
                    self.selected = 0;
                }
            }
        }
        self.clamp(h);
        None
    }

    /// After the history changed under the menu (delete, clear).
    pub fn history_changed(&mut self, h: &ClipHistory) { self.clamp(h); }

    pub fn layout(&self, w: i32, h: i32, hist: &ClipHistory) -> Layout {
        let m = self.matches(hist);
        let vis = m.len().clamp(1, MAX_ROWS).min((h - 9).max(1) as usize);
        let offset = (self.selected + 1).saturating_sub(vis).min(m.len().saturating_sub(vis));
        let box_w = BOX_W.min(w - 2).max(30);
        let box_h = 3 + vis as i32 + 4; // title+filter+sep, rows, sep+preview+hints+border
        let x = self.anchor_x.min(w - box_w - 1).max(0);
        let origin = Point::new(x, 1);
        let rows: Vec<(Rect, usize)> = m
            .iter()
            .skip(offset)
            .take(vis)
            .enumerate()
            .map(|(i, hi)| (Rect::new(origin.x + 1, origin.y + 3 + i as i32, box_w - 2, 1), *hi))
            .collect();
        let hint_y = origin.y + box_h - 2;
        Layout {
            bounds: Rect::new(origin.x, origin.y, box_w, box_h),
            rows,
            pause_btn: Rect::new(origin.x + 2, hint_y, 7, 1),
            clear_btn: Rect::new(origin.x + 10, hint_y, 7, 1),
            offset,
            total: m.len(),
        }
    }

    /// What a click at `p` does. Outside the popover closes it.
    pub fn click(&self, p: Point, w: i32, h: i32, hist: &ClipHistory) -> Option<ClipAction> {
        let l = self.layout(w, h, hist);
        if !l.bounds.contains(p) {
            return Some(ClipAction::Close);
        }
        if l.pause_btn.contains(p) {
            return Some(ClipAction::TogglePause);
        }
        if l.clear_btn.contains(p) {
            return Some(ClipAction::Clear);
        }
        for (r, hi) in &l.rows {
            if r.contains(p) {
                return Some(if pin_zone(*r).contains(p) {
                    ClipAction::TogglePin(*hi)
                } else if del_zone(*r).contains(p) {
                    ClipAction::Delete(*hi)
                } else {
                    ClipAction::Paste(*hi)
                });
            }
        }
        None
    }

    pub fn render(&self, w: i32, h: i32, hist: &ClipHistory, paused: bool) -> Vec<Layer> {
        if !self.open {
            return Vec::new();
        }
        let t = crate::theme::current();
        let l = self.layout(w, h, hist);
        let (bw, bh) = (l.bounds.w, l.bounds.h);
        let mut buf = CellBuffer::new(bw, bh);
        let blank = Cell { ch: ' ', fg: t.text, bg: t.window_bg, attrs: Default::default() };
        buf.fill(blank);
        let b = |ch: char| Cell { ch, fg: t.border, bg: t.window_bg, attrs: Default::default() };
        for x in 0..bw {
            buf.set(x, 0, b('─'));
            buf.set(x, bh - 1, b('─'));
        }
        for y in 0..bh {
            buf.set(0, y, b('│'));
            buf.set(bw - 1, y, b('│'));
        }
        buf.set(0, 0, b('╭'));
        buf.set(bw - 1, 0, b('╮'));
        buf.set(0, bh - 1, b('╰'));
        buf.set(bw - 1, bh - 1, b('╯'));
        let title = format!(" Clipboard ({}){} ", hist.len(), if paused { " paused" } else { "" });
        buf.write_str(2, 0, &title, t.accent, t.window_bg);

        buf.write_str(2, 1, &format!("/ {}\u{2588}", self.filter), t.text, t.window_bg);
        for x in 1..bw - 1 {
            buf.set(x, 2, b('─'));
        }

        let sel_pos = self.selected;
        if l.rows.is_empty() {
            let msg = if hist.is_empty() { "nothing copied yet" } else { "no matches" };
            buf.write_str(2, 3, msg, t.text, t.window_bg);
        }
        for (i, (r, hi)) in l.rows.iter().enumerate() {
            let y = 3 + i as i32;
            let e = &hist.entries()[*hi];
            let highlighted = l.offset + i == sel_pos;
            let (fg, bg) = if highlighted { (t.text, t.active_bg) } else { (t.text, t.window_bg) };
            for x in 1..bw - 1 {
                buf.set(x, y, Cell { ch: ' ', fg, bg, attrs: Default::default() });
            }
            buf.write_str(2, y, if e.pinned { "\u{2605}" } else { "\u{2606}" }, t.accent, bg);
            buf.write_str(4, y, e.source.tag(), t.border, bg);
            let room = (r.w - 14).max(4) as usize;
            buf.write_str(9, y, &crate::clipboard::preview(&e.text, room), fg, bg);
            buf.write_str(bw - 4, y, "\u{2715}", t.close_fg, bg);
        }
        if l.total > l.rows.len() {
            if l.offset > 0 {
                buf.write_str(bw - 2, 2, "\u{25B2}", t.accent, t.window_bg);
            }
            if l.offset + l.rows.len() < l.total {
                buf.write_str(bw - 2, bh - 4, "\u{25BC}", t.accent, t.window_bg);
            }
        }

        let sep_y = bh - 4;
        for x in 1..bw - 1 {
            buf.set(x, sep_y, b('─'));
        }
        let prev = self
            .selected_index(hist)
            .map(|i| hist.entries()[i].text.lines().next().unwrap_or("").to_string())
            .unwrap_or_default();
        buf.write_str(2, bh - 3, &crate::clipboard::preview(&prev, (bw - 4) as usize), t.text, t.window_bg);
        let hint_y = bh - 2;
        buf.write_str(2, hint_y, if paused { "[Resume]" } else { "[Pause] " }, t.accent, t.window_bg);
        buf.write_str(10, hint_y, "[Clear]", t.accent, t.window_bg);
        buf.write_str(19, hint_y, "Enter paste \u{B7} Alt+c copy \u{B7} p pin \u{B7} d del", t.border, t.window_bg);
        vec![Layer { z: 5200, origin: Point::new(l.bounds.x, l.bounds.y), buf, opacity: 1.0, scissor: None }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::{ClipLimits, ClipSource};

    fn hist(n: usize) -> ClipHistory {
        let mut h = ClipHistory::new(ClipLimits::default());
        for i in 0..n {
            h.record(&format!("entry {i}"), ClipSource::Copy);
        }
        h
    }

    #[test]
    fn keys_navigate_and_clamp() {
        let h = hist(3);
        let mut m = ClipMenu::default();
        m.open(0);
        m.key(ClipKey::Down, &h);
        assert_eq!(m.selected_index(&h), Some(1));
        m.key(ClipKey::End, &h);
        m.key(ClipKey::Down, &h);
        assert_eq!(m.selected_index(&h), Some(2));
        m.key(ClipKey::Home, &h);
        m.key(ClipKey::Up, &h);
        assert_eq!(m.selected_index(&h), Some(0));
    }

    #[test]
    fn enter_pastes_selected_newest_first() {
        let h = hist(3);
        let mut m = ClipMenu::default();
        m.open(0);
        assert_eq!(h.get(0).unwrap().text, "entry 2");
        assert_eq!(m.key(ClipKey::Enter, &h), Some(ClipAction::Paste(0)));
        m.key(ClipKey::Down, &h);
        assert_eq!(m.key(ClipKey::Copy, &h), Some(ClipAction::CopyOnly(1)));
        assert_eq!(m.key(ClipKey::Pin, &h), Some(ClipAction::TogglePin(1)));
        assert_eq!(m.key(ClipKey::Delete, &h), Some(ClipAction::Delete(1)));
        assert_eq!(m.key(ClipKey::Pause, &h), Some(ClipAction::TogglePause));
    }

    #[test]
    fn typing_filters_and_esc_clears_then_closes() {
        let h = hist(12);
        let mut m = ClipMenu::default();
        m.open(0);
        for c in "entry 1".chars() {
            m.key(ClipKey::Char(c), &h);
        }
        // "entry 1", "entry 10", "entry 11" match.
        assert_eq!(h.matching("entry 1").len(), 3);
        assert_eq!(m.layout(100, 40, &h).total, 3);
        assert_eq!(m.key(ClipKey::Esc, &h), None, "first Esc clears the filter");
        assert_eq!(m.layout(100, 40, &h).total, 12);
        assert_eq!(m.key(ClipKey::Esc, &h), Some(ClipAction::Close));
        m.key(ClipKey::Char('\n'), &h);
        assert_eq!(m.layout(100, 40, &h).total, 12, "control chars are ignored");
    }

    #[test]
    fn layout_scrolls_to_keep_selection_visible() {
        let h = hist(30);
        let mut m = ClipMenu::default();
        m.open(0);
        for _ in 0..15 {
            m.key(ClipKey::Down, &h);
        }
        let l = m.layout(100, 40, &h);
        assert_eq!(l.rows.len(), MAX_ROWS);
        assert!(l.rows.iter().any(|(_, hi)| *hi == 15), "selected row is visible");
        assert!(l.bounds.y + l.bounds.h <= 40);
    }

    #[test]
    fn clicks_hit_the_right_zones() {
        let h = hist(3);
        let mut m = ClipMenu::default();
        m.open(5);
        let l = m.layout(100, 40, &h);
        let (r, hi) = l.rows[1];
        let mid = Point::new(r.x + 10, r.y);
        assert_eq!(m.click(mid, 100, 40, &h), Some(ClipAction::Paste(hi)));
        assert_eq!(m.click(Point::new(r.x + 1, r.y), 100, 40, &h), Some(ClipAction::TogglePin(hi)));
        assert_eq!(m.click(Point::new(r.x + r.w - 2, r.y), 100, 40, &h), Some(ClipAction::Delete(hi)));
        assert_eq!(m.click(Point::new(l.pause_btn.x, l.pause_btn.y), 100, 40, &h), Some(ClipAction::TogglePause));
        assert_eq!(m.click(Point::new(l.clear_btn.x, l.clear_btn.y), 100, 40, &h), Some(ClipAction::Clear));
        assert_eq!(m.click(Point::new(0, 30), 100, 40, &h), Some(ClipAction::Close), "outside closes");
    }

    #[test]
    fn renders_within_bounds_even_when_empty() {
        let h = ClipHistory::new(ClipLimits::default());
        let mut m = ClipMenu::default();
        m.open(90);
        let layers = m.render(100, 30, &h, false);
        assert_eq!(layers.len(), 1);
        let l = m.layout(100, 30, &h);
        assert!(l.bounds.x + l.bounds.w <= 100 && l.bounds.y + l.bounds.h <= 30);
        assert!(m.render(100, 30, &hist(50), true).len() == 1);
    }
}
