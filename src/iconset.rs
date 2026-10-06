//! File-type icon glyphs for the list view and the no-graphics fallback.
//!
//! The graphical icons (`icons.rs` PNGs via the Kitty graphics protocol) are
//! used when the terminal supports them. Where it does not -- kmscon, a bare
//! Linux VT, plain SSH to a non-graphics terminal -- the file manager and the
//! desktop fall back to a single glyph per file type. The `icon_style` config
//! key picks which glyph set, set process-globally at daemon startup (like the
//! theme):
//!   - `ascii`  (default): BMP geometric/symbol shapes, present in essentially
//!              every console/monospace font and monochrome so they take the
//!              theme color. Renders everywhere.
//!   - `nerd`:  Nerd Font file-type glyphs -- the nicest console result, but
//!              Private-Use-Area, so tofu without a Nerd Font installed.
//!              tetron-os ships one and sets this.
//!   - `emoji`: color emoji -- nice on a GUI terminal, tofu on a console.

use crate::openwith::Role;
use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// Which glyph set the file manager / desktop use for the non-graphics icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    /// BMP geometric shapes -- renders in every font. The default.
    #[default]
    Ascii,
    /// Nerd Font file-type glyphs -- needs a Nerd Font (tetron-os ships one).
    Nerd,
    /// Color emoji -- GUI terminals only.
    Emoji,
}

/// The glyph for `role` in a specific `style`.
pub fn glyph_for(role: Role, style: IconStyle) -> char {
    match style {
        IconStyle::Ascii => match role {
            Role::Directory => '\u{25A0}',  // ■ filled square
            Role::Executable => '\u{25CF}', // ● filled circle
            Role::Text => '\u{25A4}',       // ▤ square with horizontal lines
            Role::Code => '\u{25C7}',       // ◇ unfilled diamond
            Role::Image => '\u{25C6}',      // ◆ filled diamond
            Role::Audio => '\u{266A}',      // ♪ eighth note
            Role::Video => '\u{25B6}',      // ▶ play
            Role::Archive => '\u{25A3}',    // ▣ square within a square
            Role::Pdf => '\u{25A6}',        // ▦ square with grid
            Role::Other => '\u{25A1}',      // □ unfilled square
        },
        IconStyle::Nerd => match role {
            Role::Directory => '\u{F07B}',  //  folder
            Role::Executable => '\u{F120}', //  terminal
            Role::Text => '\u{F0F6}',       //  file-text
            Role::Code => '\u{F1C9}',       //  file-code
            Role::Image => '\u{F1C5}',      //  file-image
            Role::Audio => '\u{F1C7}',      //  file-audio
            Role::Video => '\u{F1C8}',      //  file-video
            Role::Archive => '\u{F1C6}',    //  file-archive
            Role::Pdf => '\u{F1C1}',        //  file-pdf
            Role::Other => '\u{F016}',      //  file
        },
        IconStyle::Emoji => match role {
            Role::Directory => '\u{1F4C1}',  // 📁
            Role::Image => '\u{1F5BC}',      // 🖼
            Role::Audio => '\u{1F3B5}',      // 🎵
            Role::Video => '\u{1F3AC}',      // 🎬
            Role::Archive => '\u{1F4E6}',    // 📦
            Role::Pdf => '\u{1F4D5}',        // 📕
            Role::Executable => '\u{2699}',  // ⚙
            _ => '\u{1F4C4}',                // 📄
        },
    }
}

fn slot() -> &'static RwLock<IconStyle> {
    static STYLE: OnceLock<RwLock<IconStyle>> = OnceLock::new();
    STYLE.get_or_init(|| RwLock::new(IconStyle::Ascii))
}

/// Set the active icon style (daemon startup, from config).
pub fn set(style: IconStyle) {
    *slot().write().unwrap() = style;
}

/// The glyph for `role` in the active style.
pub fn glyph(role: Role) -> char {
    glyph_for(role, *slot().read().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_has_a_glyph_in_every_style() {
        for style in [IconStyle::Ascii, IconStyle::Nerd, IconStyle::Emoji] {
            for role in [
                Role::Directory, Role::Image, Role::Video, Role::Audio, Role::Text,
                Role::Code, Role::Archive, Role::Pdf, Role::Executable, Role::Other,
            ] {
                let g = glyph_for(role, style);
                assert_ne!(g, '\0', "{style:?}/{role:?}");
            }
        }
    }

    #[test]
    fn ascii_glyphs_are_bmp() {
        // The point of the default style: no astral-plane / PUA codepoints.
        for role in [Role::Directory, Role::Other, Role::Executable, Role::Audio, Role::Video] {
            assert!((glyph_for(role, IconStyle::Ascii) as u32) <= 0xFFFF);
        }
    }
}
