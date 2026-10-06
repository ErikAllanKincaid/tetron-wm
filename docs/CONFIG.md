# tetron-wm (NRO fork) configuration reference

Configuration lives at `~/.config/tetron-wm/config.toml`. Every option has a default, so the file is optional: with no config at all, the defaults below apply. You only add keys you want to change. A ready-to-edit, fully-commented template is `config.example.toml` in the repo root.

This is the NRO fork, whose defaults differ from upstream tuiui in three places (noted below): snapping and window shadows are off, and the default theme is nord.

## Option reference

Each row is the TOML key, its type, the fork default, and what it does.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `snapping_enabled` | bool | `false` (upstream: true) | Drag a window to a screen edge to snap it there. |
| `snap_threshold` | int | `3` | Cells from the edge that trigger a snap (1–10); only applies when snapping is on. |
| `window_shadows` | bool | `false` (upstream: true) | Draw drop shadows behind windows. |
| `natural_scroll` | bool | `true` (NRO default) | "Natural" wheel/touchpad scroll: down goes back into history. Set `false` for traditional. Does not affect Shift+PageUp/PageDown. |
| `launch_maximized` | bool | `false` | Open new app windows maximized (filling the work area). |
| `auto_tile` | bool | `false` | Auto-arrange all windows into the tiling grid as they open and close. |
| `grid_rows` | int | `2` | Tiling grid rows (1–6). |
| `grid_cols` | int | `2` | Tiling grid columns (1–6). |
| `tile_gap` | int | `0` | Cells of gutter between tiled windows. |
| `theme` | string | `"nord"` (upstream: midnight) | Color theme: a built-in (`midnight`, `nord`, `gruvbox`, `dracula`, `pastoral`) or the stem of a theme file (see `theme_dir`). A file with a built-in's name overrides it. (`light` is a deprecated alias for `pastoral`.) |
| `theme_dir` | string | `<config>/themes` | Directory of `*.toml` theme files. Relative paths resolve against the config dir; also overridable via `$TETRON_WM_THEMES_DIR`. A theme file is a flat table of palette fields (`desktop_bg`, `window_bg`, `title_focus`, `title_blur`, `title_fg`, `border`, `shadow`, `ctrl_fg`, `close_fg`, `menubar_bg`, `dock_bg`, `text`, `dim`, `accent`, `active_bg`) → `#rrggbb` / `#rrggbbaa` / named color; optional `name` and `base` (built-in to inherit unset fields from). Create one with `tetron-wm theme --dump <name>`; list all names with `tetron-wm theme --list`. Low-contrast themes are warned about in the debug log, never rejected. |
| `icon_style` | string | `"ascii"` | File-type icon glyphs where graphical (Kitty) icons can't render (kmscon, bare VT, SSH): `ascii` (BMP shapes, renders everywhere), `nerd` (Nerd Font glyphs, needs one installed — tetron-os ships it), or `emoji`. |
| `truecolor` | bool | auto-detect | Force 24-bit color on/off. Unset = detect from known truecolor terminals + `COLORTERM`. Set `true` when the terminal supports truecolor but omits `COLORTERM` (common with `TERM=xterm-256color`), else subtle themes quantize to the 256-color cube (nord → teal/navy). `TETRON_WM_TRUECOLOR=1/0` env overrides this. |
| `default_project_dir` | string | home (`~`) | Directory the working-directory picker opens at. |
| `show_hidden_dirs` | bool | `false` | Show hidden (dot) directories in the picker by default. |
| `filemanager_view` | string | `list` | File-manager view: `icon` or `list`. List is the default; this override key round-trips but is not yet applied at startup. |
| `recent_dirs` | list of string | `[]` | Recently used working directories; maintained by the app, not hand-set. |
| `desktop_enabled` | bool | `true` | Show icons on the wallpaper/desktop. |
| `desktop_pins` | list of table | none | Pinned desktop shortcuts (see "App entries"); otherwise the desktop shows only real ~/Desktop contents. |
| `desktop_positions` | map | `{}` | Saved desktop icon positions; maintained by the app. |
| `assistant_command` | string | `opencode` | Agent CLI the ✦ assistant panel runs. |
| `assistant_args` | list of string | `[]` | Extra arguments passed to the assistant CLI. |
| `assistant_mode` | string | `"panel"` | How the assistant opens: `panel` (right-docked) or `window` (floating). |
| `update_branch` | string | `"main"` | In-app updater channel: `main` (stable prebuilt) or `dev`. |
| `apps` | list of table | `[]` | Apps auto-started at launch and shown in the dock (see "App entries"). |
| `launcher` | list of table | `[]` | Apps offered in the launcher/spotlight; falls back to `apps` when empty. |
| `default_apps` | map string→string | built-in | File-role → handler command map, for "open with". |
| `dock_badges` | map string→string | claude=orange, kilo=yellow | Per-app dock badge colors: a keyword (case-insensitive substring of the app name or command) mapped to a named color or `#rrggbb`. |

## App entries

`apps`, `launcher`, and `desktop_pins` are arrays of tables. Each entry's fields:

| Field | Type | Meaning |
|---|---|---|
| `name` | string | Label shown in the titlebar and dock. |
| `command` | string | Executable to run (or a `@builtin` like `@files`, `@store`). |
| `args` | list of string | Arguments for the command. |
| `category` | string | Grouping shown in the launcher menu. |
| `requires_cwd` | bool | Prompt for a working directory before launching. |
| `cwd` | string | Fixed working directory to start in. |
| `cli` | bool | Wrap a CLI tool so its window does not exit instantly (gives it a shell). |
| `warn` | string | Confirmation text shown before launching (for destructive tools). |

Example:

```toml
[[apps]]
name = "Shell"
command = "bash"
args = []

[[launcher]]
name = "htop"
command = "htop"
cli = true
category = "System"
```

## Keyboard and mouse (not config, built in)

These behaviors are fixed in this fork and do not need configuration:

- Select text in a terminal window by left-drag (highlighted).
- Paste the primary selection: middle-click, three-finger tap, or `Shift+Insert`.
- Clipboard: `Ctrl+Shift+C` copies the selection (and to the host via OSC 52), `Ctrl+Shift+V` pastes it.
- Scrollback: mouse wheel, or `Shift+PageUp` / `Shift+PageDown`.
- Hold `Shift` to force terminal selection/paste even over an app that is grabbing the mouse.

## Notes

- Changes take effect on the next tetron-wm launch (Settings → Appearance can toggle a subset live).
- The live Settings UI exposes snapping, snap threshold, grid rows/cols, gap, auto-tile, launch-maximized, shadows, theme, assistant, and updates; the remaining keys are file-only or app-managed.
