# Theming in tetron-wm — adding presets, and making themes file-based

How the color theme is wired, every surface it must reach (missing one ruins
the cohesive look), the contrast rules a palette must satisfy, and what it
would take to load themes from a user file.

## 1. Architecture

The palette lives in one place: `src/theme.rs`. A `Theme` is a flat `Copy`
struct of 15 `Rgba` colors. A process-global `RwLock<Theme>` holds the *current*
theme so Settings can restyle the live desktop without threading a `Theme`
through every call.

- `theme::current() -> Theme` — snapshot the active theme (called each frame).
- `theme::set(name)` — swap the active theme by preset name.
- `theme::named(name) -> Theme` — resolve a name to a palette (falls back to `midnight`).
- `theme::PRESETS: &[&str]` — the built-in names (Settings cycler, power menu, `theme` CLI).
- `theme::set_terminal_colors(bg, fg)` — apphost-only override of `window_bg`/`text` for the `terminal_bg`/`terminal_fg` config keys.

Every UI surface reads `theme::current()` at render time — it never stores a
color. So changing a preset's palette, or the active theme, is instant.

> **Three processes, three theme globals.** client, daemon, and **apphost** are
> separate processes, each with its own `theme` global. The daemon/client swap
> live on `SetTheme`. The **apphost does not** — it reads the theme once at
> startup (`apphost/server.rs`) from `TETRON_WM_THEME`/`config.theme`. So a
> theme change (and `terminal_bg`/`terminal_fg`) only re-colors already-running
> **terminal cells** after the apphost restarts (`tetron-wm kill` + relaunch),
> not on a bare `reload`.

### The 15 fields (`Theme` struct)

| field | colors |
|-------|--------|
| `desktop_bg`   | wallpaper / root background |
| `window_bg`    | window body; default terminal background; **light text on dark buttons** |
| `title_focus`  | focused window titlebar |
| `title_blur`   | unfocused window titlebar |
| `title_fg`     | titlebar text |
| `border`       | window borders (drawn over a transparent cell — see §4) |
| `shadow`       | drop shadow (has alpha) |
| `ctrl_fg`      | titlebar min/max glyphs |
| `close_fg`     | close glyph; **destructive-button fill** (see §5) |
| `menubar_bg`   | menubar + dock + store side panel |
| `dock_bg`      | dock background |
| `text`         | body text; default terminal foreground; desktop icon labels |
| `dim`          | secondary / hint text |
| `accent`       | selection fg, highlights, affirmative-button fill |
| `active_bg`    | selected-row / active background; icon selection (at alpha 200) |

## 2. Adding a new built-in preset

This touches **one file**, `src/theme.rs`:

1. Add `pub const fn <name>() -> Self { Theme { .. } }` with all 15 fields.
2. Add a `Theme::named()` match arm: `"<name>" => Self::<name>(),`.
3. Add `"<name>"` to `PRESETS`.

The Settings cycler (`settings.rs:224`), power-menu theme list
(`powermenu.rs:47`), and `tetron-wm theme <name>` (`main.rs:62`) all read
`PRESETS`, so the name appears everywhere automatically. Colors use the local
`rgb(r,g,b)` helper (alpha 255); use an `Rgba { .. }` literal for alpha (`shadow`).

**But a palette alone is not a complete theme.** Read §4 and §5 — several
surfaces live outside the 15 fields, and a palette that ignores the contrast
rules looks broken on light backgrounds even though every field is set.

## 3. The consumers (read the theme each frame)

```
compositor.rs  wm.rs  chrome.rs  desktop.rs  tray.rs  launcher.rs
settings.rs  store.rs  activity.rs  filemanager.rs  dirpicker.rs
help.rs  logsview.rs  imageview.rs  powermenu.rs  confirmclose.rs
launchwarn.rs  assistant.rs  ptyhost.rs  session.rs
```

Many widgets alias fields through tiny `pal_*()` / `menu_*()` helpers at the top
of the file (`settings.rs:13`, `store.rs:14`, `activity.rs:19`,
`filemanager.rs:702`, `launcher.rs:19`) — match that pattern. Adding a new
*field* to `Theme` means filling it in **every** preset (the struct has no
`Default`) and wiring the consumer.

> **History:** the theme was originally skin-deep — terminal cells, launcher,
> file manager, and desktop icon labels carried hardcoded *midnight* palettes,
> so switching themes (or using light) left them dark/unreadable. Commit
> `ac78191` routed them all through `theme::current()`. The takeaway for a new
> theme: the risk is never the 15 fields, it is a surface still holding a
> hardcoded color.

## 4. Surfaces beyond the 15 fields (easy to miss)

These are the non-obvious couplings that make a theme look *cohesive* rather
than merely colored. All were learned while building the pastoral theme.

- **Terminal cells** (`ptyhost.rs:251`). An app cell's "default" fg/bg — and the
  ANSI `Foreground`/`Background` named-color fallback — resolve to `th.text` /
  `th.window_bg` (snapshotted once per frame). This is what gives the light
  theme black-on-white terminals. Set in the apphost process (see §1 caveat).
- **Window borders draw over a transparent cell** (`wm.rs:338`). Border glyphs
  use `bg: Rgba::TRANSPARENT`, not `window_bg`. With `window_bg` the border was
  a full opaque cell — a wide white band on the pastoral theme. The thin line glyph
  must float over whatever is behind it.
- **Desktop icon labels** (`desktop.rs:284`) use `th.text` (always contrasts the
  `desktop_bg` wallpaper), and the selection highlight is `active_bg` forced to
  **alpha 200** so the wallpaper shows through. The old hardcoded light grey was
  invisible on the light paper wallpaper.
- **Menubar brand label layout** (`chrome.rs`). Not a color, but a coupling: the
  brand button text (`GO_LABEL = " tetron "`) drives hardcoded column offsets
  `MODE_X`/`ASSIST_X`/`APP_X`. Change the label → recompute them, and check
  `tests/chrome_tests.rs` (byte-vs-char columns for multi-byte glyphs).

## 5. Contrast invariants a palette MUST satisfy

A theme is not just 15 pleasant colors; the following relationships must hold or
specific surfaces become unreadable. The red-on-green dialog bug (`e5b068b`)
came from violating the first one.

- **Buttons = light text on a saturated fill.** The convention (`e5b068b`):
  - *Destructive* (Close, Shut Down, Remove, Restart app server, dangerous
    Launch): text `window_bg` on fill `close_fg`.
  - *Affirmative/benign* (Connect, calendar "today"): text `window_bg` on fill `accent`.
  - The old `close_fg`-text-on-`accent`-fill was red-on-green — unreadable,
    worst on light. **Therefore:** `window_bg` must read clearly on *both*
    `close_fg` and `accent`, and both of those must be saturated enough to serve
    as a fill (not a pale tint).
- **`text` must contrast `window_bg`** (body/terminal) **and `desktop_bg`**
  (icon labels), since it is used on both.
- **`title_blur` must differ visibly from `desktop_bg`** — an inactive titlebar
  that matches the wallpaper vanishes (the pastoral theme uses a taupe darker than
  its paper wallpaper for exactly this).
- **`accent` must be legible as text on panels** (`window_bg`/`menubar_bg`), not
  only as a fill — the pastoral theme darkens its green to `#2E7D32` for label
  legibility while the brighter logo green `#4CAF50` carries the window `border`.

## 6. Known remaining hardcoded colors (coverage gaps)

Still theme-independent today — a fully cohesive theme pass (or a file-based
theme) should route or consciously exempt these:

| site | color | note |
|------|-------|------|
| `imageview.rs:40` | `rgb(200,208,220)` | image-view placeholder label fg (midnight grey) — **low-contrast on light**; a genuine gap. |
| `desktop.rs:641-642` | `rgb(224,228,238)` / `rgb(30,34,46,245)` | desktop right-click overlay menu / rename field — stays a dark box on every theme. |
| `settings.rs:19`, `store.rs:21` | `GREEN rgb(126,231,135)` | "on"/verified/available status markers — semantic success-green, fixed. |
| `activity.rs:25,257` | `RED rgb(241,76,76)`, kill-box `rgb(45,0,0)` | CPU/kill danger colors — semantic, fixed. |

Status/danger colors (`GREEN`/`RED`) are arguably *meant* to stay constant
across themes; the imageview and desktop-overlay ones are true omissions.

## 7. Making themes file-based — feasible

> **Status: implemented** (branch `feat/theme-file-loader`). Theme files live in
> `<config>/themes/*.toml` (override via `theme_dir` config key or
> `$TETRON_WM_THEMES_DIR`). `theme.rs` adds `ThemeFile` (serde, every field
> optional, unknown keys ignored, optional `base` to inherit a built-in),
> `preset_names()` (built-ins + file themes; a same-named file overrides a
> built-in), a registry scanned once on first use, and `validate()` (WCAG-contrast
> warnings, logged not fatal). `badge::parse_color` gained `#rrggbbaa`. CLI:
> `tetron-wm theme --dump <name>` writes a starter, `--list` prints all names. The
> three `PRESETS` call sites now use `preset_names()`. Not yet done: the §6
> semantic-slot widening (success/danger/imageview/overlay) and runtime hot-reload
> (a new file needs `tetron-wm kill` + relaunch). The paragraphs below are the
> original plan, kept for rationale.

**Yes.** Because every surface reads `theme::current()` and nothing (bar §6)
hardcodes a color, a file-loaded palette lights up the whole desktop with no
render changes. A hex/named-color parser already exists:
`badge::parse_color` (`src/badge.rs:20`).

### Proposed shape

- Theme files in `~/.config/tetron-wm/themes/*.toml`, a flat table of the 15
  field names → `#rrggbb` or named color:

  ```toml
  name = "solarized-dark"
  desktop_bg = "#002b36"
  window_bg  = "#073642"
  accent     = "#268bd2"
  # ... all 15
  ```

- A string-typed serde struct `ThemeFile { name, <15 fields> }`, mapped
  field-by-field through `badge::parse_color` into a `Theme`. Keep `Theme`
  itself `Copy`/non-serde; the string form lets colors be hex *or* named.
- Scan the dir at startup into a registry `Vec<(String, Theme)>`.
- `theme::named()` checks built-ins first, then the registry.

### Friction points (all about resolving the preset *list*)

`PRESETS` is a compile-time `&[&str]`, read in three spots; to surface
file themes in the UI, replace those reads with a runtime source:

1. `settings.rs:224` — Settings cycler.
2. `powermenu.rs:47` — power-menu theme list.
3. `main.rs:62` — `theme` CLI usage/validation.

Add `theme::preset_names() -> Vec<String>` (built-ins + registry) and point
those three at it; `Theme::named()` gains one registry lookup.

Also note the §1 multi-process caveat: the **apphost loads its theme at
startup**, so a file theme must be on disk before the apphost starts (it is, if
it lives in the config dir) — terminals still only recolor on apphost restart.

### Effort / risk

Small, contained: new `ThemeFile` + dir scan + registry; swap one `const` slice
for a function in three places. No protocol change, no new `FrameMsg`, no widget
edits. Care points: a malformed file must be skipped and logged (`dbg_log`) with
built-ins still working; a missing field should default rather than abort; and a
file theme should be validated against the §5 contrast invariants (or at least
documented) so a user's theme does not reproduce the red-on-green bug.

## 8. Can themes be *truly* modular — 100% in the file?

**Yes for 100% of the colors; no for the logic — and that split is the point.**
The render code is already data-driven, so a 100% file-based *palette* is a
mechanical expansion. The logic (layout + contrast invariants) must stay in
code precisely so a user's file cannot break it.

### Everything that is a *value* can live in the file

Three additions make the palette fully file-expressible:

1. **Widen `Theme` to cover the §6 holdouts** — add `success`/`danger` (the
   `GREEN`/`RED` status colors), the imageview placeholder fg, and the
   desktop-overlay menu fg/bg. Then no color is left hardcoded.
2. **Carry alpha in the format** — use `#rrggbbaa`, not `#rrggbb`. Several
   subtleties are *opacity*, not hue: `shadow` (a=110/120/51) and the
   icon-selection blend (a=200). Without alpha in the file these are unreachable.
3. **Promote derived button colors to explicit fields** (optional, for strict
   100%) — e.g. `btn_destructive_fg/bg`, `btn_affirmative_fg/bg` — so the file
   owns them instead of code computing `window_bg`-on-`close_fg`.

All additive: no protocol change, no new `FrameMsg`, no render-loop edits.

### What should NOT go in the file (and cannot usefully)

Two of the subtleties are not colors — they are **structural rules constant
across every theme**:

- **Rendering decisions**: border bg is `TRANSPARENT` (§4); icon selection is
  semi-transparent; the menubar brand label drives `MODE_X/ASSIST_X/APP_X`.
  These do not vary per theme, so a file would just repeat them — and could get
  them wrong.
- **Contrast relationships (§5)**: you cannot *store* "window_bg must be legible
  on close_fg." You can only *validate* it.

Pushing these into the file hands every user the ability to reintroduce the
red-on-green bug, per theme, silently. The correct design keeps them centralized
**so a theme physically cannot break them**: the file supplies colors; the code
owns the invariants that consume them.

### Verdict

A 100% file-based palette is clean and achievable. "Truly modular" is best read
as: the file is the single source for all *color data* (incl. alpha and semantic
slots), while the loader adds a **contrast validator** that warns/rejects a theme
failing the §5 rules — turning each invariant from a landmine into a guardrail.
That is *more* robust than the current hardcoded presets, not less. Non-color
knobs people lump into "theme" (brand label string, file-icon glyph style)
belong in general config, not the color file; bundle them later if wanted, but
they are orthogonal.

## 9. Where does the theme file go?

### Where things live today

- **User config**: `~/.config/tetron-wm/config.toml` — XDG, resolved by
  `config.rs:221`, honoring `$XDG_CONFIG_HOME` and falling back to `~/.config`
  on every platform (deliberately *not* `dirs::config_dir()`, to avoid macOS's
  `Library/Application Support`). This is the only user-edited file.
- **Generated runtime state**: `~/.local/share/tetron-wm/` — the data dir, where
  the assistant's `AGENTS.md` is *stamped* at launch. Machine-written, not meant
  for hand editing.
- **Built-in themes**: compiled into the binary (`theme.rs`). No theme file
  exists yet.

That split is the key principle: **themes are user-edited config, so they belong
in the config dir, not the data dir.** Putting them under `~/.local/share` would
fight XDG semantics (data dir = app-generated) and hide them from users.

### Recommendation: `~/.config/tetron-wm/themes/<name>.toml`

One theme per file, in a `themes/` subdir next to `config.toml`:

- **Discoverable**: sits right beside the file users already edit.
- **Drop-in**: share a theme as a single file; copy it in, it appears.
- **XDG for free**: reuse the existing `config_path()` base, so
  `$XDG_CONFIG_HOME` is honored automatically — no new path logic.
- **Name = filename stem** (`nord.toml` → `nord`), with an optional `name =`
  key to override. `config.theme = "<name>"` selects it exactly as today.
- **Survives updates**: lives in the user's home, untouched by the installer.

### Precedence (lets users override a built-in)

Resolve `theme::named(name)` as: **user `themes/` dir → bundled example themes →
compiled built-ins**, first match wins. So a user can drop `nord.toml` in to
*replace* the built-in Nord, or add entirely new names. A malformed user file is
skipped (logged via `dbg_log`), falling through to the built-in — never a hard
failure.

### Flexibility / escape hatches (brainstorm)

- **`theme_dir` config key** (`Option<String>`): point the loader at an
  arbitrary directory (a dotfiles repo, a shared drive). Unset = the default
  `~/.config/tetron-wm/themes/`. Cheap to add, covers power users.
- **System-wide themes**: optionally also scan `/usr/share/tetron-wm/themes/`
  (or `$XDG_DATA_DIRS`) so a distro package can ship themes. Lower precedence
  than the user dir. Probably YAGNI for a single-user TUI — add only if packaged.
- **Inline in `config.toml`** (e.g. `[themes.solarized]` tables): no new dir, but
  clutters config and loses the drop-in/share story. Reject as the primary
  mechanism; a single `[theme.custom]` override block could be a *convenience*
  on top of the dir if ever wanted.
- **Ship starters as examples**: mirror the existing `config.example.toml` with a
  `themes.example/` dir (or `--dump-theme <name>` to write a built-in out as an
  editable file). Gives users a correct, validator-passing template to copy —
  the best defense against the §5 bugs.
- **Hot-reload**: scan on daemon start and refresh on `SetTheme`; optionally a
  `tetron-wm theme --reload` / a Settings "Rescan themes" action so editing a
  file does not require a restart. (Terminals still need an apphost restart per
  the §1 caveat.)

**Bottom line:** default to `~/.config/tetron-wm/themes/<name>.toml` (zero new
path logic, correct XDG, discoverable), with user-dir-wins precedence over
built-ins and an optional `theme_dir` key for everyone else.

---

References (code): `src/theme.rs`, `src/config.rs:69`, `src/badge.rs:20`,
`src/ptyhost.rs:251`, `src/wm.rs:338`, `src/desktop.rs:284`,
`src/settings.rs:224`, `src/powermenu.rs:47`, `src/session.rs:2352`,
`src/apphost/server.rs:17`.
Commits: `208adab` (light palette), `ac78191` (route every surface + terminal
config), `e423199` (Store/Settings/Activity), `e5b068b` (dialog contrast).
