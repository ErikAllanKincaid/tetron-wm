//! The settings panel: a native window with a category sidebar and a content
//! pane of editable settings, backed by [`Config`]. Changes are applied live and
//! persisted to `config.toml` by the session.

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use crate::config::{AppEntry, Config};
use crate::geometry::Point;

// Chrome colors follow the active desktop theme (were hardcoded midnight, which
// left Settings dark under every theme). Named `pal_*` to avoid colliding with
// local `bg`/`fg` bindings in the render code.
fn pal_bg() -> Rgba { crate::theme::current().window_bg }
fn pal_fg() -> Rgba { crate::theme::current().text }
fn pal_dim() -> Rgba { crate::theme::current().dim }
fn pal_sel() -> Rgba { crate::theme::current().active_bg }
fn pal_accent() -> Rgba { crate::theme::current().accent }
// Semantic success color (e.g. the "on"/checked state); not theme-swapped.
const GREEN: Rgba = Rgba { r: 126, g: 231, b: 135, a: 255 };

const SIDEBAR_W: i32 = 18;
const SECTIONS: &[&str] = &["Windows", "Appearance", "Updates", "Apps", "Default Apps", "Assistant", "About"];

/// Roles listed in the Default Apps section (config key, label).
const DEFAULT_APP_ROLES: &[(&str, &str)] = &[
    ("image", "Images"),
    ("text", "Text"),
    ("code", "Code"),
    ("audio", "Audio"),
    ("video", "Video"),
    ("pdf", "PDF"),
    ("archive", "Archives"),
    ("editor", "Editor"),
    ("terminal", "Terminal"),
];

/// An action the session must perform on behalf of the settings panel
/// (these touch the network / spawn processes, so the panel only requests them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    /// Restart the app server (closes all apps) — shown only after a compat
    /// warning flagged the running apphost as too old for this binary.
    RestartApphost,
    /// Check upstream for a newer commit.
    CheckUpdates,
    /// Install the latest version.
    InstallUpdate,
}

/// In-progress custom-app entry shown in the Apps section's add form.
#[derive(Default)]
struct AppEdit {
    name: String,
    command: String,
    /// Focused field: 0 = name, 1 = command.
    field: u8,
}

/// Settings panel state (owns a working copy of the config).
pub struct Settings {
    /// Whether the running apphost is older than this binary supports.
    apphost_outdated: bool,
    cfg: Config,
    section: usize,
    sel: usize,
    action: Option<SettingsAction>,
    update_status: String,
    /// `Some` while the Apps section's add form is open.
    edit: Option<AppEdit>,
    /// `Some` while the Appearance → Wallpaper filename box is being typed into;
    /// holds the in-progress text (committed to `cfg.wallpaper` on Enter).
    wp_edit: Option<String>,
}

impl Settings {
    /// Create a settings panel editing a copy of `cfg`.
    pub fn new(cfg: Config) -> Self {
        Self { apphost_outdated: false, cfg, section: 0, sel: 0, action: None, update_status: String::new(), edit: None, wp_edit: None }
    }

    /// Take a pending action requested by the user (cleared on read).
    pub fn take_action(&mut self) -> Option<SettingsAction> {
        self.action.take()
    }

    /// Set the text shown under the Updates section after a check.
    pub fn set_update_status(&mut self, s: String) {
        self.update_status = s;
    }

    /// Show the "Restart app server" row (set when the apphost was flagged as
    /// older than this binary's minimum-compatible protocol).
    pub fn set_apphost_outdated(&mut self, outdated: bool) {
        self.apphost_outdated = outdated;
    }

    /// Jump to the Updates section (used to reopen there after an update reload).
    pub fn show_updates_section(&mut self) {
        if let Some(i) = SECTIONS.iter().position(|s| *s == "Updates") {
            self.section = i;
            self.sel = 0;
        }
    }

    /// The live (edited) config.
    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Whether the Apps section's text-entry form is currently open (the client
    /// forwards typed characters only in this state).
    pub fn is_editing(&self) -> bool {
        self.edit.is_some() || self.wp_edit.is_some()
    }

    /// Number of interactive rows in the current section.
    fn item_count(&self) -> usize {
        match self.section {
            0 => 7,                            // snapping, threshold, grid rows/cols, gap, auto-tile, launch-maximized
            1 => 5,                            // shadows, theme, wallpaper toggle, wallpaper file, icon-scrim
            2 => if self.apphost_outdated { 4 } else { 3 }, // check, install, branch [, restart apphost]
            3 => self.cfg.launcher.len() + 1,  // custom apps + "＋ Add app…"
            4 => DEFAULT_APP_ROLES.len(),
            5 => 2,                            // assistant: open-as mode, agent
            _ => 0,                            // About
        }
    }

    pub fn move_up(&mut self) {
        if self.wp_edit.is_some() {
            return; // typing a filename — arrows do nothing
        }
        if let Some(e) = self.edit.as_mut() {
            e.field = 0; // focus the Name field
            return;
        }
        self.sel = self.sel.saturating_sub(1);
    }
    pub fn move_down(&mut self) {
        if self.wp_edit.is_some() {
            return;
        }
        if let Some(e) = self.edit.as_mut() {
            e.field = 1; // focus the Command field
            return;
        }
        if self.sel + 1 < self.item_count() {
            self.sel += 1;
        }
    }
    pub fn prev_section(&mut self) {
        if self.edit.is_some() || self.wp_edit.is_some() {
            return; // don't leave a half-typed form via arrow keys
        }
        if self.section > 0 {
            self.section -= 1;
            self.sel = 0;
        }
    }
    pub fn next_section(&mut self) {
        if self.edit.is_some() || self.wp_edit.is_some() {
            return;
        }
        if self.section + 1 < SECTIONS.len() {
            self.section += 1;
            self.sel = 0;
        }
    }

    /// Append a character to the active text field (Apps add form, or the
    /// Appearance → Wallpaper filename box).
    pub fn type_char(&mut self, c: char) {
        if let Some(e) = self.edit.as_mut() {
            match e.field {
                0 => e.name.push(c),
                _ => e.command.push(c),
            }
        } else if let Some(s) = self.wp_edit.as_mut() {
            s.push(c);
        }
    }

    /// Delete the last character of the active text field.
    pub fn backspace(&mut self) {
        if let Some(e) = self.edit.as_mut() {
            match e.field {
                0 => { e.name.pop(); }
                _ => { e.command.pop(); }
            }
        } else if let Some(s) = self.wp_edit.as_mut() {
            s.pop();
        }
    }

    /// Abandon the active text field without saving (Apps add form or the
    /// wallpaper filename box).
    pub fn cancel_edit(&mut self) {
        self.edit = None;
        self.wp_edit = None;
    }

    /// Toggle / activate the selected row.
    pub fn toggle(&mut self) {
        self.adjust(0);
    }
    /// Decrease / turn off / remove the selected row.
    pub fn left(&mut self) {
        self.adjust(-1);
    }
    /// Increase / turn on the selected row.
    pub fn right(&mut self) {
        self.adjust(1);
    }

    /// Apply a change to the selected setting. `dir`: 0 = toggle, -1/+1 = down/up.
    fn adjust(&mut self, dir: i32) {
        // While the wallpaper filename box is open, Enter (dir 0) commits it and
        // every other adjust is swallowed so typing cannot change another row.
        if self.wp_edit.is_some() {
            if dir == 0 {
                self.commit_wallpaper();
            }
            return;
        }
        match (self.section, self.sel) {
            (0, 0) => self.cfg.snapping_enabled = flip(self.cfg.snapping_enabled, dir),
            (0, 1) => {
                self.cfg.snap_threshold = match dir {
                    -1 => (self.cfg.snap_threshold - 1).max(1),
                    1 => (self.cfg.snap_threshold + 1).min(10),
                    // Enter/space on a number wraps 1..=10.
                    _ => if self.cfg.snap_threshold >= 10 { 1 } else { self.cfg.snap_threshold + 1 },
                };
            }
            (0, 2) => self.cfg.grid_rows = step_u8(self.cfg.grid_rows, dir, 1, 6),
            (0, 3) => self.cfg.grid_cols = step_u8(self.cfg.grid_cols, dir, 1, 6),
            (0, 4) => {
                self.cfg.tile_gap = match dir {
                    -1 => (self.cfg.tile_gap - 1).max(0),
                    1 => (self.cfg.tile_gap + 1).min(4),
                    _ => if self.cfg.tile_gap >= 4 { 0 } else { self.cfg.tile_gap + 1 },
                };
            }
            (0, 5) => self.cfg.auto_tile = flip(self.cfg.auto_tile, dir),
            (0, 6) => self.cfg.launch_maximized = flip(self.cfg.launch_maximized, dir),
            (1, 0) => self.cfg.window_shadows = flip(self.cfg.window_shadows, dir),
            (1, 1) => {
                let presets = crate::theme::preset_names();
                let cur_idx = presets.iter().position(|p| p == &self.cfg.theme).unwrap_or(0);
                let next_idx = if dir == -1 {
                    (cur_idx + presets.len() - 1) % presets.len()
                } else {
                    (cur_idx + 1) % presets.len()
                };
                self.cfg.theme = presets[next_idx].clone();
            }
            (1, 2) => self.cfg.wallpaper_enabled = flip(self.cfg.wallpaper_enabled, dir),
            // Wallpaper filename: Enter opens the text box (pre-filled with the
            // current value); ←/→ do nothing (it is free text, not a cycler).
            (1, 3) if dir == 0 => {
                self.wp_edit = Some(self.cfg.wallpaper.clone().unwrap_or_default());
            }
            // Icon label scrim on/off: "on" keeps the default (or a configured
            // color); "off" stores "none" so labels sit directly on the wallpaper.
            (1, 4) => {
                self.cfg.desktop_scrim = if scrim_on(&self.cfg) { Some("none".into()) } else { None };
            }
            // Updates section: Enter/Space (dir 0) requests an action from the session.
            (2, 0) if dir == 0 => self.action = Some(SettingsAction::CheckUpdates),
            (2, 1) if dir == 0 => self.action = Some(SettingsAction::InstallUpdate),
            (2, 3) if dir == 0 && self.apphost_outdated => {
                self.action = Some(SettingsAction::RestartApphost)
            }
            (2, 2) => {
                let b = crate::config::UPDATE_BRANCHES;
                let cur = b.iter().position(|x| *x == self.cfg.update_branch).unwrap_or(0);
                let next = match dir { -1 => (cur + b.len() - 1) % b.len(), _ => (cur + 1) % b.len() };
                self.cfg.update_branch = b[next].to_string();
            }
            // Apps section.
            (3, _) => self.adjust_apps(dir),
            (5, 0) => {
                self.cfg.assistant_mode =
                    if self.cfg.assistant_mode == "panel" { "window".into() } else { "panel".into() };
            }
            (5, 1) => {
                // Switch the agent between the supported CLIs (opencode ⇄ hermes),
                // stored in `assistant_command`.
                let agents = crate::assistant::AGENTS;
                let cur = self
                    .cfg
                    .assistant_command
                    .as_deref()
                    .and_then(|c| agents.iter().position(|&a| a == c))
                    .unwrap_or(0);
                let next = match dir {
                    -1 => (cur + agents.len() - 1) % agents.len(),
                    _ => (cur + 1) % agents.len(),
                };
                self.cfg.assistant_command = Some(agents[next].to_string());
            }
            (4, i) => {
                if let Some((key, _)) = DEFAULT_APP_ROLES.get(i) {
                    let role = role_from_key(key);
                    let cands = crate::openwith::candidates(role);
                    let cur = self.cfg.default_apps.get(*key).cloned().unwrap_or_default();
                    let idx = cands.iter().position(|c| c == &cur).unwrap_or(0);
                    let next = match dir {
                        -1 => (idx + cands.len() - 1) % cands.len(),
                        _ => (idx + 1) % cands.len(),
                    };
                    let val = cands[next].clone();
                    if val.is_empty() {
                        self.cfg.default_apps.remove(*key);
                    } else {
                        self.cfg.default_apps.insert((*key).to_string(), val);
                    }
                }
            }
            _ => {}
        }
    }

    /// Apps section: Enter on the form commits it; Enter on "＋ Add app…" opens
    /// the form; Left removes the selected custom app.
    fn adjust_apps(&mut self, dir: i32) {
        if self.edit.is_some() {
            if dir == 0 {
                self.commit_edit();
            }
            return;
        }
        let n = self.cfg.launcher.len();
        if dir == 0 {
            if self.sel == n {
                self.edit = Some(AppEdit::default()); // "＋ Add app…" row
            }
        } else if dir == -1 && self.sel < n {
            self.cfg.launcher.remove(self.sel);
            self.sel = self.sel.min(self.item_count().saturating_sub(1));
        }
    }

    /// Validate and save the add form into `cfg.launcher`. Empty fields just move
    /// focus to the offending field instead of committing.
    fn commit_edit(&mut self) {
        let (name, command) = match self.edit.as_ref() {
            Some(e) => (e.name.trim().to_string(), e.command.trim().to_string()),
            None => return,
        };
        if name.is_empty() {
            self.edit.as_mut().unwrap().field = 0;
            return;
        }
        if command.is_empty() {
            self.edit.as_mut().unwrap().field = 1;
            return;
        }
        // Split the command line into program + args on whitespace.
        let mut parts = command.split_whitespace().map(String::from);
        let program = parts.next().unwrap();
        let args: Vec<String> = parts.collect();
        self.cfg.launcher.push(AppEntry {
            name,
            command: program,
            args,
            category: Some("Custom".into()),
            requires_cwd: None,
            cwd: None,
            cli: None,
            warn: None,
        });
        self.edit = None;
        self.sel = self.cfg.launcher.len(); // park on the "＋ Add app…" row
    }

    /// Save the wallpaper filename box into `cfg.wallpaper` (empty = no
    /// wallpaper → `None`), then close the box.
    fn commit_wallpaper(&mut self) {
        if let Some(s) = self.wp_edit.take() {
            let s = s.trim();
            self.cfg.wallpaper = if s.is_empty() { None } else { Some(s.to_string()) };
        }
    }

    /// Handle a content-local click; returns `true` if a setting changed.
    pub fn handle_click(&mut self, p: Point, _w: i32, _h: i32) -> bool {
        if self.edit.is_some() || self.wp_edit.is_some() {
            return false; // text-entry boxes are keyboard-driven
        }
        if p.x < SIDEBAR_W {
            let i = (p.y - 1) as usize;
            if i < SECTIONS.len() {
                self.section = i;
                self.sel = 0;
            }
            return false;
        }
        let row = p.y - 3;
        if row >= 0 && (row as usize) < self.item_count() {
            self.sel = row as usize;
            self.toggle();
            return true;
        }
        false
    }

    /// Render the panel into a `w × h` content buffer.
    pub fn render(&self, w: i32, h: i32) -> CellBuffer {
        let mut buf = CellBuffer::new(w, h);
        buf.fill(Cell { ch: ' ', fg: pal_fg(), bg: pal_bg(), attrs: Default::default() });

        // Sidebar.
        for (i, s) in SECTIONS.iter().enumerate() {
            let y = 1 + i as i32;
            let sel = i == self.section;
            let (fg, bg) = if sel { (pal_accent(), pal_sel()) } else { (pal_dim(), pal_bg()) };
            for x in 0..SIDEBAR_W {
                buf.set(x, y, Cell { ch: ' ', fg, bg, attrs: Default::default() });
            }
            buf.write_str(2, y, s, fg, bg);
        }
        for y in 0..h {
            buf.set(SIDEBAR_W, y, Cell { ch: '\u{2502}', fg: pal_dim(), bg: pal_bg(), attrs: Default::default() });
        }

        let cx = SIDEBAR_W + 2;
        buf.write_str(cx, 1, SECTIONS[self.section], pal_accent(), pal_bg());

        match self.section {
            0 => {
                self.row(&mut buf, cx, 3, 0, "Drag-to-cell snapping", toggle_val(self.cfg.snapping_enabled));
                self.row(&mut buf, cx, 4, 1, "Snap threshold (cells)", format!("\u{25C2} {} \u{25B8}", self.cfg.snap_threshold));
                self.row(&mut buf, cx, 5, 2, "Grid rows", format!("\u{25C2} {} \u{25B8}", self.cfg.grid_rows));
                self.row(&mut buf, cx, 6, 3, "Grid columns", format!("\u{25C2} {} \u{25B8}", self.cfg.grid_cols));
                self.row(&mut buf, cx, 7, 4, "Tile gap (cells)", format!("\u{25C2} {} \u{25B8}", self.cfg.tile_gap));
                self.row(&mut buf, cx, 8, 5, "Auto-tile windows", toggle_val(self.cfg.auto_tile));
                self.row(&mut buf, cx, 9, 6, "Launch maximized", toggle_val(self.cfg.launch_maximized));
            }
            1 => {
                self.row(&mut buf, cx, 3, 0, "Window shadows", toggle_val(self.cfg.window_shadows));
                self.row(&mut buf, cx, 4, 1, "Theme", self.cfg.theme.clone());
                self.row(&mut buf, cx, 5, 2, "Wallpaper", toggle_val(self.cfg.wallpaper_enabled));
                // Filename text box. While editing, show the live buffer with a
                // cursor; otherwise the saved value (or a placeholder).
                let file_val = match &self.wp_edit {
                    Some(s) => format!("{s}\u{2588}"), // block cursor while typing
                    None => match self.cfg.wallpaper.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                        Some(p) => p.to_string(),
                        None => "(none — press Enter to type)".to_string(),
                    },
                };
                self.row(&mut buf, cx, 6, 3, "Wallpaper file", file_val);
                self.row(&mut buf, cx, 7, 4, "Icon label scrim", toggle_val(scrim_on(&self.cfg)));
                buf.write_str(cx, 9, "Add your image to ~/.config/tetron-wm/wallpapers/", pal_dim(), pal_bg());
                buf.write_str(cx, 10, "then type its filename above (needs chafa).", pal_dim(), pal_bg());
                buf.write_str(cx, 11, "Scrim: dark backing behind icon labels over a wallpaper.", pal_dim(), pal_bg());
            }
            2 => {
                self.row(&mut buf, cx, 3, 0, "Check for updates", String::new());
                self.row(&mut buf, cx, 4, 1, "Update & Reload", String::new());
                self.row(&mut buf, cx, 5, 2, "Channel", format!("\u{25C2} {} \u{25B8}", self.cfg.update_branch));
                let sha = &crate::GIT_SHA[..crate::GIT_SHA.len().min(7)];
                buf.write_str(cx, 7, &format!("installed: v{} ({})", crate::VERSION, sha), pal_dim(), pal_bg());
                if !self.update_status.is_empty() {
                    let col = if self.update_status.contains("available") { GREEN } else { pal_dim() };
                    buf.write_str(cx, 8, &self.update_status, col, pal_bg());
                }
                if self.cfg.update_branch != "main" {
                    buf.write_str(cx, 9, "dev channel: builds from source (slower).", pal_dim(), pal_bg());
                }
                if self.apphost_outdated {
                    self.row(&mut buf, cx, 6, 3, "Restart app server", "(closes apps)".into());
                    buf.write_str(cx, 10, "The app server predates this update; some features", pal_dim(), pal_bg());
                    buf.write_str(cx, 11, "won't work until it restarts (your apps will close).", pal_dim(), pal_bg());
                }
            }
            3 => self.render_apps(&mut buf, cx, w),
            5 => {
                // Agent switches between opencode and hermes; a hand-edited
                // assistant_command may name any binary, shown as-is.
                let agent = self.cfg.assistant_command.as_deref().unwrap_or(crate::assistant::DEFAULT_AGENT);
                let mark = if crate::assistant::agent_available(agent) { "" } else { " (not installed)" };
                self.row(&mut buf, cx, 3, 0, "Open as", format!("\u{25C2} {} \u{25B8}", self.cfg.assistant_mode));
                self.row(&mut buf, cx, 4, 1, "Agent", format!("\u{25C2} {agent} \u{25B8}{mark}"));
                buf.write_str(cx, 6, "The \u{2726} menubar button opens the assistant.", pal_dim(), pal_bg());
                buf.write_str(cx, 7, "Its briefing pack: ~/.local/share/tetron-wm/assistant", pal_dim(), pal_bg());
                buf.write_str(cx, 8, "Extra args: assistant_args in config.toml.", pal_dim(), pal_bg());
            }
            4 => {
                for (i, (key, label)) in DEFAULT_APP_ROLES.iter().enumerate() {
                    let val = self.cfg.default_apps.get(*key).cloned().unwrap_or_else(|| "(ask)".into());
                    let shown = match val.as_str() { "@image" => "image viewer".into(), "@navigate" => "open folder".into(), v => v.to_string() };
                    self.row(&mut buf, cx, 3 + i as i32, i, label, format!("\u{25C2} {} \u{25B8}", shown));
                }
            }
            _ => {
                buf.write_str(cx, 3, "tetron-wm — a desktop environment for the terminal", pal_fg(), pal_bg());
                buf.write_str(cx, 5, "Settings are saved to ~/.config/tetron-wm/config.toml", pal_dim(), pal_bg());
                buf.write_str(cx, 6, "github.com/ErikAllanKincaid/tetron-wm", pal_dim(), pal_bg());
            }
        }
        buf
    }

    /// Render the Apps section: either the add form or the custom-app list.
    fn render_apps(&self, buf: &mut CellBuffer, cx: i32, w: i32) {
        if let Some(e) = self.edit.as_ref() {
            buf.write_str(cx, 3, "Add a custom app", pal_fg(), pal_bg());
            self.field(buf, cx, 5, "Name", &e.name, e.field == 0);
            self.field(buf, cx, 6, "Command", &e.command, e.field == 1);
            buf.write_str(cx, 8, "Enter save \u{00B7} \u{2191}\u{2193} switch field \u{00B7} Esc cancel", pal_dim(), pal_bg());
            return;
        }

        if self.cfg.launcher.is_empty() {
            buf.write_str(cx, 3, "No custom apps yet.", pal_dim(), pal_bg());
        }
        for (i, a) in self.cfg.launcher.iter().enumerate() {
            let y = 3 + i as i32;
            let sel = i == self.sel;
            let marker = if sel { "\u{25B8} " } else { "  " };
            buf.write_str(cx, y, marker, pal_accent(), pal_bg());
            buf.write_str(cx + 2, y, &a.name, if sel { pal_fg() } else { pal_dim() }, pal_bg());
            let cmd = command_line(a);
            let cmd_x = cx + 16;
            let max = (w - cmd_x - 12).max(4) as usize;
            buf.write_str(cmd_x, y, &truncate(&cmd, max), pal_dim(), pal_bg());
            if sel {
                buf.write_str(w - 12, y, "\u{2190} remove", pal_dim(), pal_bg());
            }
        }
        // "＋ Add app…" row.
        let add_y = 3 + self.cfg.launcher.len() as i32;
        let add_sel = self.sel == self.cfg.launcher.len();
        let marker = if add_sel { "\u{25B8} " } else { "  " };
        buf.write_str(cx, add_y, marker, pal_accent(), pal_bg());
        buf.write_str(cx + 2, add_y, "\u{FF0B} Add app\u{2026}", if add_sel { GREEN } else { pal_dim() }, pal_bg());
    }

    /// Draw one labelled form field with a block cursor on the focused field.
    fn field(&self, buf: &mut CellBuffer, x: i32, y: i32, label: &str, value: &str, focused: bool) {
        let lcol = if focused { pal_accent() } else { pal_dim() };
        buf.write_str(x, y, &format!("{label}:"), lcol, pal_bg());
        let vx = x + 10;
        let shown = if focused { format!("{value}\u{2588}") } else { value.to_string() };
        buf.write_str(vx, y, &shown, pal_fg(), pal_bg());
    }

    fn row(&self, buf: &mut CellBuffer, x: i32, y: i32, idx: usize, label: &str, value: String) {
        let sel = idx == self.sel && self.item_count() > 0;
        let marker = if sel { "\u{25B8} " } else { "  " };
        buf.write_str(x, y, marker, pal_accent(), pal_bg());
        buf.write_str(x + 2, y, label, if sel { pal_fg() } else { pal_dim() }, pal_bg());
        let vx = x + 30;
        let vcol = if value.contains("on") { GREEN } else { pal_fg() };
        buf.write_str(vx, y, &value, vcol, pal_bg());
    }
}

fn role_from_key(key: &str) -> crate::openwith::Role {
    use crate::openwith::Role::*;
    match key {
        "image" => Image, "video" => Video, "audio" => Audio, "text" => Text,
        "code" => Code, "archive" => Archive, "pdf" => Pdf, _ => Other,
    }
}

fn flip(current: bool, dir: i32) -> bool {
    match dir {
        0 => !current,
        d => d > 0,
    }
}

/// Step a `u8` setting within `[lo, hi]`. `dir` -1/+1 decrements/increments;
/// 0 (Enter/Space) wraps up to `lo` after `hi`.
fn step_u8(v: u8, dir: i32, lo: u8, hi: u8) -> u8 {
    match dir {
        -1 => v.saturating_sub(1).max(lo),
        1 => (v + 1).min(hi),
        _ => if v >= hi { lo } else { v + 1 },
    }
}

fn toggle_val(on: bool) -> String {
    if on { "\u{25CF} on".into() } else { "\u{25CB} off".into() }
}

/// Whether the desktop icon-label scrim is on (anything but `"none"`/`"off"`).
fn scrim_on(cfg: &Config) -> bool {
    !matches!(
        cfg.desktop_scrim.as_deref().map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("none") | Some("off")
    )
}

/// The full command line (program + args) of a launcher entry, for display.
fn command_line(a: &AppEntry) -> String {
    if a.args.is_empty() {
        a.command.clone()
    } else {
        format!("{} {}", a.command, a.args.join(" "))
    }
}

/// Truncate `s` to at most `max` characters, appending `…` when cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let keep = max.saturating_sub(1);
        format!("{}\u{2026}", s.chars().take(keep).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advance the section selector to the Apps panel ("Windows" → … → "Apps").
    fn to_apps(s: &mut Settings) {
        while SECTIONS[s.section] != "Apps" {
            s.next_section();
        }
    }

    #[test]
    fn add_custom_app_splits_command_and_args() {
        let mut s = Settings::new(Config::default());
        to_apps(&mut s);
        assert!(s.config().launcher.is_empty());

        s.toggle(); // open the add form on the "＋ Add app…" row
        assert!(s.is_editing());
        for c in "SSH box".chars() { s.type_char(c); }
        s.move_down(); // focus the Command field
        for c in "ssh user@host".chars() { s.type_char(c); }
        s.toggle(); // commit

        assert!(!s.is_editing());
        let apps = &s.config().launcher;
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "SSH box");
        assert_eq!(apps[0].command, "ssh");
        assert_eq!(apps[0].args, vec!["user@host".to_string()]);
        assert_eq!(apps[0].category.as_deref(), Some("Custom"));
    }

    #[test]
    fn empty_field_blocks_commit_and_refocuses() {
        let mut s = Settings::new(Config::default());
        to_apps(&mut s);
        s.toggle(); // open form
        // Commit with both fields empty: stays editing, focused on Name.
        s.toggle();
        assert!(s.is_editing());
        assert!(s.config().launcher.is_empty());
    }

    #[test]
    fn cancel_edit_discards_the_form() {
        let mut s = Settings::new(Config::default());
        to_apps(&mut s);
        s.toggle();
        for c in "junk".chars() { s.type_char(c); }
        s.cancel_edit();
        assert!(!s.is_editing());
        assert!(s.config().launcher.is_empty());
    }

    /// Move selection to the Appearance → "Wallpaper file" row.
    fn to_wallpaper_file(s: &mut Settings) {
        while SECTIONS[s.section] != "Appearance" {
            s.next_section();
        }
        s.sel = 0;
        while s.sel < 3 {
            s.move_down();
        }
    }

    #[test]
    fn wallpaper_file_box_commits_filename() {
        let cfg = Config { wallpaper: None, ..Config::default() };
        let mut s = Settings::new(cfg);
        to_wallpaper_file(&mut s);
        s.toggle(); // open the filename box (empty, since wallpaper is None)
        assert!(s.is_editing());
        for c in "gentle-dawn.png".chars() { s.type_char(c); }
        s.toggle(); // commit
        assert!(!s.is_editing());
        assert_eq!(s.config().wallpaper.as_deref(), Some("gentle-dawn.png"));
    }

    #[test]
    fn wallpaper_default_filename_is_wallpaper_jpg() {
        assert_eq!(Config::default().wallpaper.as_deref(), Some("wallpaper.jpg"));
    }

    #[test]
    fn icon_scrim_toggle_flips() {
        let mut s = Settings::new(Config::default());
        while SECTIONS[s.section] != "Appearance" {
            s.next_section();
        }
        s.sel = 4; // "Icon label scrim" row
        assert!(s.config().desktop_scrim.is_none()); // default = on
        s.toggle();
        assert_eq!(s.config().desktop_scrim.as_deref(), Some("none")); // -> off
        s.toggle();
        assert!(s.config().desktop_scrim.is_none()); // -> back on
    }

    #[test]
    fn wallpaper_file_empty_clears_to_none() {
        let cfg = Config { wallpaper: Some("old.png".into()), ..Config::default() };
        let mut s = Settings::new(cfg);
        to_wallpaper_file(&mut s);
        s.toggle(); // opens pre-filled with "old.png"
        for _ in 0.."old.png".len() { s.backspace(); }
        s.toggle(); // commit empty → None
        assert_eq!(s.config().wallpaper, None);
    }

    #[test]
    fn wallpaper_file_cancel_keeps_prior_value() {
        let cfg = Config { wallpaper: Some("keep.png".into()), ..Config::default() };
        let mut s = Settings::new(cfg);
        to_wallpaper_file(&mut s);
        s.toggle();
        for c in "junk".chars() { s.type_char(c); }
        s.cancel_edit();
        assert!(!s.is_editing());
        assert_eq!(s.config().wallpaper.as_deref(), Some("keep.png"));
    }

    #[test]
    fn left_removes_selected_custom_app() {
        let cfg = Config {
            launcher: vec![
                AppEntry { name: "A".into(), command: "a".into(), args: vec![], category: Some("Custom".into()), requires_cwd: None, cwd: None, cli: None, warn: None },
                AppEntry { name: "B".into(), command: "b".into(), args: vec![], category: Some("Custom".into()), requires_cwd: None, cwd: None, cli: None, warn: None },
            ],
            ..Config::default()
        };
        let mut s = Settings::new(cfg);
        to_apps(&mut s);
        s.sel = 0;
        s.left(); // remove "A"
        assert_eq!(s.config().launcher.len(), 1);
        assert_eq!(s.config().launcher[0].name, "B");
    }

    #[test]
    fn default_apps_section_cycles_handler() {
        let mut s = Settings::new(Config::default());
        while SECTIONS[s.section] != "Default Apps" {
            s.next_section();
        }
        // Row 0 is the first role; cycling changes its handler in the config.
        s.sel = 0;
        let before = s.config().default_apps.clone();
        s.right();
        assert_ne!(s.config().default_apps, before, "cycling changed a handler");
    }

    #[test]
    fn windows_section_steps_grid_dimensions() {
        let mut s = Settings::new(Config::default());
        // Windows is section 0; row 2 = grid rows, row 3 = grid columns.
        s.section = 0;
        s.sel = 2;
        s.right();
        assert_eq!(s.config().grid_rows, 3);
        s.left();
        assert_eq!(s.config().grid_rows, 2);
        s.sel = 3;
        s.right();
        assert_eq!(s.config().grid_cols, 3);
        s.sel = 5; // auto-tile toggle
        s.toggle();
        assert!(s.config().auto_tile);
    }

    #[test]
    fn assistant_section_switches_agent_between_opencode_and_hermes() {
        let mut s = Settings::new(Config::default());
        s.section = 5; // Assistant
        assert_eq!(SECTIONS[s.section], "Assistant");
        s.sel = 1; // the Agent row
        assert!(s.config().assistant_command.is_none(), "unset defaults to opencode");
        s.toggle(); // opencode -> hermes
        assert_eq!(s.config().assistant_command.as_deref(), Some("hermes"));
        s.toggle(); // hermes -> opencode
        assert_eq!(s.config().assistant_command.as_deref(), Some("opencode"));
        s.left(); // wraps backward to hermes
        assert_eq!(s.config().assistant_command.as_deref(), Some("hermes"));
    }
}
