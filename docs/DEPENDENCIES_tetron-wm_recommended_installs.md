# tetron-wm recommended installs

tetron-wm is one binary and runs without anything else installed. Each feature below uses an outside program when it is present, and quietly hides or simplifies itself when it is not, so you can add these later at any time. Most of the app-type extras can also be installed from the in-app Store (leader `S`).

## Quick install

Ubuntu or Debian, everything that is packaged:

```sh
sudo apt install chafa sshpass wireplumber network-manager bluez upower xclip wl-clipboard htop btop alsa-utils fonts-dejavu-core
```

macOS with [Homebrew](https://brew.sh):

```sh
brew install chafa blueutil htop btop
```

The suite installer (`install-tetron-wm-suite.sh`, see the [README](../README.md#install)) installs these for you after a prompt, only the ones that are missing, with apt, dnf, pacman or Homebrew. It asks twice: once for the recommended tools (chafa, sshpass, htop, btop, upower, alsa-utils, the DejaVu Sans Mono font, and xclip or wl-clipboard on a desktop), default yes, and once for the system services PipeWire (`wireplumber`), NetworkManager and BlueZ, default no, because those change how the machine manages sound, network and Bluetooth. Flags: `--extras` / `--no-extras` and `--services` / `--no-services`. Its tetron-wm step also offers gpm and the Nerd Font.

## By feature

| Feature | Program | Without it | Notes |
| --- | --- | --- | --- |
| Image wallpaper | `chafa` | Plain colored desktop | Renders the picture as colored cells, so it works on every terminal. [chafa](https://hpjansson.org/chafa/) |
| Mouse on a bare Linux console | `gpm` | No mouse on a raw VT | Not needed under kmscon or in a GUI terminal. `sudo apt install gpm && sudo systemctl enable --now gpm` |
| Colors and mouse on a bare console | `kmscon` (maintained Aetf fork) | 16 colors, gpm mouse only | `install-kmscon.sh`. Details: [SETUP_tetron-wm_bare_console_kmscon.md](SETUP_tetron-wm_bare_console_kmscon.md) |
| Volume (menubar, leader `=`, Settings → Sound) | `wpctl` (PipeWire, package `wireplumber`) | Volume shows no level and can not change; Settings says so | Also lists and switches outputs (HDMI, headphones, speakers). macOS uses its built-in `osascript`. |
| Volume mixer row | `wiremix`, else `pulsemixer`, `ncpamixer`, `alsamixer` | Row offers "Get wiremix (Store)" | wiremix is in the Store, or `cargo install wiremix` (needs `libpipewire-0.3-dev`, `pkg-config`, `clang`). `alsamixer` comes with `alsa-utils`. Change the order with `[tray_tools] volume`. |
| Wi-Fi segment | `nmcli` (NetworkManager) | No Wi-Fi control; the network address still shows | macOS uses `networksetup`. The popover's Open row prefers `wlctl`, `nmtui`, `iwctl`. |
| Bluetooth segment | `bluetoothctl` (BlueZ) | No Bluetooth control | macOS: `blueutil`. The Open row prefers `bluetuith` (in the Store). |
| Battery details | `upower` | The battery percentage still shows | The Open row prefers `jolt`. macOS uses `pmset`. |
| CPU and memory popovers | `htop` or `btop` | Falls back to `top` | Change with `[tray_tools] cpu`. |
| Desktop clipboard sync | `wl-copy` / `wl-paste` (Wayland) or `xclip` (X11) | History still records copies made inside tetron-wm | Only matters on a desktop session; over ssh and on a bare console tetron-wm's own clipboard is used. |
| Add Remote with a password | `sshpass` | Add Remote asks you to type the password in its terminal instead | The password is used once and never saved. `ssh`, `scp` and `ssh-copy-id` (package `openssh-client`, usually preinstalled) are needed for Systems and remote Files. |
| Icons and symbols drawn correctly | DejaVu Sans Mono (`fonts-dejavu-core`, Fedora `dejavu-sans-mono-fonts`, Arch `ttf-dejavu`) | A rare missing symbol where the terminal font lacks it and no installed font fills in | The default `icon_style = "auto"` draws only symbols DejaVu has. On Linux, terminals fill glyphs their own font lacks from any installed font, so DejaVu being installed is enough; you do not have to switch your terminal font. kmscon uses it directly (`install-kmscon.sh` installs it with Terminus). macOS needs nothing. |
| Richer icons | A Symbols Nerd Font | Use `icon_style = "auto"` (default), which needs no special font | Only for `icon_style = "nerd"`. The suite installer can install it. |
| Real images in windows | A terminal with the Kitty graphics protocol (Kitty, Ghostty, WezTerm) | Images are drawn as cells instead | |
| AI assistant (✦) | `opencode`, or `hermes` | Settings → Assistant marks the agent "(not installed)" | Switch in Settings → Assistant. [opencode](https://opencode.ai) |
| Updates from Settings | `curl`; `cargo` for the dev channel | Update manually with the install command | The main channel downloads a prebuilt release. |
| Mesh VPN segment | `tetron` and `tetron-tui` | No VPN segment | [tetron-tui](https://github.com/ErikAllanKincaid/tetron-tui). Optional companion. |

## Store installs

The Store installs apps with the toolchain each one needs (Go, Rust, Node or Python). If a toolchain is missing, the Store offers to set it up first, so you do not need to install toolchains in advance.
