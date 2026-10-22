# tetron-wm

**A desktop for your terminal.** Floating windows, a taskbar, a launcher, an app store and a mouse, all drawn as text inside the terminal you already have. Think of it as a graphical tmux.

<p align="center">
  <img src="assets/tetron-wm.png" alt="tetron-wm running in a terminal: floating windows, a top taskbar, a status tray and a mouse cursor" width="900">
</p>

Your apps run in the background, so you can close the terminal, drop an ssh connection or update tetron-wm, and everything is still there when you come back.

## Why use it

- **Windows in a terminal.** Run btop, a shell, vim and a file manager side by side. Drag titlebars, resize edges, click the taskbar. The mouse also reaches apps that use it.
- **Keeps running.** Detach and reattach from any terminal or over ssh, like tmux. Reloading or updating the UI does not close your apps.
- **Works almost anywhere.** Any modern terminal, over ssh, or on a bare Linux console with no X11 or Wayland.
- **Batteries included.** An app store with 600+ terminal apps, a file manager, desktop icons, themes, wallpapers, a clipboard history and an optional AI assistant.

## Install

Prebuilt binary for Linux (x86_64, aarch64) and macOS (Apple silicon, Intel). No Rust needed.

```sh
curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/install-tetron-wm-suite.sh | bash
```

It installs to `/usr/local/bin` and uses `sudo` for that. To install into your home instead:

```sh
TETRON_WM_BIN_DIR="$HOME/.local/bin" curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/install.sh | sh
```

Or build from source with a [Rust toolchain](https://rustup.rs) (1.95 or newer):

```sh
cargo install --git https://github.com/ErikAllanKincaid/tetron-wm
```

### Recommended extras

tetron-wm works on its own; these make the most of it. Ubuntu or Debian:

```sh
sudo apt install chafa sshpass wireplumber network-manager bluez upower xclip wl-clipboard htop btop alsa-utils
```

The suite installer above offers to install them for you (only the missing ones; the system services PipeWire, NetworkManager and BlueZ are a separate question that defaults to no). They add the image wallpaper (chafa), volume and output switching (PipeWire `wpctl`), Wi-Fi, Bluetooth and battery details, desktop clipboard sync, password-based Add Remote, and better tray tools. On a bare console add `gpm` for the mouse, or kmscon for colors and mouse. What each one does, what happens without it, and the macOS equivalents: [docs/DEPENDENCIES_tetron-wm_recommended_installs.md](docs/DEPENDENCIES_tetron-wm_recommended_installs.md).

Want a bare Linux machine that boots straight into the desktop? Use the suite installer, which can also set up the kmscon console, autologin and autostart, and can undo all of it. See [Bare console](#bare-console-no-x11-or-wayland).

## tetron mesh VPN companion network

###### TLDR: Install the entire suite.

Install suite.

[tetron](https://github.com/ErikAllanKincaid/tetron)

```shell
curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron/main/contrib/install-tetron-suite.sh | bash
```

tetron-wm has first-class, optional support for [tetron](https://github.com/ErikAllanKincaid/tetron), the P2P mesh VPN it ships alongside in the tetron-os image (below), right in the menubar tray. The integration is self-effacing: the tray talks to the tetron daemon over its own Unix IPC socket using `tetron-proto` (the shared wire protocol, the same crate tetron-systray and tetron-webui speak), and if no daemon answers (tetron not installed, or not running) the segment simply hides. There is nothing to configure, and tetron-wm works identically with or without tetron present.

- **Status at a glance.** The tray shows `⇄<peers>` with a connection-quality dot: `●` when at least one network has a direct path, `○` when every connection is relay-only, and `off` when tetron is reachable but in standby. It is read-only, a background poll that bounds every call so a stuck daemon can never hang the tray.
- **On/off from the popover.** Click the segment for a popover with a global **Tetron [on/off]** toggle plus one row per network. Each network row toggles just that network's data plane (**Resume** / **Standby**) with its own `●`/`○` direct-or-relay indicator; the global toggle moves them all at once. These map to tetron's own `Resume`/`Standby` IPC messages, the same effect as `tetron resume` / `tetron standby` from the CLI.
- **Open terminal.** An action row at the bottom of the popover opens a terminal into tetron: it launches [`tetron-tui`](https://github.com/ErikAllanKincaid/tetron-tui) (the full-screen tetron dashboard) when that is installed, and otherwise prints `tetron status` and drops you into an interactive shell with the full `tetron` CLI. Invites and joining a network live in the cli or tetron-tui, not in the tray.

## First five minutes

```sh
tetron-wm
```

1. **Open the launcher.** Click the button at the top left, or press `Ctrl+Space` then `l`. The first entry is a **Shell**.
2. **Move windows.** Drag a titlebar to move, an edge to resize. The titlebar buttons minimize, maximize and close. From the keyboard, `Ctrl+Space` then `Tab` focuses the next window (keep pressing `Tab` to cycle), `Ctrl+Space` then `f` lists every window to pick one by number, and `Ctrl+Space` then `w` moves the focused window with the arrow keys (`r` to resize, Enter when done).
3. **Get apps.** Open the **Store** (`Ctrl+Space`, then `S`), search and install. If an app needs a toolchain you do not have, the installer notices and offers to set it up first.
4. **Find things fast.** `Ctrl+Space` then `space` opens a search over every app.
5. **See all shortcuts.** `Ctrl+Space` then `?`.
6. **Leave without closing anything.** `Ctrl+Space` then `q` detaches. Run `tetron-wm` again to come back.

`Ctrl+Space` is the **leader**: press it, release it, then press the next key. It keeps tetron-wm shortcuts from colliding with your terminal or the apps inside it. Every key, and how to change it, is in [docs/KEYS_tetron-wm_leader_key_table.md](docs/KEYS_tetron-wm_leader_key_table.md).

## Sessions: detach, reattach, stay running

```sh
tetron-wm ls                    # list sessions: attached | detached | recoverable | stale
tetron-wm attach                # come back to a running session
tetron-wm attach -d             # take over a session another terminal or ssh login holds
tetron-wm -s work               # a separate workspace called "work", side by side with the default
tetron-wm reload                # restart only the UI; apps keep running
tetron-wm kill                  # stop everything and close all apps
```

Any command can target a session with `-s NAME`. More examples are in the Recipes section of [docs/CONFIG.md](docs/CONFIG.md), and `tetron-wm --help` lists every command.

## Make it yours

Everything works with no config file. To change something, copy the example and edit it:

```sh
mkdir -p ~/.config/tetron-wm
cp config.example.toml ~/.config/tetron-wm/config.toml
tetron-wm reload
```

- [config.example.toml](config.example.toml) is a ready-to-copy file with every option at its default and a note on each.
- [docs/CONFIG.md](docs/CONFIG.md) is the full reference plus copy-paste recipes: tiling grids, themes, wallpaper, launcher entries, clipboard, servers that must stay up, and more.
- Many options are also in the app under **Settings** (`Ctrl+Space`, then `,`).

Common first changes:

| I want                                   | Set                                                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| A different color theme                  | `theme = "nord"` (also `newmoon`, `midnight`, `gruvbox`, `dracula`, `pastoral`)                                            |
| Windows to snap to screen edges          | `snapping_enabled = true`                                                                                                  |
| Windows arranged in a grid automatically | `auto_tile = true`, `grid_rows`, `grid_cols`                                                                               |
| A wallpaper image                        | put it in `~/.config/tetron-wm/wallpapers/` and set `wallpaper = "name.jpg"` (needs [chafa](https://hpjansson.org/chafa/)) |
| A different leader key                   | `leader = "ctrl+b"`                                                                                                        |
| No Power menu on a server                | `power_menu = false`                                                                                                       |

## Bare console (no X11 or Wayland)

tetron-wm draws everything as cells, so it also runs on a raw Linux console. For colors and mouse there, use the maintained [kmscon](https://github.com/Aetf/kmscon) fork. One script can set it all up, including autologin and starting tetron-wm at boot:

```sh
curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/install-tetron-wm-suite.sh | bash
```

It asks before every step, is safe to re-run, and `--uninstall` puts your login console back. Choose `--profile gui`, `console` or `appliance`, and use `--dry-run` to preview the plan.

- Step-by-step: [docs/SETUP_tetron-wm_bare_console_kmscon.md](docs/SETUP_tetron-wm_bare_console_kmscon.md)
- Why each piece is needed, with diagrams: [docs/tetron-wm_bare_console_guide.html](docs/tetron-wm_bare_console_guide.html)
- Just kmscon: [install-kmscon.sh](install-kmscon.sh)
- Mouse on a plain Linux VT comes from the `gpm` service (`sudo apt install gpm`). Without a mouse, every window can still be switched (leader `Tab`, leader `f`) and moved or resized (leader `w`) from the keyboard.

## Updating

Inside the app: **Settings → Updates**. The default channel installs the latest prebuilt release, reloads, and keeps your apps running. Or re-run the install command above. If you track the `dev` branch it builds from source.

## Troubleshooting

- **Colors look washed out.** Your terminal may do 24-bit color without saying so. Set `truecolor = true`, or run once with `TETRON_WM_TRUECOLOR=1 tetron-wm`.
- **Colors are wrong or unreadable on a basic terminal.** Force the color depth: `TETRON_WM_COLORS=16 tetron-wm` (16 colors, the default on the bare Linux console), `TETRON_WM_COLORS=mono` (black and white, also set by `NO_COLOR`), or `256` / `truecolor`.
- **Nothing happens on `attach`.** Run `tetron-wm ls`. A `detached` session is waiting for you. `recoverable` means the daemon is gone but the apps are alive, so run `tetron-wm` to restore the windows.
- **Copying text out of an app that grabs the mouse** (vim, w3m): press `Ctrl+Space` then `x`, drag over the text, then `Ctrl+Space` then `C`.
- **No wallpaper.** Install `chafa`, or set `wallpaper_enabled = false` for a solid desktop.
- **Something else.** The in-app **Logs** viewer shows `~/tetron-wm-debug.log`, which is what to paste into a bug report.

## Files

| Path                                         | What                                                                    |
| -------------------------------------------- | ----------------------------------------------------------------------- |
| `~/.config/tetron-wm/config.toml`            | your settings                                                           |
| `~/.config/tetron-wm/themes/`, `wallpapers/` | your themes and images                                                  |
| `~/.local/share/tetron-wm/`                  | app-managed data: clipboard history, icon positions, recent directories |
| `~/tetron-wm-debug.log`                      | log                                                                     |
| `$XDG_RUNTIME_DIR/tetron-wm-$USER/`          | sockets                                                                 |

## More

- [docs/CONFIG.md](docs/CONFIG.md): every config option, plus copy-paste recipes for common setups
- [config.example.toml](config.example.toml): a ready-to-copy config with every option at its default
- [docs/FEATURES_tetron-wm_what_it_can_do.md](docs/FEATURES_tetron-wm_what_it_can_do.md): everything tetron-wm can do, by task
- [docs/KEYS_tetron-wm_leader_key_table.md](docs/KEYS_tetron-wm_leader_key_table.md): the leader key table
- [docs/DEPENDENCIES_tetron-wm_recommended_installs.md](docs/DEPENDENCIES_tetron-wm_recommended_installs.md): optional programs per feature
- [docs/SETUP_tetron-wm_bare_console_kmscon.md](docs/SETUP_tetron-wm_bare_console_kmscon.md): the bare console setup
- [CHANGELOG.md](CHANGELOG.md): what changed in each release
- [agent/](agent/README.md): what the built-in AI assistant (the ✦ button) is told about the desktop
- [CLAUDE.md](CLAUDE.md): architecture and conventions for contributors and coding agents

Build and test from a clone:

```sh
cargo build
cargo test
cargo clippy --all-targets
```

## License

[MIT](LICENSE)
