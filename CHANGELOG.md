# Changelog

All notable changes to tetron-wm are recorded here. The project uses
[semantic versioning](https://semver.org); while pre-1.0, minor versions may
carry user-visible feature work and the occasional breaking config change.

## [Unreleased]

### Added
- **One-shot suite installer** (`contrib/install-tetron-wm-suite.sh`, with a
  TLDR in the README). A single prompted `curl | bash` that optionally installs
  tetron-wm, seeds `~/.config/tetron-wm/` (config.toml + themes/ + wallpapers/),
  installs kmscon, and sets up tty1 autologin into the desktop. Every step is
  opt-in; the tty1 step is off by default, confirmed, and prints how to undo it.
  `-y` takes the defaults; `--all -y` builds the full console appliance.

### Fixed
- **Suite installer no longer aborts the kmscon step on an Enter-through run.**
  A "yes" to the suite's "Install kmscon?" prompt now carries through to the
  kmscon installer (passes `-y`) instead of hitting a second, No-default
  "Proceed with the source build?" prompt that an Enter would decline. The
  kmscon script now also logs that it is building from source under `-y`
  instead of proceeding silently.
- **kmscon source build no longer dies on Debian bookworm / LMDE 6's old
  meson.** The maintained fork needs meson >= 1.1 but those ship 1.0.1;
  `install-kmscon.sh` now detects a too-old meson and bootstraps a newer one
  into a throwaway venv for the build (ninja from apt is new enough).
- **tty1 launch snippet now uses the binary's absolute path and prepends
  `~/.local/bin` to PATH.** The login profile runs before the interactive rc,
  so a bare `tetron-wm` could be missing from PATH at login and drop you to a
  shell instead of the desktop.

## [0.14.4] — 2026-10-08

### Fixed
- **Settings → Appearance no longer freezes when editing the wallpaper
  filename.** With the Wallpaper-file text box open, mouse clicks were swallowed
  and every row went dead, so a mouse-driven user had no way to close it and the
  panel looked frozen. A click now dismisses the box (committing the typed
  filename, or cancelling the Apps add-form), just like clicking away.
- **A wedged `chafa` can no longer hang the whole UI.** The image-wallpaper
  render shelled out to `chafa` with inherited stdin and no timeout, on the
  daemon's input/render loop; now it runs with stdin closed, its output drained
  on a thread, and an 8-second hard timeout (then killed, with a log line).

### Added
- **Desktop icon-label scrim is configurable.** The dark backing drawn behind
  desktop icon glyphs and labels over a wallpaper is now set by `desktop_scrim`
  in `config.toml` (a color, or `"none"`/`"off"` to turn it off so labels sit
  directly on the wallpaper); unset keeps the default semi-transparent black.
  **Settings → Appearance** gains an **Icon label scrim** on/off toggle. Only
  applies over a wallpaper; the solid desktop is unchanged.

## [0.14.3] — 2026-10-07

### Added
- **Image wallpaper, configurable (v1).** Set `wallpaper = "<path>"` in
  `config.toml` to put an image behind your windows. The path may be absolute,
  `~`-relative, or relative to the config dir (a `wallpapers/` subdir is checked
  first, then the config root), so a bare filename means "next to your config".
  The image is rendered to colored cells via `chafa`, so it works on any
  terminal including a bare kmscon console. **Settings → Appearance** gains a
  **Wallpaper** on/off toggle (`wallpaper_enabled`) and a **Wallpaper file** text
  box: drop an image in `~/.config/tetron-wm/wallpapers/` and type its filename
  (the panel shows that path as a reminder). If `chafa` is missing or the image
  can not be read, the desktop stays solid and the reason is logged. The
  `TETRON_WM_WALLPAPER` env var still overrides the config for quick testing.
  `install.sh` now installs `chafa` as an optional dependency.
- **`install.sh` installs a Symbols Nerd Font** (the glyphs `icon_style = "nerd"`
  draws) as an optional dependency: a brew cask on macOS, and on Linux the
  "Symbols Only" release dropped into `~/.local/share/fonts` (no sudo) so
  fontconfig uses it as a glyph fallback in kmscon and most terminals.

## [0.14.2] — 2026-10-06

### Changed
- **Open windows now live in a top-panel taskbar; the bottom dock is gone.** The
  menubar row carries the taskbar of open windows (same-app grouping, a
  drop-down group chooser, and a colored letter badge per app), with a `+`
  new-shell button just right of the ✦ assistant. Pills show full labels, then
  shrink to badge-only, then collapse to a trailing `…+N` overflow marker under
  width pressure so they never collide with the tray (taskbar scroll is a
  planned follow-up). Retiring the bottom row gives every window one more line
  of screen. Right-click a pill for its context menu (minimise / maximise /
  close / reset size), which now drops down from the bar.
- **Power actions moved into the launcher.** The top-right host-name button is
  gone; the launcher's bottom section now holds **Systems**, **Exit**, **Restart**,
  **Shutdown** (Systems opens a top-left popover; the others open their confirm
  dialog). Reuses the existing power/systems flows.
- **Volume sits in the far-right corner** of the menubar (past the clock), flush
  to the screen edge — a fixed, eyes-free spot to scroll to change volume. Its
  level is zero-shift padded so changing digits (5 → 100) never nudges the other
  segments.
- **The menubar clock shows the time only** (`HH:MM`); the date/day live in the
  calendar popover (one click on the clock).
- **Tray drop-priority** sheds CPU → Mem → Battery → Bell → Tetron → Wi-Fi →
  Bluetooth under width pressure; the clock and volume are always kept.
- **Renamed from `tuiui` to `tetron-wm` (breaking).** This is a fork of upstream
  `jaylfc/tuiui`; sharing the name let installs and the in-app updater collide
  with upstream. The binary, crate, config dir (`~/.config/tetron-wm`), state dir,
  log, socket, service unit (`tetron-wm-apphost`), `TETRON_WM_*` env vars, and
  release assets (`tetron-wm-<target>.tar.gz`) are all renamed. Existing installs
  must be reinstalled; config/service paths do not migrate automatically.
- **Menubar launcher button now reads `tetron`**, not the full binary name.
- **The theme now applies to every surface.** Terminals, the launcher, the file
  manager, desktop icon labels, the Store, Settings, and the Activity Monitor all
  read the active theme instead of a hardcoded midnight palette, so theme switches
  (and the pastoral theme) are no longer skin-deep.
- **Finer window borders** — a thin green line instead of a wide white band.
- **No default desktop pins** — the desktop shows only the real contents of
  `~/Desktop`; add pins via `[[desktop_pins]]`.
- **The file manager opens in list view by default** (was the icon grid). The
  grid view's fallback glyph now sits just above the label instead of floating
  high in the tile.

### Added
- **Scroll the wheel over the Volume tray segment to change the system volume**
  (up = louder, ±5 per notch), instead of it falling through to window scrollback.
  Works regardless of which window is focused (the menubar row is treated as
  chrome, so a focused Store/Files/Logs no longer swallows the wheel there).
- **Choose truecolor vs 256-color** — a `truecolor` config key (and
  `TETRON_WM_TRUECOLOR=1/0` env override) forces 24-bit color on or off. Auto-
  detection only enabled truecolor for known terminals or when `COLORTERM`
  advertised it, so terminals that do truecolor but set `TERM=xterm-256color`
  without `COLORTERM` fell back to the 256-color palette — which quantized the
  subtle themes (nord → teal/navy, dracula) into the wrong colors. Env wins over
  the config key, both win over auto-detection and the graphics probe.
- **File-based themes.** Drop a `*.toml` in `~/.config/tetron-wm/themes/` (or set
  `theme_dir`) and it appears everywhere a built-in does — Settings → Appearance,
  the power-menu remote form, `tetron-wm theme <name>`. A file is a flat table of
  the palette fields → `#rrggbb` / `#rrggbbaa` (alpha) / named color, with optional
  `name` and `base` (a built-in to inherit unset fields from); a file named after a
  built-in overrides it. `tetron-wm theme --dump <name>` writes a built-in out as an
  editable starter; `--list` prints every theme name. Malformed files are skipped
  and logged; low-contrast themes are warned about (never rejected) so a user theme
  cannot silently reproduce the red-on-green button bug.
- **Pastoral theme** — a fifth, and first light-appearance, palette
  (`theme = "pastoral"`, or cycle it in Settings → Appearance): paper wallpaper,
  black-on-white terminals, light panels, tetron-green highlights. (Named for the
  palette, not its brightness, so the name does not clash with a theme's
  light/dark `appearance`; `theme = "light"` still works as a deprecated alias.)
- **`terminal_bg` / `terminal_fg` config keys** — decouple terminal colors from the
  desktop theme (hex or named color); unset = follow the theme.
- **`icon_style` config key** — picks the file-type icon glyphs used where the
  graphical (Kitty) icons can't render (kmscon, a bare VT, plain SSH): `ascii`
  (default; BMP geometric shapes that render in every font and take the theme
  color), `nerd` (Nerd Font glyphs — needs a Nerd Font installed), or `emoji`.
  Previously the file manager and desktop always used emoji, which showed as
  tofu on a console.

### Fixed
- **The menubar tray glyphs now follow `icon_style`.** Volume, battery, and the
  bell used hardcoded color emoji (🔇/🔉/🔊, ⚡/🔋, 🔔) regardless of the icon
  style, so they clashed with the ascii/nerd file icons (and showed as tofu on a
  console). All tray indicators (CPU, memory, battery, volume, bluetooth, network,
  bell) now resolve through the same `icon_style` set — BMP shapes under `ascii`,
  Nerd Font glyphs under `nerd`, emoji under `emoji`.
- **Desktop icon glyph sits just above its label** — the no-graphics fallback
  glyph was stranded high in the tile (centered in the image area); it now sits at
  the bottom of the icon rect like the file manager's grid view.
- **Dragging a mouse-reporting app's window (e.g. tetron-tui) works now.** The
  client routed each mouse event by the pointer's position in the *last frame's*
  app content rect, so dragging the titlebar downward "fell into" the app as the
  cursor crossed that stale rect before the window caught up — the window stuck,
  stalled, or only dragged upward (a plain shell window, which reports no mouse
  area, was unaffected). The client now takes a pointer grab on the button press:
  the whole drag goes to whatever the press landed on (chrome or app) until
  release, regardless of where the pointer wanders.
- **Window dragging is smoother in general.** Two more drag glitches fixed:
  (1) a window could stay "stuck" to the pointer after release — a lost release
  event left the drag active, and a no-button move now ends it (self-heal, as text
  selection already did); (2) fast vertical drags froze or dragged only one
  direction — the spurious-teleport guard dropped *any* half-screen step, now it
  only drops a big jump that lands on a screen edge (the real off-screen-fling
  signature), so fast mid-screen drags pass through.
- **Confirm-dialog buttons are readable now.** The destructive confirms (Close,
  Shut Down, Remove, the dangerous-launch Launch, Restart) drew red text on the
  green accent -- an unreadable red-on-green, especially on the pastoral (light) theme. They
  now use light text on a red fill (a proper destructive button); the benign
  Connect button and the calendar's "today" cell use light text on the accent.
- **Window rename triggers only on the title text now**, not anywhere on the
  titlebar, and clears any in-progress move/drag when it starts — so a
  double-click on empty titlebar space no longer accidentally starts a rename or
  leaves a stray, mispositioned titlebar behind.
- **Install/update now pull from the fork, not upstream.** `install.sh`, the in-app
  updater, and the Systems remote-installer pointed at `jaylfc/tuiui`, so a prebuilt
  install or a Settings → Updates run fetched upstream and dropped the tetron
  integration. All now target `ErikAllanKincaid/tetron-wm`.

## [0.14.1] — 2026-10-05

### Fixed
- **Tray "Open terminal" now works when a window sits under the popover.** The
  popover opened the native `tetron-tui` only when nothing was behind it; over
  an app window the click was swallowed as a text-selection on the window
  beneath and nothing launched. `mouse_select` runs before the tray router on
  `MouseDown` but did not treat an open tray popover as an overlay, so it
  claimed the click. The tray popover now counts as an overlay like every other
  modal menu.

## [0.14.0] — 2026-10-05

### Added
- `tetron-wm --version` (and `-V`) prints the version.

### Changed
- **Version scheme now tracks tetron core's minor.** Addons version as
  `0.<tetron-core-minor>.<patch>`; this release follows `tetron-proto 0.14.x`,
  so the jump from `0.2.14` to `0.14.0` is a scheme change, not 12 minors of
  feature work. Patch resets to 0.

## [0.2.14] — 2026-07-20

### Fixed
- **File-manager icons are now real desktop-style tiles.** 0.2.13's FM icons
  rendered as a squashed one-row smear: the Icon view's tiles were still the
  old 3-row (glyph + label) cells, so images were crushed into a single row.
  Tiles are now the desktop's proportions (icon area + centered label), role
  icons and thumbnails fill the tile without stretching, and render /
  hit-test / image placement all share the same geometry helpers so they
  can't drift. Glyph fallback matches the new layout.
- **The FM context menu is now a real right-click menu.** It used to open as
  a centered "Actions" dialog with one horizontal text line. It's now a
  compact vertical menu (Open / Rename / Copy / Cut / Delete / Get Info)
  anchored at the click (or the focused tile via keyboard), with clickable
  rows, hover/arrow-key highlight, click-outside-to-dismiss — and Esc now
  closes the menu instead of the whole Files window.

## [0.2.13] — 2026-07-20

### Added
- **File-manager icons now match the desktop.** The FM's Icon view renders
  the same role-icon image tiles as desktop icons (folders, text, code,
  archives, …) on Kitty-graphics terminals, instead of text glyphs; image
  files keep their real thumbnails. Glyph fallback elsewhere is unchanged.
- **Right-click in the file manager.** Right-clicking an entry raises the
  window, selects the entry, and opens the context menu (open / rename /
  delete / copy / …) — previously the menu was keyboard-only.
- **App variants in the catalog.** A catalog entry can declare launcher
  variants (extra entries emitted when the app is installed). First user:
  Claude Code ships a **⚠️ variant** running `--dangerously-skip-permissions`.
- **Launch-warning dialog.** Launcher entries can carry a `warn` message
  (the Claude ⚠️ variant does; your own `[[launcher]]` config entries can
  too): launching one opens a confirm dialog before anything spawns —
  Enter/`y` launches, Esc/`n` cancels.

### Fixed
- **The desktop now notices external changes to `~/Desktop`.** A folder or
  file created in a terminal or the file manager appears on the desktop
  within ~2 seconds (throttled mtime poll), instead of only after a reload.

## [0.2.12] — 2026-07-19

### Fixed
- **`tetron-wm launch <cli-tool>` no longer opens a window that instantly dies.**
  The v0.2.11 help-then-shell wrapper for catalog-tagged CLI tools only
  applied to the launcher menu / Spotlight / Store paths; the `tetron-wm launch`
  escape hatch (used by the AI assistant and scripts) still spawned the bare
  binary, which printed usage and exited. A bare `tetron-wm launch gum` now gets
  the same wrapper; passing args (`tetron-wm launch gum choose a b`) runs the
  command exactly as given.

## [0.2.11] — 2026-07-04

### Added
- **CLI tools get a proper launch path.** Some catalog apps are CLI tools, not
  persistent TUIs (himalaya, gum, freeze, khal, dust, …) — launching one used
  to open a window that printed an error or usage text and immediately died.
  A one-time audit flagged **52** such entries with a new `"cli": true` catalog
  field; they now show a **`CLI` tag** in the launcher (menu + Spotlight) and
  the Store, and launching one opens a shell that prints the tool's `--help`
  first, then drops to your interactive shell with the tool on `$PATH` — see
  the commands, then use them. `requires_cwd` still applies (the shell starts
  in the picked directory), and user config can set `cli = true` on custom
  entries too.

### Fixed
- **Activity monitor now shows the command/args of local app sessions.** An
  `Any`-downcast through `Box<dyn AppHost>` could never match the concrete
  host type, so the cmd column was always blank; it now uses the `AppHost`
  trait's own `launch_cmd`.

## [0.2.10] — 2026-07-04

### Fixed
- **Installing and self-updating no longer break when GitHub rate-limits the
  API.** `install.sh` resolved the latest release through the unauthenticated
  `api.github.com` REST endpoint, which is capped at 60 requests/hour per IP.
  Once that budget is spent GitHub answers **403**, and the script reported it
  as `no published release yet` before falling back to a slow `cargo install`
  source build — so `curl | sh` appeared to say the project had no releases,
  and the in-app **Update & Reload** silently dropped into a multi-minute build
  that looked hung (the long-standing "update from Settings gets stuck"
  report). Both now resolve the tag from the web redirect
  `github.com/OWNER/REPO/releases/latest` → `.../releases/tag/vX.Y.Z`, which
  isn't subject to the API rate limit, and only fall back to the REST API if
  that redirect can't be parsed. Settings' "check for updates" uses the same
  path, so a spent API budget no longer shows a false "Couldn't check
  (offline?)".

## [0.2.9] — 2026-06-13

### Changed
- **The debug log now survives a reload, and records the version + binary** of
  every daemon start. `dbg_init` previously *truncated* `~/tetron-wm-debug.log` on
  each daemon startup — so an in-app update that reloaded the daemon wiped its
  own trace, leaving the log useless for diagnosing update failures. It now
  appends a richer banner (`v<version>, git <sha>, exe <path>`) and the reload
  → respawn seam is logged on both sides (`daemon: reload — exiting…`,
  `client: daemon reload — …respawning`, `daemon: spawning <exe> --daemon`).
  An update attempt now leaves a complete, persistent trace: the install
  steps, the reload, and the **post-update version** in the next banner — so a
  "version never changed" failure is finally visible in one log.

## [0.2.8] — 2026-06-13

### Added
- **Switchable assistant agent**: Settings → Assistant now has an **Agent** row
  that flips the ✦ panel between the two supported CLIs — **opencode** (default)
  and **hermes**. The choice is stored in `assistant_command`, so an existing
  config that points at one of them is reflected in the switch (and a
  hand-edited `assistant_command` can still name any binary).

## [0.2.7] — 2026-06-13

### Fixed
- **In-app update could "succeed" without changing the running version**: the
  updater reloads by running a bare `tetron-wm reload`, but it runs in a
  non-interactive `sh -lc` whose PATH may not include the install dir
  (`~/.local/bin` is added by interactive shell config, not a login `sh`). When
  `tetron-wm` wasn't found, `install.sh` still wrote the new binary but the daemon
  never restarted onto it — so the version never moved and the update appeared
  to fail every time. The updater now reloads via the **absolute path** of the
  freshly-installed binary (`{install_dir}/tetron-wm reload`), removing the PATH
  dependency.

### Changed
- **The updater now logs to `~/tetron-wm-debug.log`**: the in-app update runs in a
  window whose output was otherwise lost. Update checks, the install request,
  and each install step (install.sh / cargo fallback / reload / failure) now
  leave breadcrumbs in the debug log, so a failed update is visible in the log
  users paste.

## [0.2.6] — 2026-06-13

### Added
- **Dock right-click context menu**: right-click a dock pill for Minimise /
  Maximise / Close / Reset size. Grouped pills target the group's focused
  window (else the first); Close routes through the usual confirm dialog;
  Reset re-centres the window at half the work area — the rescue hatch for a
  stranded or mis-sized window. Render and hit-testing share the same geometry
  fns, and the menu y-clamps so it stays on-screen (and clickable) on very
  short terminals. Ported from the `dev` branch (#11, hardened in #20).

### Changed
- **Activity-monitor / apphost follow-ups**: `tetron-wm ps` and `tetron-wm kill-app`
  now drain the on-connect roster (and any queued events) before matching the
  `AppList` reply, via a shared `fetch_app_list()` helper; `kill-app all` is
  safe-by-default (only reaps already-dead apps — live apps need an explicit
  id). The apphost protocol gains an optional `pid` field on
  `HostEvt::Spawned` (PROTO_VERSION → 2, additive, `MIN_COMPAT` unchanged) so
  the daemon's `RemoteAppHost` can fill the activity panel's `pid` column, and
  it now stashes each spawn's command + args so the panel shows real data in
  normal daemon mode. Re-landed from the stale `#16` branch onto current
  `main` (keeping main's newer visibility-gated `refresh_activity`).

## [0.2.5] — 2026-06-13

### Fixed
- **Update loop on the `main` channel**: the v0.2.4 release binaries were built
  from the pre-bump commit and reported their version as `0.2.3`, so the in-app
  updater saw a perpetual "update available" (`0.2.3 → 0.2.4`) that installing
  could never settle. Re-released as 0.2.5 (identical code) built from the
  correct commit. The release workflow now fails if the release tag and the
  `Cargo.toml` version disagree, so a version-skewed release can't ship again.

### Changed
- **Releases cut themselves on a version bump**: merging a `Cargo.toml` version
  change to `main` now auto-tags and publishes the release (no tag push
  required), alongside the existing tag-push and manual `workflow_dispatch`
  paths.

## [0.2.4] — 2026-06-13

### Fixed
Robustness fixes from a code review of the networked seams — these had landed
on the `dev` branch but never reached `main`, so they were absent from the
0.2.x releases. Ported to `main`:
- **Log-copy could crash the daemon**: copying a large (>200 KB) log whose
  non-ASCII bytes straddled the tail cut sliced a string mid-UTF-8 and panicked.
  The cut now advances to a char boundary first.
- **Apphost reader resilience**: a single corrupted/blank protocol line used to
  tear down the whole apphost connection (dropping every window's frame stream);
  `recv` now logs-and-skips a bad line while still propagating genuine IO errors.
- **Updater branch validation**: a hand-edited/typo'd `update_branch` flowed
  into a shell command and a URL; it's now sanitized to a git-safe charset and
  falls back to `main`.
- **Control-socket lock resilience**: `apply_ctl` no longer holds the queue lock
  across `core.apply` (which could block the control thread), and both lock
  sites recover a poisoned lock so one panic can't wedge `tetron-wm launch/tile/
  theme/msg`.
- **Remote file-manager freeze bounded**: tightened SSH `ConnectTimeout` (4→3s)
  and the frequent navigation ops (list/home → 5s) so an unreachable saved
  system is a brief hitch, not a multi-second freeze.

## [0.2.3] — 2026-06-13

### Fixed
- **In-app update could land in a shadowed binary**: when the fast path
  (`install.sh`) failed — e.g. a brief network blip or racing a release's
  asset upload — the updater fell back to `cargo install --git`, which writes to
  cargo's default `~/.cargo/bin`. If that differs from the dir of the running
  binary (e.g. a release install in `~/.local/bin`), the rebuilt binary landed
  in a `$PATH`-shadowed location and the update silently appeared to do nothing.
  The cargo fallback now targets the running binary's own `bin/` dir via
  `--root`, so both update paths replace the binary the user actually runs.

## [0.2.2] — 2026-06-13

### Changed
- **Assistant standardised on opencode**: the ✦ assistant now runs a single
  agent CLI — **opencode** (model-agnostic, MCP-extensible) — instead of a
  six-framework menu. The per-framework machinery is gone (Claude Code / Kilo /
  Hermes / OpenClaw / smallcode plumbing, the `CLAUDE.md` / `HERMES.md` /
  `knowledge/*.md` stamping, the smallcode `.env` and OpenClaw workspace pointer,
  and the Settings framework picker); the briefing is stamped once as `AGENTS.md`.
  `assistant_command` in config.toml still overrides the binary, and broader
  OS/computer-use control is added via MCP servers in opencode's own config.
  Migration: if you had `assistant_command` pinned to `hermes` or `openclaw`,
  add their TUI launch arg to `assistant_args` (`["--tui"]` / `["tui"]`) — the
  per-framework default arguments are no longer applied.

### Added
- **Revoke SSH key when forgetting a system**: the ✕ in the Systems menu now
  opens a confirm with an opt-in *"Also revoke this PC's key on &lt;host&gt;"*
  toggle. When checked, removing the system also strips this machine's public
  key from that remote's `~/.ssh/authorized_keys` (the inverse of the key copy
  that adding it performed). Exact full-line match only (never touches other
  keys), across all your local identities; it rewrites the file in place inside
  `~/.ssh` (preserving its `0600` perms and SELinux context) and keeps a
  `.tetron-wm.bak`. Best-effort on a background thread — an offline host never blocks
  the removal or the UI, the local forget always succeeds, and any remote-side
  failure is logged rather than silently assumed done.

### Fixed
- **Activity Monitor refresh while hidden**: the per-frame app snapshot was
  still running when the Activity panel was open but **minimized** (or hidden in
  simple mode), so the window-drag stutter could persist in those states. The
  refresh now also requires the panel to be visible.
- **Update check no longer offers downgrades**: the main-channel version
  comparison was a plain string compare, so a local build *newer* than the
  latest release (mid-release-cut, or a dev/source build) showed a bogus
  "update available → older version". It now compares release version numbers
  (`major.minor.patch`) and only offers a strictly-newer release; an
  unparseable release tag reports "couldn't check" instead of guessing.

## [0.2.1] — 2026-06-13

### Added
- **Activity Monitor** (Ctrl+Space → A, or `@activity` in the launcher): a
  live, auto-refreshing table of every app the apphost is hosting (id, pid,
  command, age, dimensions, state) with kill-app controls and an Enter/y vs
  Esc/n confirm for live apps. Also `tetron-wm ps` and `tetron-wm kill-app <id|all>`
  CLI subcommands that work from any terminal or SSH session.

### Fixed
- **Window dragging stutter**: moving (or resizing) a floating window felt
  sticky and jerky once a few apps were open. The activity monitor's row
  refresh was running every frame even while its panel was closed, and each
  pass cloned every hosted app's full terminal grid just to read its size —
  stalling the render loop mid-drag. The refresh now does nothing unless the
  Activity Monitor window is actually open.
- **Update loop on the main channel**: "Check for updates" compared the
  installed build against the tip of the `main` branch, but the main channel
  installs the latest prebuilt *release*. Any commit landed on `main` after the
  last release showed a permanent "update available" that re-installing the
  same release could never clear. The check now compares the installed version
  against the latest release tag — matching what `install.sh` installs — and
  reports it as versions (`v0.2.0 → v0.2.1`) instead of commit hashes. The dev
  channel still tracks the branch tip and shows short commit hashes.

## [0.2.0] — 2026-06-10

The "desktop, networked" release: switch between machines, an AI copilot, and
a lot of polish.

### Added
- **Systems switcher** (power menu → Systems): saved machines with live ●/○
  reachability dots; **Add Remote** transfers an SSH key, installs tetron-wm + gpm
  + your terminal's terminfo on the remote, syncs your systems list, and
  connects; per-system themes; drop back to the local desktop when the remote
  session ends.
- **AI assistant** (✦ menubar button): a persistent chat panel — or a floating
  window (**Settings → Assistant**) — running any of six agent frameworks
  (Claude Code, opencode, smallcode, Kilo, Hermes, OpenClaw). Instructions live
  in the repo `agent/` pack, stamped into the agent's forced working directory
  in every convention the CLIs read. The agent can drive the desktop
  (`tetron-wm launch/tile/theme/msg`), read the logs, fix tetron-wm and open PRs, and
  operate across your saved machines over ssh/scp.
- **Menubar clock + calendar**: date+time, click for a month calendar with
  month navigation and `khal` events.
- **Notifications**: a background app's bell rings a dock attention dot + a 🔔
  tray popover (click to focus); apps' OSC-52 copies are forwarded to the host
  clipboard.
- **Remote files**: browse saved systems over ssh in the file manager; copy
  between machines with Ctrl+C/Ctrl+V (background `scp`, `-3` for remote↔remote).
- **Logs viewer** (launcher → tetron-wm → Logs): tails `~/tetron-wm-debug.log`; `c`
  copies it to the host clipboard via OSC 52.
- **Scrollback in app windows**: the mouse wheel scrolls a PTY window's history;
  any keystroke snaps back to the live bottom.
- **Battery** tray segment (Linux sysfs / macOS `pmset`).
- **Control CLI**: `tetron-wm launch`, `tile`, `theme`, `msg` drive a running
  desktop from any shell (also the assistant's control surface).
- **Updates channel switcher** (Settings → Updates): track `main` (fast
  prebuilt releases) or a `dev` branch (built from source).
- **Update safety net**: updates never kill running apps today (the app server
  is a separate process the reload doesn't touch, and the wire protocol is
  skew-tolerant by construction) — but if a future update ever *must* break
  that protocol, a safety dialog now appears after the reload: it warns that
  restarting the app server will close the user's N apps, and offers
  **Keep apps** (save your work first; restart later from a row in
  Settings → Updates) or **Restart app server**. Armed by bumping
  `MIN_COMPAT` in `apphost/proto.rs`; dormant otherwise.

### Changed
- The in-app updater now installs the **prebuilt latest release** by default
  (a download, not a source compile), falling back to `cargo install`; it
  reopens Settings on the Updates screen after the reload instead of vanishing.
- Add Remote / reconnect copies the local terminal's terminfo to the remote and
  always falls back to `xterm-256color`, fixing "cannot find terminfo entry for
  'xterm-ghostty'" (and the same for Kitty and other modern terminals).
- Dependencies refreshed to current: crossterm 0.29, portable-pty 0.9,
  sysinfo 0.39, toml 1.1, infer 0.19. Minimum supported Rust is now **1.95**
  (declared via `rust-version` in Cargo.toml).

### Fixed
- Settings' "Check for updates" / "Update & Reload" now respond to **mouse**
  clicks, not only the keyboard.
- The updater window auto-closes after a successful reload.
- Clippy is clean across all targets.

## [0.1.0]

Initial development series: floating windows, persistent daemon/thin client,
app store, settings, theming, menubar tray, grid tiling, native image layer,
file manager, desktop icons, cascading launcher, apphost/frontend split with
live reloads, mouse passthrough, simple view mode, dock grouping, bare-console
(gpm) mouse, apphost service, terminal office suite.
