#!/bin/sh
# tetron-wm installer — downloads the latest prebuilt binary for your platform.
#
#   curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/install.sh | sh
#
# The default install dir is /usr/local/bin (system-wide, already on PATH), which
# needs root — the script uses sudo when the dir is not writable. For a no-sudo,
# userspace install set TETRON_WM_BIN_DIR to a writable dir, e.g.:
#   TETRON_WM_BIN_DIR="$HOME/.local/bin" curl -fsSL …/install.sh | sh
#
# One install method per host. On Debian and Ubuntu (dpkg and apt-get present):
# - tetron-wm installed by apt: the release .deb upgrades it, through apt. When
#   the release has no .deb the script changes nothing and exits with status 3,
#   so it never writes over a file the package owns.
# - a raw copy (not from a package): upgraded in place, as before.
# - neither: the release .deb is installed when the release has one, else the
#   raw binary.
# Everywhere else (other Linux, macOS) the raw binary, as before.
set -eu

# Prebuilt binaries come from the tetron-wm release repo. The binary is named
# `tetron-wm` and installs to its own paths, so it collides with nothing else.
REPO="ErikAllanKincaid/tetron-wm"
PKG="tetron-wm"
BIN_DIR="${TETRON_WM_BIN_DIR:-/usr/local/bin}"
# Release downloads and the tag can be overridden (tests serve a local release).
RELEASES="${TETRON_WM_RELEASES_URL:-https://github.com/$REPO/releases}"

# Does writing BIN_DIR need root? Walk up to the first existing ancestor and
# test it: a path under $HOME (even if the dir doesn't exist yet) is writable
# and needs no sudo, while /usr/local/bin exists but is root-owned.
need_sudo_for() {
  d="$1"
  while [ ! -e "$d" ]; do d="$(dirname "$d")"; [ "$d" = "/" ] && break; done
  [ -w "$d" ] && return 1 || return 0
}
SUDO=""
raw_sudo() {
  if need_sudo_for "$BIN_DIR"; then
    if [ "$(id -u)" -eq 0 ]; then :
    elif command -v sudo >/dev/null 2>&1; then
      echo "tetron-wm: $BIN_DIR needs root — using sudo (set TETRON_WM_BIN_DIR to a user dir for a no-sudo install)"
      SUDO="sudo"
    else
      echo "tetron-wm: cannot write $BIN_DIR and sudo is unavailable." >&2
      echo "  Retry with:  TETRON_WM_BIN_DIR=\"\$HOME/.local/bin\" curl -fsSL …/install.sh | sh" >&2
      exit 1
    fi
  fi
}

# Stock Debian often has wget but not curl; accept either.
fetch() {
  if command -v curl >/dev/null 2>&1; then curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then wget -qO- "$1"
  else echo "tetron-wm: need curl or wget to download" >&2; return 1
  fi
}

# Resolve the latest release tag WITHOUT the api.github.com REST endpoint.
# That endpoint is rate-limited to 60 requests/hour per IP and answers 403 once
# the budget is spent — which previously surfaced as a bogus "no published
# release yet" and silently wedged the in-app updater into a slow source-build
# fallback. The web redirect github.com/OWNER/REPO/releases/latest 302s to
# .../releases/tag/vX.Y.Z off a different, effectively-unlimited path; we read
# the tag straight out of its Location header. The REST API is only a fallback
# for when the redirect can't be parsed (and we're still under the hourly cap).
latest_tag() {
  redirect="https://github.com/$REPO/releases/latest"
  loc=""
  if command -v curl >/dev/null 2>&1; then
    loc="$(curl -fsSI "$redirect" | tr -d '\r' | awk 'tolower($1)=="location:"{print $2}' | tail -1)"
  elif command -v wget >/dev/null 2>&1; then
    loc="$(wget -qS --max-redirect=0 -O /dev/null "$redirect" 2>&1 | tr -d '\r' | awk 'tolower($1)=="location:"{print $2}' | tail -1)"
  fi
  case "$loc" in
    */releases/tag/*) printf '%s\n' "${loc##*/}"; return 0 ;;
  esac
  fetch "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4
}

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Darwin/arm64)        target="aarch64-apple-darwin" ;;
  Darwin/x86_64)       target="x86_64-apple-darwin" ;;
  Linux/x86_64)        target="x86_64-unknown-linux-gnu" ;;
  Linux/aarch64)       target="aarch64-unknown-linux-gnu" ;;
  *)
    echo "tetron-wm: no prebuilt binary for $os/$arch."
    echo "Install with Rust instead:  cargo install --git https://github.com/$REPO"
    exit 1 ;;
esac

tag="${TETRON_WM_TAG:-}"
if [ -z "$tag" ]; then
  echo "tetron-wm: finding latest release…"
  tag="$(latest_tag || true)"
fi
if [ -z "${tag:-}" ]; then
  echo "tetron-wm: couldn't resolve the latest release (GitHub unreachable or rate-limited)."
  echo "Install with Rust instead:  cargo install --git https://github.com/$REPO"
  exit 1
fi

# Debian and Ubuntu: dpkg and apt-get present.
deb_host() {
  [ "$os" = Linux ] && command -v dpkg >/dev/null 2>&1 && command -v apt-get >/dev/null 2>&1
}
# apt installed the tetron-wm package.
pkg_installed() {
  dpkg-query -W -f='${db:Status-Status}' "$PKG" 2>/dev/null | grep -qx installed
}
# The first tetron-wm (in BIN_DIR, else on PATH) that no package owns.
raw_copy() {
  for p in "$BIN_DIR/tetron-wm" "$(command -v tetron-wm 2>/dev/null || true)"; do
    [ -n "$p" ] && [ -f "$p" ] || continue
    dpkg -S "$(readlink -f "$p")" >/dev/null 2>&1 && continue
    printf '%s\n' "$p"
    return 0
  done
  return 1
}
# Name of the release's tetron-wm_<version>-<rev>_<arch>.deb, or nothing. Read
# from the release page's asset list (not the rate-limited REST API), so a
# packaging-only revision (-2) is found without guessing.
deb_asset() {
  fetch "$RELEASES/expanded_assets/$tag" 2>/dev/null \
    | grep -o "${PKG}_[^\"/<> ]*_$1\.deb" | head -1 || true
}
# Download the .deb and its .sha256, check it, install it with apt-get.
install_deb() {
  asset="$1"
  tmp="$(mktemp -d)"
  echo "tetron-wm: downloading $asset ($tag)…"
  if ! fetch "$RELEASES/download/$tag/$asset" > "$tmp/$asset" \
    || ! fetch "$RELEASES/download/$tag/$asset.sha256" > "$tmp/$asset.sha256"; then
    echo "tetron-wm: download of $asset failed; nothing changed." >&2
    rm -rf "$tmp"; exit 1
  fi
  want="$(cut -d' ' -f1 < "$tmp/$asset.sha256")"
  got="$(sha256sum "$tmp/$asset" | cut -d' ' -f1)"
  if [ -z "$want" ] || [ "$want" != "$got" ]; then
    echo "tetron-wm: checksum mismatch for $asset; nothing changed." >&2
    rm -rf "$tmp"; exit 1
  fi
  apt_sudo=""
  [ "$(id -u)" -eq 0 ] || apt_sudo="sudo"
  echo "tetron-wm: installing $asset with apt-get…"
  if ! $apt_sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y "$tmp/$asset"; then
    echo "tetron-wm: apt-get could not install $asset." >&2
    rm -rf "$tmp"; exit 1
  fi
  rm -rf "$tmp"
  BIN_DIR="/usr/bin"
  echo "tetron-wm: installed $tag -> /usr/bin/tetron-wm (package $PKG, managed by apt)"
}

method="raw"
note=""
if deb_host; then
  debarch="$(dpkg --print-architecture)"
  if pkg_installed; then
    asset="$(deb_asset "$debarch")"
    if [ -z "$asset" ]; then
      echo "tetron-wm: installed by apt, and release $tag has no .deb for $debarch." >&2
      echo "  Nothing changed. See $RELEASES/tag/$tag" >&2
      exit 3
    fi
    method="deb"
  elif raw="$(raw_copy)"; then
    note="tetron-wm: $raw is not from a package, so it was upgraded in place. To let apt manage tetron-wm, delete it and run this script again."
  else
    asset="$(deb_asset "$debarch")"
    [ -n "$asset" ] && method="deb"
  fi
fi

if [ "$method" = deb ]; then
  install_deb "$asset"
else
  raw_sudo
  url="$RELEASES/download/$tag/tetron-wm-$target.tar.gz"
  echo "tetron-wm: downloading $tag ($target)…"
  $SUDO mkdir -p "$BIN_DIR"
  if ! fetch "$url" | $SUDO tar -xz -C "$BIN_DIR" 2>/dev/null || [ ! -f "$BIN_DIR/tetron-wm" ]; then
    echo "tetron-wm: no prebuilt binary for $target in $tag."
    echo "Install with Rust instead:  cargo install --git https://github.com/$REPO"
    exit 1
  fi
  $SUDO chmod +x "$BIN_DIR/tetron-wm"
  echo "tetron-wm: installed $tag -> $BIN_DIR/tetron-wm"
  [ -z "$note" ] || echo "$note"
fi

# Optional, OS-aware dependency step. Installs helpers some features need:
#   blueutil (macOS)  — Bluetooth tray control
#   gpm (Linux)       — mouse on a bare console / VT
#   sshpass           — automates the one-time password for Systems → Add Remote
#   chafa             — render an image wallpaper to cells (works on kmscon too)
#   Symbols Nerd Font — the glyphs used by `icon_style = "nerd"`
# Transparent and skippable: it prints what it runs, skips silently with no
# package manager, honours TETRON_WM_SKIP_DEPS, and in a non-interactive
# `curl | sh` requires explicit TETRON_WM_INSTALL_DEPS=1 so piping the installer
# never surprises you with package installs.
# A Nerd Font supplies the Private-Use-Area glyphs tetron-wm draws for
# `icon_style = "nerd"`. "Symbols Only" is tiny and has no Latin letters, so
# fontconfig uses it purely as a fallback for those glyphs (in kmscon and most
# terminals) without changing your normal text font. No sudo on Linux (user
# fonts dir); a brew cask on macOS.
install_nerd_font() {
  # Already have one? fontconfig is the arbiter on both platforms.
  if command -v fc-list >/dev/null 2>&1 && fc-list 2>/dev/null | grep -qi "nerd font"; then
    return 0
  fi
  case "$(uname -s)" in
    Darwin)
      if command -v brew >/dev/null 2>&1; then
        echo "tetron-wm: installing optional dependency Symbols Nerd Font (icon_style=nerd)…"
        brew install --cask font-symbols-only-nerd-font 2>/dev/null \
          || echo "tetron-wm: Nerd Font skipped (run 'brew install --cask font-symbols-only-nerd-font' later for icon_style=nerd)"
      fi ;;
    Linux)
      if ! command -v unzip >/dev/null 2>&1; then
        echo "tetron-wm: Nerd Font skipped (needs 'unzip'; install it + a Nerd Font for icon_style=nerd)"
        return 0
      fi
      fdir="$HOME/.local/share/fonts"
      url="https://github.com/ryanoasis/nerd-fonts/releases/latest/download/NerdFontsSymbolsOnly.zip"
      tmp="$(mktemp -d)" || return 0
      echo "tetron-wm: installing optional dependency Symbols Nerd Font (icon_style=nerd)…"
      if fetch "$url" > "$tmp/nf.zip" 2>/dev/null && [ -s "$tmp/nf.zip" ]; then
        mkdir -p "$fdir"
        if unzip -o -q "$tmp/nf.zip" '*.ttf' -d "$fdir" 2>/dev/null; then
          command -v fc-cache >/dev/null 2>&1 && fc-cache -f "$fdir" >/dev/null 2>&1
          echo "tetron-wm: Symbols Nerd Font installed to $fdir"
        else
          echo "tetron-wm: Nerd Font extract failed (install a Nerd Font manually for icon_style=nerd)"
        fi
      else
        echo "tetron-wm: Nerd Font download skipped (install a Nerd Font manually for icon_style=nerd)"
      fi
      rm -rf "$tmp" ;;
  esac
}

install_optional_deps() {
  [ "${TETRON_WM_SKIP_DEPS:-0}" = "1" ] && return 0
  if [ ! -t 0 ] && [ "${TETRON_WM_INSTALL_DEPS:-0}" != "1" ]; then return 0; fi
  case "$(uname -s)" in
    Darwin)
      if command -v brew >/dev/null 2>&1; then
        if ! command -v blueutil >/dev/null 2>&1; then
          echo "tetron-wm: installing optional dependency blueutil (Bluetooth control)…"
          brew install blueutil || echo "tetron-wm: blueutil install skipped (run 'brew install blueutil' later for Bluetooth control)"
        fi
        if ! command -v sshpass >/dev/null 2>&1; then
          # sshpass is not in homebrew-core; use the common tap, skip on failure.
          echo "tetron-wm: installing optional dependency sshpass (remote-system setup)…"
          brew install sshpass 2>/dev/null \
            || brew install esolitos/ipa/sshpass 2>/dev/null \
            || echo "tetron-wm: sshpass install skipped (Add Remote will prompt for the password interactively instead)"
        fi
        if ! command -v chafa >/dev/null 2>&1; then
          echo "tetron-wm: installing optional dependency chafa (image wallpaper)…"
          brew install chafa || echo "tetron-wm: chafa install skipped (run 'brew install chafa' later for image wallpaper)"
        fi
      fi ;;
    Linux)
      # bluetoothctl/rfkill ship with the distro; gpm gives a bare-console
      # mouse, sshpass automates Systems → Add Remote key transfers, and chafa
      # renders image wallpapers to cells (works on a kmscon console too).
      pkgs=""
      command -v gpm >/dev/null 2>&1 || pkgs="gpm"
      command -v sshpass >/dev/null 2>&1 || pkgs="$pkgs sshpass"
      command -v chafa >/dev/null 2>&1 || pkgs="$pkgs chafa"
      command -v unzip >/dev/null 2>&1 || pkgs="$pkgs unzip"   # for the Nerd Font below
      pkgs="$(echo "$pkgs" | sed 's/^ *//')"
      if [ -n "$pkgs" ]; then
        echo "tetron-wm: installing optional dependencies: $pkgs …"
        sudo apt-get install -y $pkgs 2>/dev/null \
          || sudo dnf install -y $pkgs 2>/dev/null \
          || sudo pacman -S --noconfirm $pkgs 2>/dev/null \
          || sudo zypper install -y $pkgs 2>/dev/null \
          || echo "tetron-wm: optional deps skipped (install '$pkgs' with your package manager later)"
        # Start gpm now (and on boot) where systemd manages it.
        if command -v gpm >/dev/null 2>&1 && command -v systemctl >/dev/null 2>&1; then
          sudo systemctl enable --now gpm 2>/dev/null || true
        fi
      fi ;;
  esac
  install_nerd_font
}
install_optional_deps

case ":$PATH:" in
  *":$BIN_DIR:"*) echo "Run it with:  tetron-wm" ;;
  *) echo "Add $BIN_DIR to your PATH, then run:  tetron-wm"
     echo "  e.g.  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.zprofile" ;;
esac
