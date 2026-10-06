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
    /// The only light preset. `accent` is a darkened green (`#2E7D32`) so button
    /// labels stay legible on the light panels, while the logo green `#4CAF50`
    /// carries the highlight as the window `border`.
    pub const fn light() -> Self {
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

    /// Resolve a theme by name (falls back to `midnight`).
    pub fn named(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            "dracula" => Self::dracula(),
            "light" => Self::light(),
            _ => Self::midnight(),
        }
    }
}

/// The names of the built-in presets (for the Settings cycler).
pub const PRESETS: &[&str] = &["midnight", "nord", "gruvbox", "dracula", "light"];

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
