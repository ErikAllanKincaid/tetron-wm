# tetron-wm configuration reference

Configuration lives at `~/.config/tetron-wm/config.toml`. Every option has a default, so the file is optional: with no config at all, the defaults below apply. You only add keys you want to change. A ready-to-edit, fully-commented template is `config.example.toml` in the repo root.

Out of the box, snapping and window shadows are off and the default theme is newmoon.

## Option reference

Each row is the TOML key, its type, the default, and what it does.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `snapping_enabled` | bool | `false` | Drag a window to a screen edge to snap it there. |
| `snap_threshold` | int | `3` | Cells from the edge that trigger a snap (1–10); only applies when snapping is on. |
| `window_shadows` | bool | `false` | Draw drop shadows behind windows. |
| `natural_scroll` | bool | `true` | "Natural" wheel/touchpad scroll: down goes back into history. Set `false` for traditional. Does not affect Shift+PageUp/PageDown. |
| `launch_maximized` | bool | `false` | Open new app windows maximized (filling the work area). |
| `auto_tile` | bool | `false` | Auto-arrange all windows into the tiling grid as they open and close. |
| `grid_rows` | int | `2` | Tiling grid rows (1–6). |
| `grid_cols` | int | `2` | Tiling grid columns (1–6). |
| `tile_gap` | int | `0` | Cells of gutter between tiled windows. |
| `theme` | string | `"newmoon"` | Color theme: a built-in (`newmoon`, `midnight`, `nord`, `gruvbox`, `dracula`, `pastoral`) or the stem of a theme file (see `theme_dir`). A file with a built-in's name overrides it. (`light` is a deprecated alias for `pastoral`.) |
| `theme_dir` | string | `<config>/themes` | Directory of `*.toml` theme files. Relative paths resolve against the config dir; also overridable via `$TETRON_WM_THEMES_DIR`. A theme file is a flat table of palette fields (`desktop_bg`, `window_bg`, `title_focus`, `title_blur`, `title_fg`, `border`, `shadow`, `ctrl_fg`, `close_fg`, `menubar_bg`, `dock_bg`, `text`, `dim`, `accent`, `active_bg`) → `#rrggbb` / `#rrggbbaa` / named color; optional `name` and `base` (built-in to inherit unset fields from). Create one with `tetron-wm theme --dump <name>`; list all names with `tetron-wm theme --list`. Low-contrast themes are warned about in the debug log, never rejected. |
| `icon_style` | string | `"ascii"` | File-type icon glyphs where graphical (Kitty) icons can't render (kmscon, bare VT, SSH): `ascii` (BMP shapes, renders everywhere), `nerd` (Nerd Font glyphs, needs one installed — tetron-os ships it), or `emoji`. |
| `terminal_bg` | string | unset (follow theme) | Default background inside hosted app windows, independent of the desktop theme: `#rrggbb` or a named color. Unset follows the theme's `window_bg`. Applies live to running apps (no app-server restart) when the app server supports it (protocol v4+). Settings → Terminal edits it with presets, hex entry and a Preview / Save / Revert flow. |
| `terminal_fg` | string | unset (follow theme) | Default foreground inside hosted app windows: `#rrggbb` or a named color. Unset follows the theme's `text`. Same live apply and Settings section as `terminal_bg`. |
| `truecolor` | bool | auto-detect | Force 24-bit color on/off. Unset = detect from known truecolor terminals + `COLORTERM`. Set `true` when the terminal supports truecolor but omits `COLORTERM` (common with `TERM=xterm-256color`), else subtle themes quantize to the 256-color cube (nord → teal/navy). `TETRON_WM_TRUECOLOR=1/0` env overrides this. |
| `wallpaper` | string | `"wallpaper.jpg"` | Desktop wallpaper image, rendered to colored cells via `chafa` (so it works on any terminal incl. a kmscon console) and composited behind the windows. An absolute path, a `~`-path, or a path relative to the config dir (a `wallpapers/` subdir is checked first, then the config root) — so a bare filename means "drop it in `~/.config/tetron-wm/wallpapers/`". If the default `wallpaper.jpg` is not on disk, a built-in default image (70 KB, stamped to `~/.local/share/tetron-wm/default-wallpaper.jpg`) is used; any other missing file, or a missing `chafa` → solid desktop (logged). `TETRON_WM_WALLPAPER=<path>` env overrides this. |
| `wallpaper_enabled` | bool | `true` | Master on/off for the wallpaper. When `false`, the desktop is solid even if `wallpaper` names an image. (Settings → Appearance toggles this.) |
| `default_project_dir` | string | home (`~`) | Directory the working-directory picker opens at. |
| `show_hidden_dirs` | bool | `false` | Show hidden (dot) directories in the picker by default. |
| `filemanager_view` | string | `list` | File-manager view: `icon` or `list`. List is the default; this override key round-trips but is not yet applied at startup. |
| `recent_dirs` | list of string | `[]` | Legacy. Recently used working directories now live in `~/.local/share/tetron-wm/state.toml`. An old value here is migrated once and then removed from `config.toml`. |
| `desktop_enabled` | bool | `true` | Show icons on the wallpaper/desktop. |
| `desktop_scrim` | string | unset (semi-transparent black) | Backing drawn behind desktop icon glyphs and labels **when a wallpaper is shown**, for legibility. A color (`#rrggbb` / `#rrggbbaa` / named); `"none"` or `"off"` removes it so labels sit directly on the wallpaper (white ink). Unset = `#000000c8`. No effect on the solid desktop. (Settings → Appearance → "Icon label scrim" toggles it on/off.) |
| `desktop_pins` | list of table | none | Pinned desktop shortcuts (see "App entries"); otherwise the desktop shows only real ~/Desktop contents. |
| `desktop_positions` | map | `{}` | Legacy. Saved desktop icon positions now live in `~/.local/share/tetron-wm/state.toml`. An old value here is migrated once and then removed from `config.toml`. |
| `assistant_command` | string | `opencode` | Agent CLI the ✦ assistant panel runs. |
| `assistant_args` | list of string | `[]` | Extra arguments passed to the assistant CLI. |
| `assistant_mode` | string | `"panel"` | How the assistant opens: `panel` (right-docked) or `window` (floating). |
| `update_branch` | string | `"main"` | In-app updater channel: `main` (stable prebuilt) or `dev`. |
| `apps` | list of table | `[]` | Apps auto-started at launch and shown in the taskbar (see "App entries"). |
| `launcher` | list of table | `[]` | Apps offered in the launcher/spotlight; falls back to `apps` when empty. |
| `default_apps` | map string→string | built-in | File-role → handler command map, for "open with". |
| `dock_badges` | map string→string | claude=orange, kilo=yellow | Per-app badge colors (shown on the taskbar pill): a keyword (case-insensitive substring of the app name or command) mapped to a named color or `#rrggbb`. |
| `screenshot_dir` | string | `""` | Directory for screenshots taken with the leader `P` key or launcher → tetron-wm → Screenshot. Empty uses `~/Pictures` if it exists, else `$HOME`. |
| `web_home` | string | `"https://lite.duckduckgo.com/lite/"` | Start page for text web browsers launched with no URL (w3m exits at once without one). Empty launches them bare. |
| `leader` | string | `"ctrl+space"` | Leader chord, `ctrl+<key>`. Needs `tetron-wm reload`. |
| `keys` | table of action → key or list | built-in | `[keys]` leader-key overrides per action, e.g. `launcher = "l"` or `launcher = ["l", "enter"]`. Actions and their defaults are listed by `tetron-wm --help`. An entry with no valid key is ignored and logged. |
| `clipboard_history` | bool | `true` | Keep a history of everything copied (OSC 52 stores, `Ctrl+Shift+C`, mouse selections, the `clip` CLI). Browse it from the menubar clipboard segment. |
| `clipboard_persist` | bool | `true` | Save the history to `~/.local/share/tetron-wm/clipboard-history.jsonl` (mode 0600). `false` keeps it in memory only. |
| `clipboard_record_selection` | bool | `true` | Record mouse text selections in the history, not just explicit copies. |
| `clipboard_history_size` | int | `100` | Maximum unpinned history entries. |
| `clipboard_item_max_kb` | int | `256` | Entries larger than this many KB are not recorded. |
| `clipboard_ttl_secs` | int | `0` | Expire unpinned entries after this many seconds; `0` never expires. |
| `clipboard_ignore_apps` | list of string | `[]` | Do not record copies made while an app whose name or command contains one of these (case-insensitive) is focused, e.g. password managers. |
| `clipboard_shims` | string | `"auto"` | Put `xclip`/`xsel`/`wl-copy`/`wl-paste` shims on the `PATH` of launched apps so they use this clipboard: `auto` (only when there is no display), `on`, `off`. |
| `clipboard_fake_display` | bool | `false` | Give launched apps a dummy `WAYLAND_DISPLAY` when there is no display, so tools that only look for a clipboard helper when a display variable is set (Neovim) use the shims. Programs that test for a desktop session (`xdg-open`, GUI toolkits) then think one exists, so it is off by default. |
| `clipboard_gui_bridge` | string | `"auto"` | Mirror the real X11/Wayland clipboard into the history: `auto` (when a display exists), `on`, `off`. |
| `clipboard_osc52_read` | string | `"focused"` | Who may READ the clipboard with an OSC 52 query: `focused` (only the focused window), `always`, `never`. |
| `tray_tools` | map string→string or list | built-in | Command for the tray popovers' `▸ Open …` row, per item: `cpu`, `mem`, `battery`, `bluetooth`, `network`. A value is one command line or an ordered list; the first whose program is on `$PATH` wins. Prefix a command with `hold:` to run it through `sh -c` and keep the window open until Enter (for one-shot output). An empty list removes the row. Defaults: cpu `htop`, else `top` (even if not detected); mem `hold:free -h`, `hold:vm_stat`; battery `jolt`, `hold:upower -i $(upower -e \| grep -m1 -i bat)`, `hold:pmset -g batt`; bluetooth `bluetuith`, `bluetoothctl`, `hold:blueutil --paired`; network `wlctl`, `nmtui`, `iwctl`, `hold:ip -br addr`, `hold:networksetup -listallhardwareports`, `hold:ifconfig`. |

## App entries

`apps`, `launcher`, and `desktop_pins` are arrays of tables. Each entry's fields:

| Field | Type | Meaning |
|---|---|---|
| `name` | string | Label shown in the titlebar and taskbar. |
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

These behaviors are fixed in tetron-wm and do not need configuration:

- Select text in a terminal window by left-drag (highlighted).
- Paste the primary selection: middle-click, three-finger tap, or `Shift+Insert`.
- Clipboard: `Ctrl+Shift+C` copies the selection (and to the host via OSC 52), `Ctrl+Shift+V` pastes it.
- Scrollback: mouse wheel, or `Shift+PageUp` / `Shift+PageDown`.
- Hold `Shift` to force terminal selection/paste even over an app that is grabbing the mouse.

## Notes

- Changes take effect on the next tetron-wm launch, or after `tetron-wm reload` (Settings can change a subset live).
- `config.toml` is yours. Saving from the app (a Settings change, a pin, the theme) rewrites only the keys that changed: your comments, key order, commented-out blocks and unknown keys stay, and a hand edit to any other key is not reverted even while tetron-wm is running (run `tetron-wm reload` to make it take effect). Nothing is written when nothing changed. Two limits: a comment written directly above a key goes with that key if the key is removed, and when a whole `[[launcher]]`, `[[apps]]` or `[[desktop_pins]]` list changes, comments inside that list are not kept.
- The first time the file is rewritten in a session the old copy is saved as `config.toml.bak`. A file with a syntax error is never overwritten: tetron-wm logs the error (see the Logs viewer), runs on defaults, and saves again once the file parses.
- App-managed data is not in `config.toml`: desktop icon positions and recent working directories live in `~/.local/share/tetron-wm/state.toml` (`$XDG_DATA_HOME` aware). Old `recent_dirs` and `desktop_positions` keys are moved there once and removed from `config.toml`.
- The live Settings UI exposes snapping, snap threshold, grid rows/cols, gap, auto-tile, launch-maximized, natural scroll, shadows, theme, wallpaper on/off, wallpaper file, icon-label scrim, icon style, desktop icons, clipboard (including entry expiry and max item size), terminal colors (presets, hex, preview), assistant, and updates; the remaining keys are file-only or app-managed.
