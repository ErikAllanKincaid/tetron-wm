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
    Entry { is_dir: bool, pinned: bool, role: Role },
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
    /// Hand the screen to fbi / fim (plain console only), with the folder's images.
    ViewFullscreen,
    SetWallpaper,
    CopyPath,
    /// A shell in the folder (the entry's folder for a file, the folder itself).
    OpenTerminalHere,
    /// Hand the screen to mpv / cvlc for a video (plain console only).
    PlayFullscreen,
    ExtractHere,
    ListContents,
    /// Cast a video, audio or image file to a Chromecast with catt, in a window.
    CastTo,
    /// Replace the menu with the saved Systems to copy the entry to.
    SendTo,
    /// Copy the entry to this Systems entry (by name) with scp.
    SendToSystem(String),
    // Window (title bar) actions.
    Minimize,
    Maximize,
    MoveWindow,
    RenameWindow,
    CloseWindow,
}

impl MenuAction {
    pub fn label(&self) -> String {
        match self {
            MenuAction::Open => "Open".into(),
            MenuAction::OpenWith => "Open with\u{2026}".into(),
            MenuAction::OpenWithApp(cmd) if cmd == "@image" => "Image viewer".into(),
            MenuAction::OpenWithApp(cmd) if cmd == "@pdf" => "PDF viewer".into(),
            MenuAction::OpenWithApp(cmd) if cmd == "@pdftext" => "Text (pdftotext)".into(),
            MenuAction::OpenWithApp(cmd) if cmd == "@timg" => "timg".into(),
            MenuAction::OpenWithApp(cmd) => cmd.clone(),
            MenuAction::Rename => "Rename".into(),
            MenuAction::Copy => "Copy".into(),
            MenuAction::Cut => "Cut".into(),
            MenuAction::Trash => "Move to Trash".into(),
            MenuAction::GetInfo => "Get Info".into(),
            MenuAction::Unpin => "Unpin".into(),
            MenuAction::NewFolder => "New Folder".into(),
            MenuAction::CleanUp => "Clean Up".into(),
            MenuAction::ViewFullscreen => "View full screen".into(),
            MenuAction::SetWallpaper => "Set as wallpaper".into(),
            MenuAction::CopyPath => "Copy path".into(),
            MenuAction::OpenTerminalHere => "Open terminal here".into(),
            MenuAction::PlayFullscreen => "Play full screen".into(),
            MenuAction::ExtractHere => "Extract here".into(),
            MenuAction::ListContents => "List contents".into(),
            MenuAction::CastTo => "Cast to Chromecast".into(),
            MenuAction::SendTo => "Send to\u{2026}".into(),
            MenuAction::SendToSystem(name) => name.clone(),
            MenuAction::Minimize => "Minimize".into(),
            MenuAction::Maximize => "Maximize".into(),
            MenuAction::MoveWindow => "Move".into(),
            MenuAction::RenameWindow => "Rename".into(),
            MenuAction::CloseWindow => "Close".into(),
        }
    }
}

/// What the client and machine can do, for the actions that depend on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Abilities {
    /// A route to show images full screen exists (see `fullscreen::plan`): "View full screen".
    pub fullscreen_image: bool,
    /// A route to play video full screen exists: "Play full screen".
    pub fullscreen_video: bool,
    /// A route to show a PDF full screen exists: "View full screen".
    pub fullscreen_pdf: bool,
    /// Saved Systems exist: "Send to…".
    pub send: bool,
    /// catt is installed: "Cast to Chromecast" for video, audio and images.
    pub cast: bool,
}

impl Abilities {
    /// For the attached client and this machine.
    pub fn current() -> Self {
        use crate::fullscreen::{offered, Kind};
        let caps = crate::display::get();
        let have = crate::catalog::is_installed;
        Abilities {
            fullscreen_image: offered(Kind::Images, &caps, &have),
            fullscreen_video: offered(Kind::Video, &caps, &have),
            fullscreen_pdf: offered(Kind::Pdf, &caps, &have),
            send: !crate::systems::load().is_empty() && have("scp"),
            cast: have("catt"),
        }
    }
}

/// The actions `target` offers on `surface`, top to bottom.
pub fn actions_for(surface: Surface, target: Target, can: Abilities) -> Vec<MenuAction> {
    use MenuAction as A;
    match (surface, target) {
        (Surface::Desktop, Target::Entry { pinned: true, .. }) => vec![A::Open, A::Unpin],
        (_, Target::Entry { is_dir, role, .. }) => {
            let mut v = vec![A::Open];
            if !is_dir {
                v.push(A::OpenWith);
            }
            match role {
                Role::Image => {
                    if can.fullscreen_image {
                        v.push(A::ViewFullscreen);
                    }
                    v.push(A::SetWallpaper);
                }
                Role::Video if can.fullscreen_video => v.push(A::PlayFullscreen),
                Role::Pdf if can.fullscreen_pdf => v.push(A::ViewFullscreen),
                Role::Archive => v.extend([A::ExtractHere, A::ListContents]),
                _ => {}
            }
            if can.cast && !is_dir && matches!(role, Role::Video | Role::Audio | Role::Image) {
                v.push(A::CastTo);
            }
            v.push(A::Rename);
            if surface == Surface::Files {
                v.extend([A::Copy, A::Cut]);
            }
            v.extend([A::Trash, A::CopyPath, A::OpenTerminalHere, A::GetInfo]);
            if can.send {
                v.push(A::SendTo);
            }
            v
        }
        (Surface::Desktop, Target::Background) => vec![A::NewFolder, A::OpenTerminalHere, A::CleanUp],
        (Surface::Files, Target::Background) => Vec::new(),
    }
}

/// The title bar menu of a window.
pub fn window_actions() -> Vec<MenuAction> {
    use MenuAction as A;
    vec![A::Minimize, A::Maximize, A::MoveWindow, A::RenameWindow, A::CloseWindow]
}

/// The "Send to…" choices: one per saved Systems entry.
pub fn send_to_actions() -> Vec<MenuAction> {
    crate::systems::load().into_iter().map(|s| MenuAction::SendToSystem(s.name)).collect()
}

/// The shell command that casts `file` to a Chromecast with catt. `device` is
/// catt's `-d` value (a name or an IP); empty means catt's default device.
/// catt serves the file itself and keeps running until playback ends, so this
/// belongs in a window, where closing it stops the cast.
pub fn cast_command(file: &Path, device: &str) -> String {
    let q = |s: &str| crate::systems::sh_quote(s);
    let device = device.trim();
    let dev = if device.is_empty() { String::new() } else { format!("-d {} ", q(device)) };
    format!("catt {dev}cast {}", q(&file.to_string_lossy()))
}

/// The shell command that extracts (`list` false) or lists `archive` into
/// or from `dir`, chosen by its name; `None` for an unknown kind.
pub fn archive_command(archive: &Path, dir: &Path, list: bool) -> Option<String> {
    let q = |p: &Path| crate::systems::sh_quote(&p.to_string_lossy());
    let (a, d) = (q(archive), q(dir));
    let name = archive.file_name()?.to_string_lossy().to_ascii_lowercase();
    let tar = [".tar", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar.zst"];
    Some(if tar.iter().any(|e| name.ends_with(e)) {
        if list { format!("tar -tvf {a}") } else { format!("tar -xf {a} -C {d}") }
    } else if name.ends_with(".zip") {
        if list { format!("unzip -l {a}") } else { format!("unzip {a} -d {d}") }
    } else if name.ends_with(".7z") {
        if list { format!("7z l {a}") } else { format!("7z x {a} -o{d}") }
    } else if name.ends_with(".rar") {
        if list { format!("unrar l {a}") } else { format!("unrar x {a} {d}/") }
    } else {
        // A single compressed file: keep the original, write it next to it.
        let tool = [(".gz", "gzip"), (".xz", "xz"), (".bz2", "bzip2"), (".zst", "zstd")]
            .iter()
            .find(|(e, _)| name.ends_with(e))?
            .1;
        if list { format!("{tool} -l {a}") } else { format!("{tool} -dk {a}") }
    })
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
        ContextMenu::new(Point::new(5, 3), actions_for(Surface::Files, Target::Entry { is_dir: false, pinned: false, role: Role::Text }, Abilities::default()).into_iter().take(n).collect())
    }

    #[test]
    fn cast_is_offered_for_media_files_only_when_catt_is_there() {
        use MenuAction as A;
        let none = Abilities::default();
        let can = Abilities { cast: true, ..none };
        let t = |role, is_dir| Target::Entry { is_dir, pinned: false, role };
        for role in [Role::Video, Role::Audio, Role::Image] {
            for surface in [Surface::Desktop, Surface::Files] {
                assert!(actions_for(surface, t(role, false), can).contains(&A::CastTo), "{role:?} on {surface:?}");
                assert!(!actions_for(surface, t(role, false), none).contains(&A::CastTo), "no catt, no item");
            }
        }
        for role in [Role::Text, Role::Code, Role::Archive, Role::Pdf, Role::Executable, Role::Other] {
            assert!(!actions_for(Surface::Files, t(role, false), can).contains(&A::CastTo), "{role:?} is not media");
        }
        assert!(!actions_for(Surface::Files, t(Role::Video, true), can).contains(&A::CastTo), "a folder is not a file");
        assert!(!actions_for(Surface::Desktop, Target::Entry { is_dir: false, pinned: true, role: Role::Video }, can).contains(&A::CastTo), "pinned shortcuts only open or unpin");
        assert_eq!(A::CastTo.label(), "Cast to Chromecast");
    }

    #[test]
    fn cast_command_quotes_the_file_and_the_device() {
        let f = Path::new("/home/u/My Videos/it's here.mp4");
        assert_eq!(cast_command(f, ""), "catt cast '/home/u/My Videos/it'\\''s here.mp4'");
        assert_eq!(cast_command(f, "  "), cast_command(f, ""), "blank device means catt default");
        let c = cast_command(Path::new("/m/a.mp4"), "Living Room; rm -rf ~");
        assert_eq!(c, "catt -d 'Living Room; rm -rf ~' cast '/m/a.mp4'");
    }

    #[test]
    fn actions_differ_by_surface_and_target() {
        use MenuAction as A;
        let none = Abilities::default();
        let all = Abilities { fullscreen_image: true, fullscreen_video: true, fullscreen_pdf: true, send: true, cast: false };
        let file = Target::Entry { is_dir: false, pinned: false, role: Role::Text };
        let dir = Target::Entry { is_dir: true, pinned: false, role: Role::Directory };
        let img = Target::Entry { is_dir: false, pinned: false, role: Role::Image };
        let vid = Target::Entry { is_dir: false, pinned: false, role: Role::Video };
        let zip = Target::Entry { is_dir: false, pinned: false, role: Role::Archive };
        assert_eq!(actions_for(Surface::Desktop, file, none), vec![A::Open, A::OpenWith, A::Rename, A::Trash, A::CopyPath, A::OpenTerminalHere, A::GetInfo]);
        assert_eq!(actions_for(Surface::Desktop, dir, none), vec![A::Open, A::Rename, A::Trash, A::CopyPath, A::OpenTerminalHere, A::GetInfo], "no Open with for folders");
        assert_eq!(actions_for(Surface::Desktop, Target::Entry { is_dir: false, pinned: true, role: Role::Other }, all), vec![A::Open, A::Unpin]);
        assert_eq!(actions_for(Surface::Desktop, Target::Background, all), vec![A::NewFolder, A::OpenTerminalHere, A::CleanUp]);
        assert_eq!(
            actions_for(Surface::Files, file, none),
            vec![A::Open, A::OpenWith, A::Rename, A::Copy, A::Cut, A::Trash, A::CopyPath, A::OpenTerminalHere, A::GetInfo]
        );
        assert!(actions_for(Surface::Files, Target::Background, all).is_empty());
        // Images: wallpaper always, full screen only where the client can hand over.
        let plain = actions_for(Surface::Desktop, img, none);
        assert!(plain.contains(&A::SetWallpaper) && !plain.contains(&A::ViewFullscreen));
        assert_eq!(&actions_for(Surface::Files, img, all)[..4], &[A::Open, A::OpenWith, A::ViewFullscreen, A::SetWallpaper]);
        assert!(!actions_for(Surface::Desktop, file, all).contains(&A::ViewFullscreen), "only images");
        // Video full screen and archives; Send to only with saved Systems.
        assert!(actions_for(Surface::Desktop, vid, all).contains(&A::PlayFullscreen));
        assert!(!actions_for(Surface::Desktop, vid, none).contains(&A::PlayFullscreen));
        assert!(actions_for(Surface::Files, zip, none).contains(&A::ExtractHere));
        assert_eq!(actions_for(Surface::Files, file, all).last(), Some(&A::SendTo));
        assert!(!actions_for(Surface::Files, file, none).contains(&A::SendTo));
    }

    #[test]
    fn archive_commands_by_kind() {
        let d = Path::new("/x");
        assert_eq!(archive_command(Path::new("/x/a.tar.gz"), d, false).as_deref(), Some("tar -xf '/x/a.tar.gz' -C '/x'"));
        assert_eq!(archive_command(Path::new("/x/a.ZIP"), d, true).as_deref(), Some("unzip -l '/x/a.ZIP'"));
        assert_eq!(archive_command(Path::new("/x/a.7z"), d, false).as_deref(), Some("7z x '/x/a.7z' -o'/x'"));
        assert_eq!(archive_command(Path::new("/x/log.gz"), d, false).as_deref(), Some("gzip -dk '/x/log.gz'"));
        assert_eq!(archive_command(Path::new("/x/a.txt"), d, false), None);
    }

    #[test]
    fn open_with_lists_the_viewer_only_for_images() {
        let img = open_with_actions(Path::new("/x/cat.png"));
        assert_eq!(img.first(), Some(&MenuAction::OpenWithApp("@image".into())));
        assert_eq!(img[0].label(), "Image viewer");
        let txt = open_with_actions(Path::new("/x/notes.txt"));
        assert!(!txt.contains(&MenuAction::OpenWithApp("@image".into())));
        assert!(!txt.iter().any(|a| matches!(a, MenuAction::OpenWithApp(c) if c.is_empty() || c == "@navigate")));
        // Video never offers editors; when mpv is installed it plays in the window.
        let vid = open_with_actions(Path::new("/x/clip.mp4"));
        assert!(!vid.iter().any(|a| matches!(a, MenuAction::OpenWithApp(c) if ["vi", "nano", "less"].contains(&c.as_str()))));
        if crate::catalog::is_installed("mpv") {
            assert!(vid.contains(&MenuAction::OpenWithApp("mpv --vo=tct".into())));
        }
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
