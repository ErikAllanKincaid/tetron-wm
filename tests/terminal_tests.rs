use tetron_wm::terminal::{Caps, detect_depth, frame_to_ansi};
use tetron_wm::display::ColorDepth;
use tetron_wm::compositor::CellChange;
use tetron_wm::cell::{Cell, Rgba};

fn caps(depth: ColorDepth, vt: bool) -> Caps {
    Caps { depth, vt, pixel_mouse: false, kitty_graphics: false }
}

/// The SGR sequence emitted for one cell with `fg` on `bg`.
fn sgr(caps: &Caps, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> String {
    let cell = Cell { fg: Rgba::rgb(fg.0, fg.1, fg.2), bg: Rgba::rgb(bg.0, bg.1, bg.2), ..Default::default() };
    frame_to_ansi(&[CellChange { x: 0, y: 0, cell }], caps)
}

#[test]
fn truecolor_change_emits_sgr_and_glyph() {
    let caps = caps(ColorDepth::TrueColor, false);
    let changes = vec![CellChange { x: 2, y: 1, cell: Cell { ch: 'A', fg: Rgba::rgb(255,0,0), bg: Rgba::rgb(0,0,0), attrs: Default::default() } }];
    let out = frame_to_ansi(&changes, &caps);
    // cursor move to row 2 col 3 (1-based), set truecolor fg 255;0;0, print A
    assert!(out.contains("\x1b[2;3H"));
    assert!(out.contains("38;2;255;0;0"));
    assert!(out.contains('A'));
}

#[test]
fn no_changes_emits_nothing() {
    let caps = caps(ColorDepth::TrueColor, false);
    assert_eq!(frame_to_ansi(&[], &caps), "");
}

#[test]
fn dark_grey_uses_grayscale_ramp_not_black_in_256() {
    let caps = caps(ColorDepth::Ansi256, false);
    let changes = vec![CellChange { x: 0, y: 0, cell: Cell { ch: ' ', fg: Rgba::rgb(200,200,200), bg: Rgba::rgb(44,46,50), attrs: Default::default() } }];
    let out = frame_to_ansi(&changes, &caps);
    assert!(out.contains("48;5;235"), "dark grey should map to grayscale ramp 235, got: {out}");
    assert!(!out.contains("48;5;16"), "dark grey must not collapse to black (16)");
}

#[test]
fn ansi16_emits_plain_sgr_with_dark_backgrounds_only() {
    let caps = caps(ColorDepth::Ansi16, true);
    let sgr = |fg, bg| sgr(&caps, fg, bg);
    // bright white on near-black -> fg 97, bg 40; no 38;/48; extended forms.
    let out = sgr((255, 255, 255), (10, 10, 10));
    assert!(out.contains("0;97;40"), "{out:?}");
    assert!(!out.contains("38;") && !out.contains("48;"));
    // bright red bg falls back to the dark half (41), never 101.
    assert!(sgr((255, 255, 255), (255, 0, 0)).contains(";41"));
}

#[test]
fn ansi16_contrast_guard_rescues_same_index_pairs() {
    let c = caps(ColorDepth::Ansi16, true);
    // Light grey text on a light grey bar: both quantize to VGA 7. The guard
    // swaps the fg to black (30) rather than drawing grey on grey.
    let out = sgr(&c, (180, 180, 180), (170, 170, 170));
    assert!(out.contains("0;30;47"), "{out:?}");
    // Dark grey "dim" text on black stays dark grey (90): readable as is.
    assert!(sgr(&c, (85, 85, 85), (0, 0, 0)).contains("0;90;40"));
}

#[test]
fn ansi16_off_vt_may_use_bright_backgrounds() {
    let out = sgr(&caps(ColorDepth::Ansi16, false), (0, 0, 0), (255, 255, 255));
    assert!(out.contains(";107"), "{out:?}");
}

#[test]
fn mono_emits_no_colors_and_reverses_highlights() {
    let c = caps(ColorDepth::Mono, false);
    // Normal text on a dark background: plain reset, no colour parameters.
    let plain = sgr(&c, (200, 200, 200), (20, 20, 20));
    assert!(plain.contains("\x1b[0m "), "{plain:?}");
    assert!(!plain.contains("38;") && !plain.contains(";3") && !plain.contains("48;"));
    // A light background is a highlight: reverse video.
    assert!(sgr(&c, (0, 0, 0), (220, 220, 220)).contains("\x1b[0;7m"));
    // Dark text on dark is secondary: dim. Near-white is emphasis: bold.
    assert!(sgr(&c, (100, 100, 100), (0, 0, 0)).contains("\x1b[0;2m"));
    assert!(sgr(&c, (255, 255, 255), (0, 0, 0)).contains("\x1b[0;1m"));
}

#[test]
fn depth_detection_order() {
    // TETRON_WM_COLORS wins over everything.
    assert_eq!(detect_depth(Some("256"), true, "linux", true), ColorDepth::Ansi256);
    assert_eq!(detect_depth(Some("mono"), false, "xterm-kitty", true), ColorDepth::Mono);
    // `auto` falls through; NO_COLOR then means mono.
    assert_eq!(detect_depth(Some("auto"), true, "xterm-256color", true), ColorDepth::Mono);
    // TERM classes.
    assert_eq!(detect_depth(None, false, "vt100", true), ColorDepth::Mono);
    assert_eq!(detect_depth(None, false, "linux", true), ColorDepth::Ansi16);
    // Then the truecolor signal.
    assert_eq!(detect_depth(None, false, "xterm-256color", true), ColorDepth::TrueColor);
    assert_eq!(detect_depth(None, false, "xterm-256color", false), ColorDepth::Ansi256);
}

#[test]
fn inverse_is_resolved_by_swapping_not_sgr7() {
    // Bright white on black, inverted, on a VT: the bg must be a real (dark
    // half) light colour, never SGR 7 that the VT would show as black.
    let c = caps(ColorDepth::Ansi16, true);
    let mut cell = Cell { fg: Rgba::rgb(255, 255, 255), bg: Rgba::rgb(0, 0, 0), ..Default::default() };
    cell.attrs.inverse = true;
    let out = frame_to_ansi(&[CellChange { x: 0, y: 0, cell }], &c);
    assert!(out.contains(";47m"), "{out:?}");
    assert!(!out.contains(";7m"), "{out:?}");
}

#[test]
fn vt_256_guards_against_the_kernels_own_reduction() {
    // `windows` theme dim text (#808080) on a white window: the kernel shows
    // both as its grey (7). Off a VT the code is untouched; on a VT the text
    // switches to black, still as a 256-colour code.
    let off = sgr(&caps(ColorDepth::Ansi256, false), (128, 128, 128), (255, 255, 255));
    assert!(off.contains("38;5;244"), "{off:?}");
    let on = sgr(&caps(ColorDepth::Ansi256, true), (128, 128, 128), (255, 255, 255));
    assert!(on.contains("38;5;16;48;5;231"), "{on:?}");
    // Truecolor on a VT gets the same guard.
    let tc = sgr(&caps(ColorDepth::TrueColor, true), (128, 128, 128), (255, 255, 255));
    assert!(tc.contains("38;2;0;0;0"), "{tc:?}");
    // Readable pairs pass through.
    assert!(sgr(&caps(ColorDepth::Ansi256, true), (0, 0, 0), (255, 255, 255)).contains("38;5;16"));
}

#[test]
fn vt_256_keeps_the_hue_the_kernel_would_lose() {
    // Teal desktop (#008080): cube index 30 decodes to (0,85,85) in the kernel,
    // a black background. On a VT the basic cyan index (6) is sent instead.
    let out = sgr(&caps(ColorDepth::Ansi256, true), (255, 255, 255), (0, 128, 128));
    assert!(out.contains(";48;5;6m"), "{out:?}");
    // Off a VT the cube index stays.
    assert!(sgr(&caps(ColorDepth::Ansi256, false), (255, 255, 255), (0, 128, 128)).contains(";48;5;30m"));
}

