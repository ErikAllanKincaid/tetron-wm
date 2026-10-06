//! The desktop color theme — a small palette read by the compositor, window
//! manager, and chrome each frame. A process-global current theme lets Settings
//! restyle the whole desktop live without threading a `Theme` through every call.

use crate::cell::Rgba;
use std::sync::{OnceLock, RwLock};

/// A complete desktop palette. `Copy` so render code can cheaply snapshot it.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub desktop_bg: Rgba,
    pub window_bg: Rgba,
    pub title_focus: Rgba,
    pub title_blur: Rgba,
    pub title_fg: Rgba,
    pub border: Rgba,
    pub shadow: Rgba,
    pub ctrl_fg: Rgba,
    pub close_fg: Rgba,
    pub menubar_bg: Rgba,
    pub dock_bg: Rgba,
    pub text: Rgba,
    pub dim: Rgba,
    pub accent: Rgba,
    pub active_bg: Rgba,
}

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

impl Theme {
    /// The default dark theme (tetron-wm's original palette).
    pub const fn midnight() -> Self {
        Theme {
            desktop_bg: rgb(44, 46, 50),
            window_bg: rgb(17, 20, 29),
            title_focus: rgb(29, 36, 51),
            title_blur: rgb(20, 24, 34),
            title_fg: rgb(143, 183, 255),
            border: rgb(58, 68, 88),
            shadow: Rgba { r: 0, g: 0, b: 0, a: 110 },
            ctrl_fg: rgb(150, 165, 190),
            close_fg: rgb(255, 107, 107),
            menubar_bg: rgb(22, 27, 39),
            dock_bg: rgb(22, 27, 39),
            text: rgb(200, 208, 220),
            dim: rgb(120, 130, 150),
            accent: rgb(108, 182, 255),
            active_bg: rgb(45, 58, 85),
        }
    }

    /// Nord — cool, muted blues.
    pub const fn nord() -> Self {
        Theme {
            desktop_bg: rgb(46, 52, 64),
            window_bg: rgb(36, 41, 51),
            title_focus: rgb(59, 66, 82),
            title_blur: rgb(43, 49, 60),
            title_fg: rgb(136, 192, 208),
            border: rgb(76, 86, 106),
            shadow: Rgba { r: 0, g: 0, b: 0, a: 110 },
            ctrl_fg: rgb(180, 190, 205),
            close_fg: rgb(191, 97, 106),
            menubar_bg: rgb(59, 66, 82),
            dock_bg: rgb(59, 66, 82),
            text: rgb(216, 222, 233),
            dim: rgb(120, 130, 150),
            accent: rgb(136, 192, 208),
            active_bg: rgb(76, 86, 106),
        }
    }

    /// Gruvbox — warm, retro.
    pub const fn gruvbox() -> Self {
        Theme {
            desktop_bg: rgb(60, 56, 54),
            window_bg: rgb(40, 40, 40),
            title_focus: rgb(60, 56, 54),
            title_blur: rgb(50, 48, 47),
            title_fg: rgb(250, 189, 47),
            border: rgb(102, 92, 84),
            shadow: Rgba { r: 0, g: 0, b: 0, a: 120 },
            ctrl_fg: rgb(213, 196, 161),
            close_fg: rgb(251, 73, 52),
            menubar_bg: rgb(50, 48, 47),
            dock_bg: rgb(50, 48, 47),
            text: rgb(235, 219, 178),
            dim: rgb(146, 131, 116),
            accent: rgb(250, 189, 47),
            active_bg: rgb(80, 73, 69),
        }
    }

    /// Dracula — purple/pink on dark.
    pub const fn dracula() -> Self {
        Theme {
            desktop_bg: rgb(54, 57, 76),
            window_bg: rgb(40, 42, 54),
            title_focus: rgb(68, 71, 90),
            title_blur: rgb(50, 52, 66),
            title_fg: rgb(189, 147, 249),
            border: rgb(98, 114, 164),
            shadow: Rgba { r: 0, g: 0, b: 0, a: 120 },
            ctrl_fg: rgb(200, 200, 220),
            close_fg: rgb(255, 85, 85),
            menubar_bg: rgb(68, 71, 90),
            dock_bg: rgb(68, 71, 90),
            text: rgb(248, 248, 242),
            dim: rgb(139, 145, 175),
            accent: rgb(189, 147, 249),
            active_bg: rgb(98, 114, 164),
        }
    }

    /// Light — paper ground, black-on-white terminals, tetron-green highlights.
    /// Pastoral — the only light-*appearance* preset (named for its palette, not
    /// its brightness, so the name does not collide with the `appearance` field).
    /// `accent` is a darkened green (`#2E7D32`) so button labels stay legible on
    /// the light panels, while the logo green `#4CAF50` carries the highlight as
    /// the window `border`.
    pub const fn pastoral() -> Self {
        Theme {
            desktop_bg: rgb(236, 230, 216),
            window_bg: rgb(255, 255, 255),
            // Active titlebar reads clearly green; inactive is a distinct taupe
            // bar, darker than the paper wallpaper so it does not blend in.
            title_focus: rgb(196, 227, 180),
            title_blur: rgb(212, 207, 193),
            title_fg: rgb(43, 43, 43),
            border: rgb(76, 175, 80),
            shadow: Rgba { r: 50, g: 45, b: 30, a: 51 },
            ctrl_fg: rgb(106, 106, 106),
            close_fg: rgb(209, 82, 76),
            menubar_bg: rgb(231, 226, 214),
            dock_bg: rgb(231, 226, 214),
            text: rgb(26, 26, 26),
            dim: rgb(110, 106, 94),
            accent: rgb(46, 125, 50),
            active_bg: rgb(215, 233, 205),
        }
    }

    /// Resolve a theme by name. A user theme file of the same name (loaded into the
    /// registry) wins over a built-in; otherwise a built-in preset; else `midnight`.
    pub fn named(name: &str) -> Self {
        let lname = name.to_lowercase();
        if let Some((_, t)) = registry().iter().find(|(n, _)| n.to_lowercase() == lname) {
            return *t;
        }
        Self::builtin(&lname)
    }

    /// Resolve a *built-in* preset by (lowercased) name, ignoring the registry.
    /// This is the base a partial theme file inherits from.
    fn builtin(lname: &str) -> Self {
        match lname {
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            "dracula" => Self::dracula(),
            "pastoral" => Self::pastoral(),
            // Deprecated alias: the preset was renamed light → pastoral. Kept so
            // existing `theme = "light"` configs still resolve (not in PRESETS).
            "light" => Self::pastoral(),
            _ => Self::midnight(),
        }
    }
}

/// The names of the built-in presets (for the Settings cycler). File themes are
/// appended at runtime by [`preset_names`].
pub const PRESETS: &[&str] = &["midnight", "nord", "gruvbox", "dracula", "pastoral"];

/// All theme names offered in the UI: the built-in presets, then any file themes
/// whose name does not shadow a built-in (a same-named file overrides the built-in
/// palette but keeps its slot, so it is not listed twice).
pub fn preset_names() -> Vec<String> {
    let mut names: Vec<String> = PRESETS.iter().map(|s| s.to_string()).collect();
    for (n, _) in registry() {
        if !names.iter().any(|x| x.eq_ignore_ascii_case(n)) {
            names.push(n.clone());
        }
    }
    names
}

fn slot() -> &'static RwLock<Theme> {
    static THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
    THEME.get_or_init(|| RwLock::new(Theme::midnight()))
}

/// Snapshot the current desktop theme.
pub fn current() -> Theme {
    *slot().read().unwrap()
}

/// Replace the current desktop theme (applied on the next rendered frame).
pub fn set(name: &str) {
    *slot().write().unwrap() = Theme::named(name);
}

/// Override just the terminal-relevant colors (`window_bg`/`text`) on the current
/// theme. The apphost uses this so the `terminal_bg`/`terminal_fg` config keys can
/// decouple terminal colors from the desktop theme: in the apphost process the
/// theme global is read only by `ptyhost` for hosted-app cell colors, so this
/// does not affect any chrome. `None` leaves that color following the theme.
pub fn set_terminal_colors(bg: Option<Rgba>, fg: Option<Rgba>) {
    let mut t = slot().write().unwrap();
    if let Some(bg) = bg {
        t.window_bg = bg;
    }
    if let Some(fg) = fg {
        t.text = fg;
    }
}

// ── File-based themes ────────────────────────────────────────────────────────
//
// A theme file is `<config>/themes/<name>.toml` (dir overridable via the
// `theme_dir` config key or `$TETRON_WM_THEMES_DIR`). It is a flat table of the
// palette fields → `#rrggbb`, `#rrggbbaa`, or a named color; any field left out
// inherits from `base` (a built-in preset name, default `midnight`). The file
// carries only colors — layout and the contrast rules stay in code so a theme
// cannot break them; [`validate`] warns (never aborts) on unreadable contrast.

use std::path::{Path, PathBuf};

/// On-disk form of a theme: string-typed so colors can be hex or named, every
/// field optional so a partial file still loads. Unknown keys are ignored, so a
/// file written for a newer tetron-wm still loads (version-skew tolerant).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct ThemeFile {
    /// Theme name; defaults to the file stem when absent.
    name: Option<String>,
    /// Built-in preset to inherit unset fields from (default `midnight`).
    base: Option<String>,
    // Metadata the renderer ignores — kept for tooling / forward-compat.
    schema: Option<u32>,
    appearance: Option<String>,
    // Colors (unset = inherit from `base`).
    desktop_bg: Option<String>,
    window_bg: Option<String>,
    title_focus: Option<String>,
    title_blur: Option<String>,
    title_fg: Option<String>,
    border: Option<String>,
    shadow: Option<String>,
    ctrl_fg: Option<String>,
    close_fg: Option<String>,
    menubar_bg: Option<String>,
    dock_bg: Option<String>,
    text: Option<String>,
    dim: Option<String>,
    accent: Option<String>,
    active_bg: Option<String>,
}

/// Apply one optional color string over a default, logging (and keeping the
/// default) when a value is present but unparseable.
fn field(name: &str, dst: Rgba, src: &Option<String>) -> Rgba {
    match src {
        Some(s) => match crate::badge::parse_color(s) {
            Some(c) => c,
            None => {
                crate::dbg_log(&format!("theme: field '{name}' invalid color {s:?}; keeping default"));
                dst
            }
        },
        None => dst,
    }
}

impl ThemeFile {
    fn into_theme(self) -> (Option<String>, Theme) {
        let base = Theme::builtin(&self.base.as_deref().unwrap_or("midnight").to_lowercase());
        let t = Theme {
            desktop_bg: field("desktop_bg", base.desktop_bg, &self.desktop_bg),
            window_bg: field("window_bg", base.window_bg, &self.window_bg),
            title_focus: field("title_focus", base.title_focus, &self.title_focus),
            title_blur: field("title_blur", base.title_blur, &self.title_blur),
            title_fg: field("title_fg", base.title_fg, &self.title_fg),
            border: field("border", base.border, &self.border),
            shadow: field("shadow", base.shadow, &self.shadow),
            ctrl_fg: field("ctrl_fg", base.ctrl_fg, &self.ctrl_fg),
            close_fg: field("close_fg", base.close_fg, &self.close_fg),
            menubar_bg: field("menubar_bg", base.menubar_bg, &self.menubar_bg),
            dock_bg: field("dock_bg", base.dock_bg, &self.dock_bg),
            text: field("text", base.text, &self.text),
            dim: field("dim", base.dim, &self.dim),
            accent: field("accent", base.accent, &self.accent),
            active_bg: field("active_bg", base.active_bg, &self.active_bg),
        };
        (self.name, t)
    }
}

/// Parse a theme TOML string into its name (if the file set one) and `Theme`.
fn parse_theme_toml(text: &str) -> Result<(Option<String>, Theme), String> {
    let tf: ThemeFile = toml::from_str(text).map_err(|e| e.to_string())?;
    Ok(tf.into_theme())
}

/// Serialize a palette to theme-file TOML (`--dump` starter). `#rrggbbaa` only
/// when the color actually carries alpha, else the shorter `#rrggbb`.
pub fn to_file_toml(t: &Theme, name: &str) -> String {
    fn hex(c: Rgba) -> String {
        if c.a == 255 {
            format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)
        }
    }
    format!(
        "name = \"{name}\"\n\
         # base = \"midnight\"   # unset fields inherit from this built-in\n\
         # appearance = \"dark\"\n\n\
         desktop_bg  = \"{}\"\n\
         window_bg   = \"{}\"\n\
         title_focus = \"{}\"\n\
         title_blur  = \"{}\"\n\
         title_fg    = \"{}\"\n\
         border      = \"{}\"\n\
         shadow      = \"{}\"\n\
         ctrl_fg     = \"{}\"\n\
         close_fg    = \"{}\"\n\
         menubar_bg  = \"{}\"\n\
         dock_bg     = \"{}\"\n\
         text        = \"{}\"\n\
         dim         = \"{}\"\n\
         accent      = \"{}\"\n\
         active_bg   = \"{}\"\n",
        hex(t.desktop_bg), hex(t.window_bg), hex(t.title_focus), hex(t.title_blur),
        hex(t.title_fg), hex(t.border), hex(t.shadow), hex(t.ctrl_fg), hex(t.close_fg),
        hex(t.menubar_bg), hex(t.dock_bg), hex(t.text), hex(t.dim), hex(t.accent),
        hex(t.active_bg),
    )
}

/// WCAG relative luminance (sRGB linearized).
fn luminance(c: Rgba) -> f64 {
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio between two colors (1.0 = identical … 21.0 = black/white).
fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Check a theme against the readability invariants the renderer relies on (see
/// the theming doc §5). Returns human-readable warnings; never fails — the theme
/// still loads. Keeps the red-on-green class of bug out of user themes.
pub fn validate(t: &Theme) -> Vec<String> {
    let mut warns = Vec::new();
    let mut check = |a: Rgba, b: Rgba, min: f64, msg: &str| {
        let c = contrast(a, b);
        if c < min {
            warns.push(format!("low contrast ({c:.1}:1 < {min:.1}:1): {msg}"));
        }
    };
    // Buttons draw window_bg text on a saturated fill (close_fg / accent).
    check(t.window_bg, t.close_fg, 2.0, "destructive-button text (window_bg) on fill (close_fg)");
    check(t.window_bg, t.accent, 2.0, "affirmative-button text (window_bg) on fill (accent)");
    // Body/terminal text, and desktop icon labels, both use `text`.
    check(t.text, t.window_bg, 3.0, "body text on window_bg");
    check(t.text, t.desktop_bg, 2.5, "icon-label text on desktop_bg (wallpaper)");
    // An inactive titlebar that matches the wallpaper vanishes.
    check(t.title_blur, t.desktop_bg, 1.12, "inactive titlebar vs wallpaper (too similar)");
    warns
}

/// Resolve the theme directory: `$TETRON_WM_THEMES_DIR`, else the `theme_dir`
/// config key (relative to the config dir), else `<config>/themes`.
fn themes_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("TETRON_WM_THEMES_DIR") {
        return Some(PathBuf::from(d));
    }
    let cfg_dir = crate::config::config_dir();
    match crate::config::Config::load().theme_dir {
        Some(d) => {
            let p = PathBuf::from(&d);
            if p.is_absolute() { Some(p) } else { cfg_dir.map(|c| c.join(p)) }
        }
        None => cfg_dir.map(|c| c.join("themes")),
    }
}

/// Load every `*.toml` in `dir` into `(name, Theme)` pairs. Pure (no globals), so
/// it is unit-testable; malformed files are skipped and logged, not fatal.
fn load_dir(dir: &Path) -> Vec<(String, Theme)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(String, Theme)> = Vec::new();
    for ent in rd.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else {
            continue;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_theme_toml(&text) {
                Ok((name, theme)) => {
                    let name = name.unwrap_or(stem);
                    for w in validate(&theme) {
                        crate::dbg_log(&format!("theme: '{name}': {w}"));
                    }
                    out.push((name, theme));
                }
                Err(e) => crate::dbg_log(&format!("theme: skip {} — {e}", path.display())),
            },
            Err(e) => crate::dbg_log(&format!("theme: cannot read {} — {e}", path.display())),
        }
    }
    out
}

/// The process-wide file-theme registry, scanned once on first use. A new file
/// takes effect on the next daemon/apphost start (`tetron-wm kill` + relaunch).
fn registry() -> &'static Vec<(String, Theme)> {
    static REG: OnceLock<Vec<(String, Theme)>> = OnceLock::new();
    REG.get_or_init(|| themes_dir().as_deref().map(load_dir).unwrap_or_default())
}

/// Write a built-in preset out as an editable theme file in the themes dir,
/// returning the path. Backs `tetron-wm theme --dump <name>`.
pub fn dump_builtin(name: &str) -> std::io::Result<PathBuf> {
    let dir = themes_dir().ok_or_else(|| std::io::Error::other("no themes directory"))?;
    std::fs::create_dir_all(&dir)?;
    let theme = Theme::builtin(&name.to_lowercase());
    let path = dir.join(format!("{name}.toml"));
    std::fs::write(&path, to_file_toml(&theme, name))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_hex_parses() {
        assert_eq!(crate::badge::parse_color("#11223344"), Some(Rgba { r: 0x11, g: 0x22, b: 0x33, a: 0x44 }));
        assert_eq!(crate::badge::parse_color("#112233"), Some(Rgba { r: 0x11, g: 0x22, b: 0x33, a: 255 }));
        assert_eq!(crate::badge::parse_color("#1234"), None); // bad length
    }

    #[test]
    fn partial_file_inherits_base() {
        // `base = "light"` also exercises the deprecated light → pastoral alias.
        let (name, t) = parse_theme_toml("name = \"x\"\nbase = \"light\"\naccent = \"#010203\"\n").unwrap();
        assert_eq!(name.as_deref(), Some("x"));
        assert_eq!(t.accent, Rgba { r: 1, g: 2, b: 3, a: 255 });
        assert_eq!(t.window_bg, Theme::pastoral().window_bg); // unset inherits from base
    }

    #[test]
    fn invalid_color_keeps_base_default_not_fatal() {
        let (_n, t) = parse_theme_toml("accent = \"not-a-color\"\n").unwrap();
        assert_eq!(t.accent, Theme::midnight().accent); // default base = midnight
    }

    #[test]
    fn unknown_keys_are_ignored() {
        // Version-skew: a file from a newer tetron-wm with extra keys still loads.
        let r = parse_theme_toml("accent = \"#abcdef\"\nfuture_field = \"nope\"\n");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().1.accent, Rgba { r: 0xab, g: 0xcd, b: 0xef, a: 255 });
    }

    #[test]
    fn dump_roundtrips_including_alpha() {
        let src = Theme::dracula();
        let (_n, back) = parse_theme_toml(&to_file_toml(&src, "dracula")).unwrap();
        assert_eq!(back.desktop_bg, src.desktop_bg);
        assert_eq!(back.accent, src.accent);
        assert_eq!(back.shadow, src.shadow); // alpha survives
    }

    #[test]
    fn validate_flags_low_contrast_button() {
        let mut t = Theme::midnight();
        t.window_bg = Rgba::rgb(0, 180, 0);
        t.accent = Rgba::rgb(0, 170, 0); // green-on-green: unreadable affirmative button
        assert!(validate(&t).iter().any(|s| s.contains("affirmative")), "{:?}", validate(&t));
    }

    #[test]
    fn validate_passes_shipped_pastoral_and_midnight() {
        assert!(validate(&Theme::midnight()).is_empty(), "{:?}", validate(&Theme::midnight()));
        assert!(validate(&Theme::pastoral()).is_empty(), "{:?}", validate(&Theme::pastoral()));
    }

    #[test]
    fn light_alias_resolves_to_pastoral() {
        // Use builtin() (not named()) to stay hermetic — no registry / home-dir IO.
        assert_eq!(Theme::builtin("light").accent, Theme::pastoral().accent);
        assert_eq!(Theme::builtin("pastoral").accent, Theme::pastoral().accent);
    }

    #[test]
    fn load_dir_reads_toml_and_skips_junk() {
        let dir = std::env::temp_dir().join(format!("tetron-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("solar.toml"), "accent = \"#abcdef\"\n").unwrap();
        std::fs::write(dir.join("notatheme.txt"), "ignored").unwrap();
        std::fs::write(dir.join("broken.toml"), "accent = [unclosed\n").unwrap();
        let loaded = load_dir(&dir);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, "solar");
        assert_eq!(loaded[0].1.accent, Rgba { r: 0xab, g: 0xcd, b: 0xef, a: 255 });
    }
}
