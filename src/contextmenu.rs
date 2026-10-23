//! The right-click menu shared by the desktop and the file manager: which
//! actions a target offers ([`actions_for`], [`open_with_actions`]) and one menu
//! widget ([`ContextMenu`]) that both surfaces draw and hit-test with the same
//! geometry. A new action is added here once and appears in both places.

use std::path::Path;

use crate::buffer::CellBuffer;
use crate::cell::Cell;
use crate::geometry::{Point, Rect};
use crate::openwith::Role;

/// Where the menu was opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Desktop,
    Files,
}

/// What the menu is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A file or folder (`pinned`: a desktop shortcut, not a real file).
    Entry { is_dir: bool, pinned: bool },
    /// Empty space (the desktop background).
    Background,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    /// Replace the menu with the apps that can open the entry.
    OpenWith,
    /// Open the entry with this handler (`@image` = the built-in viewer).
    OpenWithApp(String),
    Rename,
    Copy,
    Cut,
    Trash,
    GetInfo,
    Unpin,
    NewFolder,
    CleanUp,
}

impl MenuAction {
    pub fn label(&self) -> String {
        match self {
            MenuAction::Open => "Open".into(),
            MenuAction::OpenWith => "Open with\u{2026}".into(),
            MenuAction::OpenWithApp(cmd) if cmd == "@image" => "Image viewer".into(),
            MenuAction::OpenWithApp(cmd) => cmd.clone(),
            MenuAction::Rename => "Rename".into(),
            MenuAction::Copy => "Copy".into(),
            MenuAction::Cut => "Cut".into(),
            MenuAction::Trash => "Move to Trash".into(),
            MenuAction::GetInfo => "Get Info".into(),
            MenuAction::Unpin => "Unpin".into(),
            MenuAction::NewFolder => "New Folder".into(),
            MenuAction::CleanUp => "Clean Up".into(),
        }
    }
}

/// The actions `target` offers on `surface`, top to bottom.
pub fn actions_for(surface: Surface, target: Target) -> Vec<MenuAction> {
    use MenuAction as A;
    match (surface, target) {
        (Surface::Desktop, Target::Entry { pinned: true, .. }) => vec![A::Open, A::Unpin],
        (Surface::Desktop, Target::Entry { is_dir, .. }) => {
            let mut v = vec![A::Open];
            if !is_dir {
                v.push(A::OpenWith);
            }
            v.extend([A::Rename, A::Trash]);
            v
        }
        (Surface::Desktop, Target::Background) => vec![A::NewFolder, A::CleanUp],
        (Surface::Files, Target::Entry { is_dir, .. }) => {
            let mut v = vec![A::Open];
            if !is_dir {
                v.push(A::OpenWith);
            }
            v.extend([A::Rename, A::Copy, A::Cut, A::Trash, A::GetInfo]);
            v
        }
        (Surface::Files, Target::Background) => Vec::new(),
    }
}

/// The "Open with…" choices for `path`: the built-in viewer for images, then
/// the installed terminal apps from the Default Apps candidates.
pub fn open_with_actions(path: &Path) -> Vec<MenuAction> {
    let role = crate::openwith::classify(path, false);
    crate::openwith::candidates(role)
        .into_iter()
        .filter(|c| !c.is_empty() && c != "@navigate" && (c != "@image" || role == Role::Image))
        .map(MenuAction::OpenWithApp)
        .collect()
}

/// An open menu: its items, the highlighted row and where it was opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    pub anchor: Point,
    pub items: Vec<MenuAction>,
    pub sel: usize,
}

impl ContextMenu {
    pub fn new(anchor: Point, items: Vec<MenuAction>) -> Self {
        Self { anchor, items, sel: 0 }
    }

    /// Replace the items in place (the Open-with submenu), highlight the first.
    pub fn replace(&mut self, items: Vec<MenuAction>) {
        self.items = items;
        self.sel = 0;
    }

    /// The menu box: one row per item inside a one-cell border, as wide as the
    /// longest label, moved so it stays inside `bounds`. Render and hit-test
    /// both use this.
    pub fn rect(&self, bounds: Rect) -> Rect {
        let longest = self.items.iter().map(|a| a.label().chars().count()).max().unwrap_or(0) as i32;
        let w = (longest + 4).max(14).min(bounds.w.max(1));
        let h = (self.items.len() as i32 + 2).min(bounds.h.max(1));
        let x = self.anchor.x.clamp(bounds.x, (bounds.x + bounds.w - w).max(bounds.x));
        let y = self.anchor.y.clamp(bounds.y, (bounds.y + bounds.h - h).max(bounds.y));
        Rect::new(x, y, w, h)
    }

    /// The item row under `p`, if any.
    pub fn index_at(&self, p: Point, bounds: Rect) -> Option<usize> {
        let r = self.rect(bounds);
        if !r.contains(p) {
            return None;
        }
        let row = p.y - (r.y + 1);
        (row >= 0 && (row as usize) < self.items.len()).then_some(row as usize)
    }

    /// The item under `p`, if any.
    pub fn item_at(&self, p: Point, bounds: Rect) -> Option<&MenuAction> {
        self.index_at(p, bounds).map(|i| &self.items[i])
    }

    /// Highlight the row under `p` (mouse hover); off the rows keeps the highlight.
    pub fn hover(&mut self, p: Point, bounds: Rect) {
        if let Some(i) = self.index_at(p, bounds) {
            self.sel = i;
        }
    }

    pub fn up(&mut self) {
        let n = self.items.len().max(1);
        self.sel = (self.sel + n - 1) % n;
    }

    pub fn down(&mut self) {
        let n = self.items.len().max(1);
        self.sel = (self.sel + 1) % n;
    }

    pub fn selected(&self) -> Option<&MenuAction> {
        self.items.get(self.sel)
    }

    /// Draw the menu into `buf` (whose cell (0,0) is `bounds`' coordinate space
    /// origin, i.e. screen or window-content coordinates matching `bounds`).
    /// Opaque theme colours: a translucent blank cell would let the glyphs
    /// beneath (the wallpaper) show through.
    pub fn render(&self, buf: &mut CellBuffer, bounds: Rect) {
        let t = crate::theme::current();
        let (fg, bg) = (t.text, t.active_bg);
        let r = self.rect(bounds);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                buf.set(x, y, Cell { ch: ' ', fg, bg, attrs: Default::default() });
            }
        }
        for (i, item) in self.items.iter().enumerate() {
            let y = r.y + 1 + i as i32;
            if y >= r.y + r.h - 1 {
                break;
            }
            let (fg, bg) = if i == self.sel { (t.window_bg, t.accent) } else { (fg, bg) };
            for x in r.x + 1..r.x + r.w - 1 {
                buf.set(x, y, Cell { ch: ' ', fg, bg, attrs: Default::default() });
            }
            let label: String = item.label().chars().take((r.w - 3).max(0) as usize).collect();
            buf.write_str(r.x + 2, y, &label, fg, bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(n: usize) -> ContextMenu {
        ContextMenu::new(Point::new(5, 3), actions_for(Surface::Files, Target::Entry { is_dir: false, pinned: false }).into_iter().take(n).collect())
    }

    #[test]
    fn actions_differ_by_surface_and_target() {
        use MenuAction as A;
        let file = Target::Entry { is_dir: false, pinned: false };
        let dir = Target::Entry { is_dir: true, pinned: false };
        assert_eq!(actions_for(Surface::Desktop, file), vec![A::Open, A::OpenWith, A::Rename, A::Trash]);
        assert_eq!(actions_for(Surface::Desktop, dir), vec![A::Open, A::Rename, A::Trash], "no Open with for folders");
        assert_eq!(actions_for(Surface::Desktop, Target::Entry { is_dir: false, pinned: true }), vec![A::Open, A::Unpin]);
        assert_eq!(actions_for(Surface::Desktop, Target::Background), vec![A::NewFolder, A::CleanUp]);
        assert_eq!(actions_for(Surface::Files, file), vec![A::Open, A::OpenWith, A::Rename, A::Copy, A::Cut, A::Trash, A::GetInfo]);
        assert!(actions_for(Surface::Files, Target::Background).is_empty());
    }

    #[test]
    fn open_with_lists_the_viewer_only_for_images() {
        let img = open_with_actions(Path::new("/x/cat.png"));
        assert_eq!(img.first(), Some(&MenuAction::OpenWithApp("@image".into())));
        assert_eq!(img[0].label(), "Image viewer");
        let txt = open_with_actions(Path::new("/x/notes.txt"));
        assert!(!txt.contains(&MenuAction::OpenWithApp("@image".into())));
        assert!(!txt.iter().any(|a| matches!(a, MenuAction::OpenWithApp(c) if c.is_empty() || c == "@navigate")));
    }

    #[test]
    fn rect_fits_the_labels_and_stays_inside_the_bounds() {
        let m = menu(7);
        let b = Rect::new(0, 1, 80, 24);
        let r = m.rect(b);
        assert_eq!((r.x, r.y, r.h), (5, 3, 9));
        assert!(r.w >= "Move to Trash".len() as i32 + 4);
        // Anchored at the bottom-right corner: moved back inside.
        let mut m = m;
        m.anchor = Point::new(79, 24);
        let r = m.rect(b);
        assert!(r.x + r.w <= 80 && r.y + r.h <= 25, "{r:?}");
        assert!(r.y >= 1, "never over the menubar row");
    }

    #[test]
    fn rows_hit_test_hover_and_keys_wrap() {
        let mut m = menu(3);
        let b = Rect::new(0, 0, 80, 24);
        assert_eq!(m.item_at(Point::new(7, 4), b), Some(&MenuAction::Open), "first row under the top border");
        assert_eq!(m.item_at(Point::new(7, 5), b), Some(&MenuAction::OpenWith));
        assert_eq!(m.item_at(Point::new(7, 3), b), None, "border row");
        assert_eq!(m.item_at(Point::new(60, 4), b), None, "outside");
        m.hover(Point::new(7, 6), b);
        assert_eq!(m.selected(), Some(&MenuAction::Rename));
        m.down();
        assert_eq!(m.sel, 0, "down wraps");
        m.up();
        assert_eq!(m.sel, 2, "up wraps");
        m.replace(vec![MenuAction::OpenWithApp("less".into())]);
        assert_eq!((m.sel, m.items.len()), (0, 1));
    }

    #[test]
    fn render_is_opaque_and_highlights_the_selection() {
        let mut m = menu(2);
        m.sel = 1;
        let b = Rect::new(0, 0, 40, 12);
        let mut buf = CellBuffer::new(40, 12);
        m.render(&mut buf, b);
        let r = m.rect(b);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                assert_eq!(buf.get(x, y).unwrap().bg.a, 255, "opaque at {x},{y}");
            }
        }
        let t = crate::theme::current();
        assert_eq!(buf.get(r.x + 2, r.y + 2).unwrap().bg, t.accent, "highlighted row");
        let row: String = (r.x + 2..r.x + 6).map(|x| buf.get(x, r.y + 1).unwrap().ch).collect();
        assert_eq!(row, "Open");
    }
}
