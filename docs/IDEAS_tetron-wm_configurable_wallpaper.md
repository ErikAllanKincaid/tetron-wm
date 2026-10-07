# Configurable wallpaper for tetron-wm

Status: idea / design (2026-10-06). Let the desktop background be more than a single theme color: a solid color (today), a generated cell pattern, or an image, with a single cell-based engine so every mode works on every surface, including a kmscon VC tty.

## 1. Motivation

A solid color is a fine default and the one the maintainer prefers, but other users will want variety, and tetron-wm's primary deployment runs on a bare console under kmscon where the usual graphical-desktop wallpaper tricks do not apply. The goal is a wallpaper that is configurable, tasteful, and works identically on Ghostty, over SSH, and on a kmscon console, rather than a feature that only lights up on a graphical terminal.

## 2. Current state

The desktop background is one cell buffer filled with the theme's `desktop_bg` color and composited at the bottom of the layer stack: the windowed path fills it and pushes `Layer { z: 0, origin (0,0) }` (`src/session.rs` around lines 3984 and 4015), and the simple-mode path fills the work area with `desktop_bg` as well (around line 4259). Desktop icons, windows, and chrome composite above it. Because the background is already a `CellBuffer`, everything below builds on that same surface rather than introducing a new drawing model.

## 3. Design principle: one cell engine, an optional pixel bonus

The core decision is to treat the wallpaper as **cells** in all three modes. A solid color is a flat cell fill, a pattern is generated cells, and an image is converted to colored cells by a tool such as chafa. Because the output is always a `CellBuffer`, the wallpaper composites on every terminal including a kmscon console, with no dependency on any graphics protocol. A true pixel image via the Kitty graphics protocol is kept as an optional higher-fidelity bonus for capable terminals only (section 6), never as the only way to get an image.

This is also cheaper and more robust than a full-screen Kitty image: the wallpaper is a static cell layer built once and rebuilt only on resize or config change, so there is no per-frame graphics placement and no occlusion fight with the desktop icon tiles (which themselves use the Kitty layer on capable terminals).

## 4. Modes

### 4.1 Solid (default, unchanged)

A flat fill of the theme's `desktop_bg`, exactly as today. This stays the default so an existing setup is untouched.

### 4.2 Pattern (generated cells, works everywhere)

A pattern is generated directly into the wallpaper buffer at the current desktop size, so it renders on every surface. kmscon provides truecolor and a block-capable font (the install script pulls `fonts-terminus`/DejaVu), so patterns look rich on the console, not crude. Candidate generators, each parameterised by one or two theme-derived colors so a pattern tracks the active theme:

- `checker`: two-tone squares of a configurable cell size.
- `gradient`: a vertical or diagonal blend between two colors using background color per cell (and optionally `░▒▓` for extra steps).
- `hatch` / `grid`: thin lines or dots at a configurable spacing, using box-drawing glyphs.
- `tile:<glyph>`: a repeated glyph (for example a faint `·` or a logo cell) in a dim foreground over `desktop_bg`.

Patterns are pure computation, no external dependency, and are the universal upgrade over solid.

### 4.3 Image as cells (chafa, works everywhere)

A raster image is converted to colored cells by a converter and composited like any other wallpaper buffer, so an image wallpaper works on a kmscon VC tty at cell resolution. chafa is the preferred converter: it renders to Unicode half-block, sextant, and octant symbols with truecolor, so on a truecolor console the result is a recognisable image rather than plain ASCII, and chafa is already in the tetron-wm ecosystem (the default config uses `image = "chafa"` as an image handler). libcaca's `img2txt` (colored ANSI) and aalib's `aview` (monochrome) are older fallbacks.

Pipeline:

1. Shell out via `crate::system::run_capped` (hard timeout, never block the render loop) to something like `chafa --format symbols --size <W>x<H> --colors full <image>`, sized to the current desktop cell grid.
2. Parse chafa's SGR output into a `CellBuffer`. A small self-contained parser handles the truecolor SGR sequences chafa emits (`38;2;r;g;b` foreground, `48;2;r;g;b` background, reset) plus the UTF-8 glyphs. This avoids spinning up a terminal emulator just to rasterise one static image.
3. Cache the resulting buffer as the wallpaper; rebuild it on a resize (new `W`x`H`) or a wallpaper-config change.
4. If the converter is absent, fall back to the configured pattern, else to solid, and let the Store offer to install chafa (the Store already detects and offers missing tools).

### 4.4 Optional: pixel image (Kitty graphics, capable terminals only)

On a terminal where `Caps.kitty_graphics` is true (Ghostty, Kitty, WezTerm, or over SSH from one), a full-desktop `ImagePlacement` behind the windows can show a pixel-perfect photo using the existing image layer (the same machinery as the image viewer and the icon tiles). This is strictly a fidelity upgrade requested explicitly; it auto-downgrades to the chafa cell image, then to pattern or solid, on anything without graphics. Given the extra redraw and occlusion handling it needs, it is a later addition, not part of the first version.

## 5. Legibility

A busy wallpaper competes with icon labels and window edges for attention. The engine should support a `dim` factor that darkens or desaturates the generated or converted buffer toward `desktop_bg` before compositing, so text on top stays readable. For the chafa path this can be a post-process over the parsed cells (scale each background color toward the theme background by the dim factor); for patterns it is just the color choice. Default to a modest dim so the out-of-the-box image or pattern never fights the icons.

## 6. Capability matrix

| Surface | solid | pattern | image (chafa cells) | image (Kitty pixels) |
|---|---|---|---|---|
| kmscon VC tty | yes | yes | yes | no (no graphics protocol) |
| raw kernel VT (no kmscon) | yes | yes (limited colors/glyphs) | yes (limited) | no |
| Ghostty / Kitty / WezTerm | yes | yes | yes | yes |
| other terminal over SSH | yes | yes | yes | only if the local terminal has Kitty graphics |

The single cell engine covers the first three columns everywhere. The fourth column is the optional bonus.

## 7. Config schema

A single `wallpaper` key with a small scheme prefix keeps it declarative and shareable across machines (the capability check downgrades gracefully, so the same config never breaks on a console):

```toml
# wallpaper = "solid"                 # default: theme desktop_bg
# wallpaper = "pattern:checker"       # or gradient / hatch / grid / tile:<glyph>
# wallpaper = "image:~/wall.png"      # chafa -> cells; Kitty pixels where available
# wallpaper_dim = 0.4                 # 0.0 = full strength, 1.0 = fully toward desktop_bg
# wallpaper_image_hires = false       # true = use Kitty pixel placement on capable terminals
```

All keys optional with `#[serde(default)]`, so an absent `wallpaper` means solid and old and new configs interoperate. The live Settings panel can expose the mode and the dim factor later.

## 8. Implementation sketch

Follows the existing seams, mostly additive:

- `src/wallpaper.rs` (new): a pure module. Given `(width, height, theme, WallpaperSpec)` it returns a `CellBuffer`. It contains the pattern generators, the SGR-to-cells parser for chafa output, and the dim post-process. It does not touch sockets; the chafa shell-out is passed in or done by the caller through `run_capped`.
- `src/session.rs`: hold the built wallpaper `CellBuffer` plus the spec it was built for; build lazily and cache; invalidate and rebuild on `ClientMsg::Resize` (which already relayouts at line ~1902) and on a theme or wallpaper-config change; composite it as the `z:0` background layer in place of the flat fill at lines ~3984/4015 (and the simple-mode fill at ~4259).
- `src/config.rs`: the `wallpaper`, `wallpaper_dim`, and `wallpaper_image_hires` keys, all defaulted.
- `src/terminal.rs`: reuse `Caps.kitty_graphics` to gate the optional pixel path only.
- Store: the existing missing-tool detection offers to install chafa when an `image:` wallpaper is set and chafa is not found.
- `tests/`: assert a pattern buffer has the expected size and two-color content; assert the SGR parser turns a known chafa-style line into the expected cells; assert the capability gate downgrades `image:` to pattern or solid when chafa or graphics are absent.

## 9. Non-goals and deferrals

Out of the first version: animated or video wallpapers, slideshows or timed rotation, per-workspace or per-monitor wallpapers, and live reload of the image file on change. The Kitty pixel path (section 4.4) is also deferred behind the cell engine. These can layer on later without changing the core cell-buffer approach.

## 10. Open questions

- Pattern catalogue: how many built-in patterns ship, and are their colors always theme-derived or independently configurable.
- Whether `wallpaper_dim` is global or per-mode (an image usually wants more dim than a pattern).
- Caching the converted image across reloads: the daemon survives a UI reload, so the built buffer can persist in memory; persisting it to the state dir is probably not worth the staleness risk.

## 11. Prior art

chafa, image to terminal graphics with truecolor symbol output, <https://hpjansson.org/chafa/>; libcaca and `img2txt`, color ASCII art, <http://caca.zoy.org/wiki/libcaca>; aalib and `aview`, the original ASCII art library, <https://aa-project.sourceforge.net/aalib/>.
