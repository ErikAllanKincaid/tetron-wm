# tetron-wm recommended installs

tetron-wm is one binary and runs without anything else installed. Each feature below uses an outside program when it is present, and quietly hides or simplifies itself when it is not, so you can add these later at any time. Most of the app-type extras can also be installed from the in-app Store (leader `S`).

## Quick install

Ubuntu or Debian, everything that is packaged:

```sh
sudo apt install chafa mpv poppler-utils sshpass wireplumber network-manager bluez upower xclip wl-clipboard htop btop alsa-utils fonts-dejavu-core
```

macOS with [Homebrew](https://brew.sh):

```sh
brew install chafa mpv poppler fresh-editor blueutil htop btop
```

Chromecast casting, any OS with [uv](https://docs.astral.sh/uv/):

```sh
uv tool install catt
uv tool install git+https://github.com/ErikAllanKincaid/casttui
```

The suite installer (`install-tetron-wm-suite.sh`, see the [README](../README.md#install)) installs these for you after a prompt, only the ones that are missing, with apt, dnf, pacman or Homebrew. It asks twice: once for the recommended tools (chafa, mpv, poppler, fresh, sshpass, htop, btop, upower, alsa-utils, the DejaVu Sans Mono font, catt and casttui with `uv tool install` (installing uv first if missing), and xclip or wl-clipboard on a desktop), default yes, and once for the system services PipeWire (`wireplumber`), NetworkManager and BlueZ, default no, because those change how the machine manages sound, network and Bluetooth. Flags: `--extras` / `--no-extras` and `--services` / `--no-services`. Its tetron-wm step also offers gpm and the Nerd Font.

## By feature

| Feature | Program | Without it | Notes |
| --- | --- | --- | --- |
| Image wallpaper | `chafa` | Plain colored desktop | Renders the picture as colored cells, so it works on every terminal. [chafa](https://hpjansson.org/chafa/) |
| Image viewer on terminals without Kitty graphics (bare console, kmscon, tmux, ssh) | `chafa` | Coarser built-in half blocks (colored blocks on a bare console) | Draws the image as colored cells, dithered for 8 and 16 colors. Terminals with Kitty graphics show the real image either way. [chafa](https://hpjansson.org/chafa/) |
| Video in a window | `mpv` | Videos open only in a GUI player, if there is one | `mpv --vo=tct FILE` plays in a tetron-wm window on any terminal, kmscon and the bare console included. Fedora: from RPM Fusion. [mpv](https://mpv.io/) |
| Text editor in Open with… | `fresh` | Other editors on the list (nvim, vim, nano, …) | Easy, mouse-friendly terminal editor, offered first for text. Not in distro repositories: the suite installer runs its official installer on Linux (`curl -fsSL https://raw.githubusercontent.com/sinelaw/fresh/refs/heads/master/scripts/install.sh \| sh`, which picks .deb, .rpm, AUR, …); macOS `brew install fresh-editor`. [fresh](https://github.com/sinelaw/fresh) |
| PDFs in the built-in viewer | `pdftoppm`, `pdfinfo`, `pdftotext` (package `poppler-utils`; `poppler` on Arch and Homebrew) | PDFs offer only the apps you have installed | Renders one page at a time into the image viewer (characters on any terminal, the real page with Kitty graphics); the text view uses `pdftotext`. Optional: `tdf` and `timg` (Store) appear in Open with…, and `fbgs` (with fbi and ghostscript) gives full screen on a plain console. |
| Cast to Chromecast (right-click on video, audio and images) | `catt` | No Cast to Chromecast item | Serves the file from this machine until playback ends; pick the device with `cast_device`. Not in distro repositories: `uv tool install catt`. [catt](https://github.com/skorokithakis/catt) |
| Cast remote control (Open with… on video and audio) | `casttui` | Not listed in Open with… | Play/pause, seek, volume and stop for the cast, and converts codecs a Chromecast can not play. `uv tool install git+https://github.com/ErikAllanKincaid/casttui`. [casttui](https://github.com/ErikAllanKincaid/casttui) |
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
