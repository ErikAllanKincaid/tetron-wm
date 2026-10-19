# tetron-wm configuration reference

Configuration lives at `~/.config/tetron-wm/config.toml`. A ready-to-copy file with every key at its default and a short note per key is `config.example.toml` in the repository root; this document is the full reference. Every option has a default, so the file is optional: with no config at all, the defaults below apply. You only add keys you want to change. A ready-to-edit, fully-commented template is `config.example.toml` in the repo root.

Out of the box, snapping and window shadows are off and the default theme is newmoon.

## Option reference

Each row is the TOML key, its type, the default, and what it does.

| Key                          | Type                              | Default                               | Meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------- | --------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `snapping_enabled`           | bool                              | `false`                               | Drag a window to a screen edge to snap it there.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `snap_threshold`             | int                               | `3`                                   | Cells from the edge that trigger a snap (1–10); only applies when snapping is on.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `window_shadows`             | bool                              | `false`                               | Draw drop shadows behind windows.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `natural_scroll`             | bool                              | `true`                                | "Natural" wheel/touchpad scroll: down goes back into history. Set `false` for traditional. Does not affect Shift+PageUp/PageDown.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `launch_maximized`           | bool                              | `false`                               | Open new app windows maximized (filling the work area).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `auto_tile`                  | bool                              | `false`                               | Auto-arrange all windows into the tiling grid as they open and close.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `grid_rows`                  | int                               | `2`                                   | Tiling grid rows (1–6).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `grid_cols`                  | int                               | `2`                                   | Tiling grid columns (1–6).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `tile_gap`                   | int                               | `0`                                   | Cells of gutter between tiled windows.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `theme`                      | string                            | `"newmoon"`                           | Color theme: a built-in (`newmoon`, `midnight`, `nord`, `gruvbox`, `dracula`, `pastoral`) or the stem of a theme file (see `theme_dir`). A file with a built-in's name overrides it. (`light` is a deprecated alias for `pastoral`.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `theme_dir`                  | string                            | `<config>/themes`                     | Directory of `*.toml` theme files. Relative paths resolve against the config dir; also overridable via `$TETRON_WM_THEMES_DIR`. A theme file is a flat table of palette fields (`desktop_bg`, `window_bg`, `title_focus`, `title_blur`, `title_fg`, `border`, `shadow`, `ctrl_fg`, `close_fg`, `menubar_bg`, `dock_bg`, `text`, `dim`, `accent`, `active_bg`) → `#rrggbb` / `#rrggbbaa` / named color; optional `name` and `base` (built-in to inherit unset fields from). Create one with `tetron-wm theme --dump <name>`; list all names with `tetron-wm theme --list`. Low-contrast themes are warned about in the debug log, never rejected.                                                                                  |
| `icon_style`                 | string                            | `"ascii"`                             | File-type icon glyphs where graphical (Kitty) icons can't render (kmscon, bare VT, SSH): `ascii` (BMP shapes, renders everywhere), `nerd` (Nerd Font glyphs, needs one installed — tetron-os ships it), or `emoji`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `launcher_label`             | string                            | `"tetron"`                            | Text of the menubar launcher button: up to 6 single-width characters (shorter ones are centred, so the bar never shifts), or `@session` to show the session name in a named session.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `terminal_bg`                | string                            | unset (follow theme)                  | Default background inside hosted app windows, independent of the desktop theme: `#rrggbb` or a named color. Unset follows the theme's `window_bg`. Applies live to running apps (no app-server restart) when the app server supports it (protocol v4+). Settings → Terminal edits it with presets, hex entry and a Preview / Save / Revert flow.                                                                                                                                                                                                                                                                                                                                                                                  |
| `terminal_fg`                | string                            | unset (follow theme)                  | Default foreground inside hosted app windows: `#rrggbb` or a named color. Unset follows the theme's `text`. Same live apply and Settings section as `terminal_bg`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `truecolor`                  | bool                              | auto-detect                           | Force 24-bit color on/off. Unset = detect from known truecolor terminals + `COLORTERM`. Set `true` when the terminal supports truecolor but omits `COLORTERM` (common with `TERM=xterm-256color`), else subtle themes quantize to the 256-color cube (nord → teal/navy). `TETRON_WM_TRUECOLOR=1/0` env overrides this.                                                                                                                                                                                                                                                                                                                                                                                                            |
| `wallpaper`                  | string                            | `"wallpaper.jpg"`                     | Desktop wallpaper image, rendered to colored cells via `chafa` (so it works on any terminal incl. a kmscon console) and composited behind the windows. An absolute path, a `~`-path, or a path relative to the config dir (a `wallpapers/` subdir is checked first, then the config root) — so a bare filename means "drop it in `~/.config/tetron-wm/wallpapers/`". If the default `wallpaper.jpg` is not on disk, a built-in default image (70 KB, stamped to `~/.local/share/tetron-wm/default-wallpaper.jpg`) is used; any other missing file, or a missing `chafa` → solid desktop (logged). `TETRON_WM_WALLPAPER=<path>` env overrides this.                                                                                |
| `wallpaper_enabled`          | bool                              | `true`                                | Master on/off for the wallpaper. When `false`, the desktop is solid even if `wallpaper` names an image. (Settings → Appearance toggles this.)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `default_project_dir`        | string                            | home (`~`)                            | Directory the working-directory picker opens at.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `show_hidden_dirs`           | bool                              | `false`                               | Show hidden (dot) directories in the picker by default.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `filemanager_view`           | string                            | `list`                                | File-manager view: `icon` or `list`. List is the default; this override key round-trips but is not yet applied at startup.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `recent_dirs`                | list of string                    | `[]`                                  | Legacy. Recently used working directories now live in `~/.local/share/tetron-wm/state.toml`. An old value here is migrated once and then removed from `config.toml`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `desktop_enabled`            | bool                              | `true`                                | Show icons on the wallpaper/desktop.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `desktop_scrim`              | string                            | unset (semi-transparent black)        | Backing drawn behind desktop icon glyphs and labels **when a wallpaper is shown**, for legibility. A color (`#rrggbb` / `#rrggbbaa` / named); `"none"` or `"off"` removes it so labels sit directly on the wallpaper (white ink). Unset = `#000000c8`. No effect on the solid desktop. (Settings → Appearance → "Icon label scrim" toggles it on/off.)                                                                                                                                                                                                                                                                                                                                                                            |
| `desktop_pins`               | list of table                     | none                                  | Pinned desktop shortcuts (see "App entries"); otherwise the desktop shows only real ~/Desktop contents.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `desktop_positions`          | map                               | `{}`                                  | Legacy. Saved desktop icon positions now live in `~/.local/share/tetron-wm/state.toml`. An old value here is migrated once and then removed from `config.toml`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `assistant_command`          | string                            | `opencode`                            | Agent CLI the ✦ assistant panel runs.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `assistant_args`             | list of string                    | `[]`                                  | Extra arguments passed to the assistant CLI.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `assistant_mode`             | string                            | `"panel"`                             | How the assistant opens: `panel` (right-docked) or `window` (floating).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `update_branch`              | string                            | `"main"`                              | In-app updater channel: `main` (stable prebuilt) or `dev`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `apps`                       | list of table                     | `[]`                                  | Apps auto-started at launch and shown in the taskbar (see "App entries").                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `launcher`                   | list of table                     | `[]`                                  | Apps offered in the launcher/spotlight; falls back to `apps` when empty.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `default_apps`               | map string→string                 | built-in                              | File-role → handler command map, for "open with".                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `dock_badges`                | map string→string                 | claude=orange, kilo=yellow            | Per-app badge colors (shown on the taskbar pill): a keyword (case-insensitive substring of the app name or command) mapped to a named color or `#rrggbb`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `screenshot_dir`             | string                            | `""`                                  | Directory for screenshots taken with the leader `s` key or launcher → tetron-wm → Screenshot. Empty uses `~/Pictures` if it exists, else `$HOME`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `web_home`                   | string                            | `"https://lite.duckduckgo.com/lite/"` | Start page for text web browsers launched with no URL (w3m exits at once without one). Empty launches them bare.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `screensaver`                | string                            | `"black"`                             | Effect used when the screensaver starts without one named: `black` or `clock` (`tetron-wm screensaver list`). On demand only (leader `z` or `tetron-wm screensaver`), there is no idle timer.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `power_menu`                 | bool                              | `true`                                | Show launcher → Power (Suspend / Reboot / Power off). `false` removes it and refuses the actions everywhere, for machines that must stay up (remote servers).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `power_commands`             | table of action → command or list | built-in chain                        | `[power_commands]` with `suspend`, `reboot`, `poweroff`. A value is a command line or a list tried in order until one succeeds; it replaces the built-in chain (systemd, elogind, sudo/doas, busybox). An empty list hides the action.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `leader`                     | string                            | `"ctrl+space"`                        | Leader chord, `ctrl+<key>`. Needs `tetron-wm reload`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `keys`                       | table of action → key or list     | built-in                              | `[keys]` leader-key overrides per action, e.g. `launcher = "l"` or `launcher = ["l", "enter"]`. Setting an action replaces its default keys. Every action and its defaults are in the "Leader keys" table below. An entry with no valid key is ignored and logged.                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `clipboard_history`          | bool                              | `true`                                | Keep a history of everything copied (OSC 52 stores, `Ctrl+Shift+C`, mouse selections, the `clip` CLI). Browse it from the menubar clipboard segment.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `clipboard_persist`          | bool                              | `true`                                | Save the history to `~/.local/share/tetron-wm/clipboard-history.jsonl` (mode 0600). `false` keeps it in memory only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `clipboard_record_selection` | bool                              | `true`                                | Record mouse text selections in the history, not just explicit copies.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `clipboard_history_size`     | int                               | `100`                                 | Maximum unpinned history entries.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `clipboard_item_max_kb`      | int                               | `256`                                 | Entries larger than this many KB are not recorded.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `clipboard_ttl_secs`         | int                               | `0`                                   | Expire unpinned entries after this many seconds; `0` never expires.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `clipboard_ignore_apps`      | list of string                    | `[]`                                  | Do not record copies made while an app whose name or command contains one of these (case-insensitive) is focused, e.g. password managers.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `clipboard_shims`            | string                            | `"auto"`                              | Put `xclip`/`xsel`/`wl-copy`/`wl-paste` shims on the `PATH` of launched apps so they use this clipboard: `auto` (only when there is no display), `on`, `off`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `clipboard_fake_display`     | bool                              | `false`                               | Give launched apps a dummy `WAYLAND_DISPLAY` when there is no display, so tools that only look for a clipboard helper when a display variable is set (Neovim) use the shims. Programs that test for a desktop session (`xdg-open`, GUI toolkits) then think one exists, so it is off by default.                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `clipboard_gui_bridge`       | string                            | `"auto"`                              | Mirror the real X11/Wayland clipboard into the history: `auto` (when a display exists), `on`, `off`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `clipboard_osc52_read`       | string                            | `"focused"`                           | Who may READ the clipboard with an OSC 52 query: `focused` (only the focused window), `always`, `never`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `tray_placement`             | table of segment → string         | see meaning                           | `[tray_placement]`: where each status segment lives, by key `cpu`, `mem`, `battery`, `bluetooth`, `wifi`, `tetron`, `bell`: `bar` (always on the bar), `info` (in the ⓘ menu) or `auto` (on the bar, overflowing into the ⓘ menu when space runs out). Defaults: `cpu`, `mem`, `bluetooth` = `info`, the rest `auto`. A CPU or memory reading of 90% or more shows on the bar anyway.                                                                                                                                                                                                                                                                                                                                             |
| `tray_tools`                 | map string→string or list         | built-in                              | Command for the tray popovers' `▸ Open …` row, per item: `cpu`, `mem`, `battery`, `bluetooth`, `network`. A value is one command line or an ordered list; the first whose program is on `$PATH` wins. Prefix a command with `hold:` to run it through `sh -c` and keep the window open until Enter (for one-shot output). An empty list removes the row. Defaults: cpu `htop`, else `top` (even if not detected); mem `hold:free -h`, `hold:vm_stat`; battery `jolt`, `hold:upower -i $(upower -e \| grep -m1 -i bat)`, `hold:pmset -g batt`; bluetooth `bluetuith`, `bluetoothctl`, `hold:blueutil --paired`; network `wlctl`, `nmtui`, `iwctl`, `hold:ip -br addr`, `hold:networksetup -listallhardwareports`, `hold:ifconfig`. |

## App entries

`apps`, `launcher`, and `desktop_pins` are arrays of tables. Each entry's fields:

| Field          | Type           | Meaning                                                                   |
| -------------- | -------------- | ------------------------------------------------------------------------- |
| `name`         | string         | Label shown in the titlebar and taskbar.                                  |
| `command`      | string         | Executable to run (or a `@builtin` like `@files`, `@store`).              |
| `args`         | list of string | Arguments for the command.                                                |
| `category`     | string         | Grouping shown in the launcher menu.                                      |
| `requires_cwd` | bool           | Prompt for a working directory before launching.                          |
| `cwd`          | string         | Fixed working directory to start in.                                      |
| `cli`          | bool           | Wrap a CLI tool so its window does not exit instantly (gives it a shell). |
| `warn`         | string         | Confirmation text shown before launching (for destructive tools).         |

Example:

```toml
## Add to config.toml
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

## Leader keys

Press the leader (`ctrl+space`), release it, then the key. Rebind any action under `[keys]`; setting one replaces its default keys. A key is a single character or `space`, `enter`, `tab`, `esc`, `left`, `right`, `up`, `down`, `backspace`.

| Action        | Default keys | Does                                                                   |
| ------------- | ------------ | ---------------------------------------------------------------------- |
| `spotlight`   | `space`      | spotlight search                                                       |
| `launcher`    | `l`, `enter` | app menu (launcher)                                                    |
| `maximize`    | `m`, `M`     | maximize focused window                                                |
| `minimize`    | `n`, `N`     | minimize focused window                                                |
| `close`       | `esc`        | close focused window (asks first for app windows)                      |
| `snap_left`   | `[`, `left`  | snap left half                                                         |
| `snap_right`  | `]`, `right` | snap right half                                                        |
| `store`       | `S`          | app store                                                              |
| `settings`    | ``, ``       | settings                                                               |
| `activity`    | `A`          | activity monitor                                                       |
| `clipboard`   | `C`          | clipboard history                                                      |
| `select`      | `x`, `y`     | select mode: drag selects text even in mouse-capturing apps (one-shot) |
| `copy`        | `c`          | copy the mouse selection to the clipboard (Ctrl+Shift+C)               |
| `paste`       | `v`, `p`     | paste the clipboard into the focused app (Ctrl+Shift+V)                |
| `help`        | `?`, `h`     | shortcut help                                                          |
| `rename`      | `r`, `R`     | rename focused window                                                  |
| `tile`        | `t`          | tile all windows into the grid                                         |
| `auto_tile`   | `T`          | toggle auto-tile                                                       |
| `screensaver` | `z`          | start the screensaver (any key or mouse wakes it)                      |
| `screenshot`  | `s`          | save a screenshot (PNG) of the whole screen                            |
| `detach`      | `q`          | detach (apps keep running)                                             |
| `shutdown`    | `Q`          | quit tetron-wm (closes every app)                                      |

## Recipes

Copy-paste scenarios. `##` lines explain, `#>` lines are commands to run in a shell, `#$` lines are settings for `config.toml`. Run `tetron-wm --help` for every command.

### Sessions

```
## Start (or attach to) the default session.
tetron-wm
## List sessions: attached | detached | recoverable | stale.
tetron-wm ls
## Attach to a detached session (never starts one).
tetron-wm attach example-session-name
## Take over a session another terminal or ssh login is attached to.
tetron-wm attach -d
## A separate workspace side by side with the default one.
tetron-wm -s work
## Any command can target a session.
tetron-wm -s work launch htop
## Show the session name on the launcher button.
launcher_label = "@session"
## Leave with apps running: leader q, or launcher > Detach. Stop everything:
tetron-wm kill
```

### After editing config.toml

```
## Restart the UI only; apps keep running.
tetron-wm reload
## terminal_bg / terminal_fg and clipboard shims are read by the app host: stop everything and relaunch.
tetron-wm kill
tetron-wm
```

### Themes and colors

```
## List built-in and file themes.
tetron-wm theme --list
## Switch the theme.
tetron-wm theme nord
## Write a built-in theme to ~/.config/tetron-wm/themes/nord.toml to edit, then keep theme = "nord".
tetron-wm theme --dump nord
## Light desktop, dark terminals.
theme = "pastoral"
terminal_bg = "#000000"
terminal_fg = "#e6e6e6"
## Washed-out nord/dracula over ssh or a terminal that hides truecolor support.
truecolor = true

## Same, for one run only.
TETRON_WM_TRUECOLOR=1 tetron-wm
```

### Wallpaper and desktop

```
## Needs chafa. Drop an image in, then reference it by name.
mkdir -p ~/.config/tetron-wm/wallpapers && cp photo.jpg ~/.config/tetron-wm/wallpapers/
wallpaper = "photo.jpg"
## Solid desktop instead.
wallpaper_enabled = false
## Icon labels straight on the image, no backdrop.
desktop_scrim = "none"
## Try a wallpaper without editing the config.
TETRON_WM_WALLPAPER=~/Pictures/other.png tetron-wm reload
```

### Windows and tiling

```
## Add to config.toml
## Three columns on an ultrawide, auto-arranged, with a one-cell gutter.
auto_tile = true
grid_rows = 1
grid_cols = 3
tile_gap = 1

## Tile everything now.
tetron-wm tile

## Snap windows to screen edges when dragged.
snapping_enabled = true
## Open every app maximized.
launch_maximized = true
```

### Apps, launcher and desktop

```
## Open an app in the running desktop.
tetron-wm launch htop
tetron-wm launch nvim ~/code/notes.md

## Add to config.toml

## Start a shell at every launch.
[[apps]]
name = "Shell"
command = "bash"

## A project editor that asks for its directory, and a one-shot CLI whose output stays visible.
[[launcher]]
name = "Neovim in project"
command = "nvim"
requires_cwd = true

[[launcher]]
name = "Disk usage"
command = "df"
args = ["-h"]
cli = true

## A destructive entry asks first.
[[launcher]]
name = "Wipe cache"
command = "rm"
args = ["-rf", "/home/you/.cache/thing"]
warn = "This deletes the cache. Continue?"

## Pin the file manager on the desktop.
[[desktop_pins]]
name = "Files"
command = "@files"

## Open files with these programs (this table replaces the built-in map: list every role you use).
[default_apps]
text = "hx"
image = "@image"
directory = "@navigate"
video = "mpv"
```

### Keys

```
## Change the leader chord (needs reload). Avoid keys terminals use (ctrl+c, ctrl+d, ctrl+z).
leader = "ctrl+b"
## Launcher on "a" only, screensaver on "Z".
[keys]
launcher = "a"
screensaver = "Z"
```

### Clipboard

```
## Keep history in memory only, never on disk.
clipboard_persist = false
## Do not record while a password manager is focused.
clipboard_ignore_apps = ["keepassxc", "bitwarden"]
## Expire unpinned entries after a day.
clipboard_ttl_secs = 86400
## Neovim over ssh with no display: use the shims.
clipboard_shims = "on"
clipboard_fake_display = true
```

```
## Browse and manage the history from the shell.
tetron-wm clip list
tetron-wm clip pin 3
tetron-wm clip copy "some text"
```

### Remote server that must stay up

```
## No Suspend / Reboot / Power off anywhere.
power_menu = false
## Or change how they run, trying each command in order.
[power_commands]
poweroff = ["doas poweroff", "sudo poweroff"]
Hide Suspend.
suspend = []
```

### Status bar

```
## Add to config.toml
## Put battery and wifi always on the bar, CPU in the (i) menu.
[tray_placement]
battery = "bar"
wifi = "bar"
cpu = "info"

## Prefer btop for the CPU popover, remove the memory row.
[tray_tools]
cpu = ["btop", "htop"]
mem = []
```

### Screenshots, screensaver, web

```
## Save a PNG of the whole screen (needs an attached client).
tetron-wm screenshot ~/Pictures/desk.png
#$ screenshot_dir = "/home/you/shots"
## Cover the screen now; any key wakes it.
tetron-wm screensaver on clock
tetron-wm screensaver list
#$ screensaver = "clock"
## Text browsers launched with no URL open this page.
#$ web_home = "https://wiki.archlinux.org"
```

### Updates and the assistant

```
## Add to config.toml
## Track the dev channel (built from source).
update_branch = "dev"
## Use hermes instead of opencode, floating instead of docked.
assistant_command = "hermes"
assistant_mode = "window"
## Inspect what happened: the in-app Logs viewer, or
tail -n 50 ~/tetron-wm-debug.log
```

## Notes

- Changes take effect on the next tetron-wm launch, or after `tetron-wm reload` (Settings can change a subset live).
- `config.toml` is yours. Saving from the app (a Settings change, a pin, the theme) rewrites only the keys that changed: your comments, key order, commented-out blocks and unknown keys stay, and a hand edit to any other key is not reverted even while tetron-wm is running (run `tetron-wm reload` to make it take effect). Nothing is written when nothing changed. Two limits: a comment written directly above a key goes with that key if the key is removed, and when a whole `[[launcher]]`, `[[apps]]` or `[[desktop_pins]]` list changes, comments inside that list are not kept.
- The first time the file is rewritten in a session the old copy is saved as `config.toml.bak`. A file with a syntax error is never overwritten: tetron-wm logs the error (see the Logs viewer), runs on defaults, and saves again once the file parses.
- App-managed data is not in `config.toml`: desktop icon positions and recent working directories live in `~/.local/share/tetron-wm/state.toml` (`$XDG_DATA_HOME` aware). Old `recent_dirs` and `desktop_positions` keys are moved there once and removed from `config.toml`.
- The live Settings UI exposes snapping, snap threshold, grid rows/cols, gap, auto-tile, launch-maximized, natural scroll, shadows, theme, wallpaper on/off, wallpaper file, icon-label scrim, icon style, desktop icons, clipboard (including entry expiry and max item size), terminal colors (presets, hex, preview), assistant, and updates; the remaining keys are file-only or app-managed.
