//! What the client's display can show, in one shared place.
//!
//! The client owns the real terminal, so it is the only process that can detect
//! the display (colour depth, bare kernel VT, graphics). It sends a
//! [`ClientCaps`] to the daemon right after attaching (`ClientMsg::Caps`), and
//! the daemon stores it here as a process-global, the same pattern as
//! `theme::current()` and `iconset::style()`. Render code then asks this module
//! instead of reading `TERM` or guessing per widget:
//!
//! - `theme::current()` adapts the palette to [`depth`] (see `theme::adapt`),
//!   so every widget gets colours that survive the client's quantizer.
//! - `wallpaper` picks chafa's colour mode and symbol set from [`get`].
//!
//! Extending: add a `#[serde(default)]` field to [`ClientCaps`] (an older
//! daemon ignores unknown fields; an older client leaves the default), detect it
//! in `terminal::Caps::detect`, and read it here. Prefer fields to new enum
//! variants: an unknown variant fails the whole message parse.

use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// How many colours the client terminal can show. Ordered from least to most,
/// so `depth >= ColorDepth::Ansi256` reads naturally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorDepth {
    /// Black and white: no colour SGR at all, only attributes (reverse, bold,
    /// dim, underline). `NO_COLOR`, `TETRON_WM_COLORS=mono`, dumb/vt100 terminals.
    Mono,
    /// The 16 ANSI colours (8 backgrounds on a bare kernel VT).
    Ansi16,
    /// The xterm 256-colour palette.
    Ansi256,
    /// 24-bit colour. The default, so a daemon that never hears from a client
    /// (or hears from an older one) renders exactly as before.
    #[default]
    TrueColor,
}

impl ColorDepth {
    /// Parse a `TETRON_WM_COLORS` value. `None` means "auto" (or unrecognised),
    /// so detection carries on with the other signals.
    pub fn parse(v: &str) -> Option<ColorDepth> {
        match v.trim().to_ascii_lowercase().as_str() {
            "mono" | "2" | "none" | "bw" => Some(ColorDepth::Mono),
            "16" | "ansi16" | "8" => Some(ColorDepth::Ansi16),
            "256" | "ansi256" => Some(ColorDepth::Ansi256),
            "truecolor" | "24bit" | "16m" => Some(ColorDepth::TrueColor),
            _ => None,
        }
    }

    /// Short name for logs and the Settings/about display.
    pub fn name(self) -> &'static str {
        match self {
            ColorDepth::Mono => "mono",
            ColorDepth::Ansi16 => "16",
            ColorDepth::Ansi256 => "256",
            ColorDepth::TrueColor => "truecolor",
        }
    }
}

/// The display facts the client sends the daemon. Every field is
/// `#[serde(default)]` so either side can be older than the other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCaps {
    #[serde(default)]
    pub colors: ColorDepth,
    /// A bare kernel VT (`TERM=linux`): PSF fonts with no half blocks, 8
    /// background colours. Used for the wallpaper symbol set and glyph choices.
    #[serde(default)]
    pub vt: bool,
    /// The terminal accepts Kitty graphics (image icons, image viewer).
    #[serde(default)]
    pub kitty_graphics: bool,
}

/// The 16 ANSI colours as the Linux console (VGA text mode) renders them. The
/// one palette both the client quantizer (`terminal`) and the 16-colour
/// wallpaper parser (`wallpaper`) use, so a colour survives the round trip.
pub const VGA16: [crate::cell::Rgba; 16] = {
    use crate::cell::Rgba;
    const fn c(r: u8, g: u8, b: u8) -> Rgba { Rgba { r, g, b, a: 255 } }
    [
        c(0, 0, 0), c(170, 0, 0), c(0, 170, 0), c(170, 85, 0),
        c(0, 0, 170), c(170, 0, 170), c(0, 170, 170), c(170, 170, 170),
        c(85, 85, 85), c(255, 85, 85), c(85, 255, 85), c(255, 255, 85),
        c(85, 85, 255), c(255, 85, 255), c(85, 255, 255), c(255, 255, 255),
    ]
};

fn slot() -> &'static RwLock<ClientCaps> {
    static CAPS: OnceLock<RwLock<ClientCaps>> = OnceLock::new();
    CAPS.get_or_init(|| RwLock::new(ClientCaps::default()))
}

/// The attached client's display (daemon side). Defaults until a client reports.
pub fn get() -> ClientCaps {
    *slot().read().unwrap()
}

/// The attached client's colour depth (daemon side).
pub fn depth() -> ColorDepth {
    get().colors
}

/// Record the attached client's display. Returns whether anything changed, so
/// the caller only rebuilds what depends on it (wallpaper) when needed.
pub fn set(caps: ClientCaps) -> bool {
    let mut cur = slot().write().unwrap();
    let changed = *cur != caps;
    *cur = caps;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_the_documented_spellings() {
        assert_eq!(ColorDepth::parse("mono"), Some(ColorDepth::Mono));
        assert_eq!(ColorDepth::parse(" 2 "), Some(ColorDepth::Mono));
        assert_eq!(ColorDepth::parse("16"), Some(ColorDepth::Ansi16));
        assert_eq!(ColorDepth::parse("256"), Some(ColorDepth::Ansi256));
        assert_eq!(ColorDepth::parse("TrueColor"), Some(ColorDepth::TrueColor));
        assert_eq!(ColorDepth::parse("auto"), None);
        assert_eq!(ColorDepth::parse(""), None);
    }

    #[test]
    fn depth_orders_least_to_most() {
        assert!(ColorDepth::Mono < ColorDepth::Ansi16);
        assert!(ColorDepth::Ansi16 < ColorDepth::Ansi256);
        assert!(ColorDepth::Ansi256 < ColorDepth::TrueColor);
    }

    #[test]
    fn caps_wire_format_tolerates_missing_and_unknown_fields() {
        // An older client sends nothing we know: everything defaults.
        let c: ClientCaps = serde_json::from_str("{}").unwrap();
        assert_eq!(c, ClientCaps::default());
        // A newer client sends a field we do not know yet: ignored.
        let c: ClientCaps = serde_json::from_str(r#"{"colors":"ansi16","vt":true,"future":1}"#).unwrap();
        assert_eq!(c.colors, ColorDepth::Ansi16);
        assert!(c.vt);
    }
}
