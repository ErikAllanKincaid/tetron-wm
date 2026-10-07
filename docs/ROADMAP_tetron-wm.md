# tetron-wm roadmap

What is done and what is planned. Shipped items are marked ✅; the remaining
slices are the forward-looking work. User-visible changes are also recorded in
[`../CHANGELOG.md`](../CHANGELOG.md).

## Done

- **✅ Slice 1 — Shell:** compositor, window manager, PTY host, chrome, launcher.
- **✅ Slice 2 — Daemon:** persistent daemon + thin client; detach/reattach keeps windows and processes alive.
- **✅ Slice 3 — Store:** browse/search/install a 100%-verified, OS-aware catalog (incl. an AI tools category).
- **✅ Slice 4 — Settings:** sidebar settings panel writing `config.toml` (Windows, Appearance, Updates, Apps).
- **✅ Slice 5 — Theming:** five live-switchable palettes (incl. pastoral, a light-appearance theme) plus drop-in theme files, from Settings → Appearance.
- **✅ Menubar tray:** clock/CPU/mem/volume/WiFi/Bluetooth/battery with host-control popovers (macOS + Linux backends).
- **✅ Grid tiling:** configurable R×C grid — drag-to-cell, auto-tile, send-to-cell, tile-all.
- **✅ Working-directory picker:** a browsable file-tree on launch for apps flagged `requires_cwd` (the AI CLIs); remembers recent dirs.
- **✅ Native image layer:** Kitty-graphics image rendering inside windows (image viewer; thumbnail engine for the file manager).
- **✅ Default Apps:** configurable file-type → app map (Settings → Default Apps).
- **✅ File manager:** native mouse+keyboard browser — icon/list/**columns** views, navigation, copy/cut/paste, rename, new folder, delete-to-Trash, open-with-default, **image thumbnails**, **tabs**, **preview pane**, and **Get Info** (permissions/symlink).
- **✅ Desktop icons:** clickable wallpaper icons from `~/Desktop` + pins; double-click to open, drag-to-grid (persisted), right-click menu (rename/Trash/new folder), thumbnails.
- **✅ Cascading launcher:** Windows-95-style flyout menu — categories cascade into app submenus on hover/arrow; mouse + keyboard navigable; Exit/Restart/Shutdown/Systems in its bottom section.
- **✅ Top-panel taskbar:** open windows live in the top menubar as badge pills (same-app grouping, drop-down group chooser, badge-only + `…+N` overflow); the bottom dock was retired so windows get the whole screen below the bar.
- **✅ Apphost/frontend split + live updates:** apps run in a separate long-lived process; the UI can **reload (or update) without killing apps** (launcher → Restart, `tetron-wm reload`, Settings → Update & Reload).
- **✅ Mouse passthrough:** full-fidelity mouse (buttons/drag/scroll/modifiers) forwarded into apps that request it, in both views.
- **✅ Simple view mode:** `⊞`/`▦` top-bar toggle between the windowed desktop and a full-screen-single-app view.
- **✅ Window rename + app badges:** same-app windows group into one taskbar pill with a colored letter badge; rename windows (double-click title text / `Ctrl+Space r`).
- **✅ Bare-console mouse (Linux):** native `gpm` support for a mouse on a raw Linux VT (no GUI terminal needed).
- **✅ Apphost as a service:** `tetron-wm service install` runs the apphost as a per-user service (launchd / systemd `--user` / `~/.profile` fallback) that auto-starts on login and restarts on crash; the macOS LaunchAgent also restores Keychain access (e.g. Claude Code login) inside tetron-wm.
- **✅ Smart installs:** the store detects a missing toolchain (Go/Rust/Node/Python) before an install and offers to set it up first; a confirm dialog guards closing an app window (which ends its process).
- **✅ Terminal office suite:** word processor (`wordgrinder`), spreadsheets (`sheets`/`sc-im`/`visidata`), presentations (`slides`/`presenterm`), email (`himalaya`/`aerc`), calendar (`khal`/`calcure`), contacts (`khard`/`abook`), and notes (`nb`/`nap`) — all in the store.
- **✅ Systems switcher:** launcher → Systems — saved machines with live ●/○ dots; Add Remote transfers SSH keys, installs tetron-wm + gpm + your terminal's terminfo on the remote, syncs the systems list, and connects; per-system themes; drop back to the local desktop when the remote session ends.
- **✅ Clock + calendar:** time in the menubar; click for a month calendar with `◂ ▸` navigation and `khal` events.
- **✅ Notifications:** background bell → taskbar attention dot + 🔔 tray popover (click to focus); apps' OSC-52 copies forwarded to the host clipboard.
- **✅ Remote files:** browse saved systems over ssh in the file manager; copy between machines with Ctrl+C/Ctrl+V (background scp, `-3` for remote↔remote).
- **✅ Logs viewer:** launcher → tetron-wm → Logs; `c` copies the log to the host clipboard via OSC 52.
- **✅ Scrollback:** the mouse wheel pages back through any app window's history.
- **✅ Versioned releases + update channels:** semver in `Cargo.toml`, a `CHANGELOG`, and a main/dev channel switcher in Settings → Updates (main installs prebuilt releases fast; dev builds from source).
- **✅ AI assistant:** the ✦ chat panel/window — switchable between the opencode CLI (model-agnostic, MCP-extensible) and hermes (Settings → Assistant), the repo `agent/` briefing pack, desktop control via the `tetron-wm` CLI, cross-machine awareness.

## Planned

- **Slice 6 — GUI/Wayland mode** (host real GUI apps; audio/video streaming to the client) — plus a parked idea: a fullscreen **browser PWA** of tetron-wm (multiple simultaneous frontends on one apphost).
- **Slice 7 — Standalone "TUI-OS" app** (bundle a GPU terminal + tetron-wm into a fullscreen app).
- **Taskbar scroll** — when the `…+N` overflow marker appears, make it a scroll/cycle affordance so a very long window list stays reachable.
