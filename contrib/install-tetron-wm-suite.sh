#!/usr/bin/env bash
# install-tetron-wm-suite.sh — one script to stand up a tetron-wm desktop.
#
# Runs the individual installers for you and, optionally, turns a bare Linux box
# into a console appliance that boots straight into the desktop. Every step is
# opt-in and prompted; nothing happens without a yes. Pipe it:
#
#   curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/contrib/install-tetron-wm-suite.sh | bash
#
# It can install/configure, in order:
#   1. tetron-wm      — the latest prebuilt binary (this is install.sh)         [all OSes]
#   2. config dir     — ~/.config/tetron-wm/{config.toml,themes/,wallpapers/}   [all OSes]
#   3. kmscon         — the truecolor + mouse console for a bare VT             [Linux]
#   4. tty1 autologin — boot straight into tetron-wm on tty1 (via kmscon)       [Linux + systemd]
#
# Steps 3–4 only apply to a GUI-less / console box; skip them on a normal
# desktop (just say no). Step 4 edits your login console — it is off by default,
# always confirmed, and prints how to undo it.
#
# Usage:  install-tetron-wm-suite.sh [options]
#   -y, --yes        assume the default answer to every prompt (non-interactive).
#                    Defaults: wm=yes, config=yes, kmscon=yes (Linux), tty1=NO.
#   --all            select every step (wm + config + kmscon + tty1). With -y,
#                    builds the full console appliance with no prompts.
#   --wm / --no-wm             force the tetron-wm step on/off
#   --config / --no-config     force the config-dir step on/off
#   --kmscon / --no-kmscon     force the kmscon step on/off
#   --tty1 / --no-tty1         force the tty1-autologin step on/off
#   -h, --help       show this help
#
# Env overrides: REPO (default ErikAllanKincaid/tetron-wm), BRANCH (default main),
# and anything install.sh honours (TETRON_WM_BIN_DIR, TETRON_WM_INSTALL_DEPS, …).
set -euo pipefail

REPO="${REPO:-ErikAllanKincaid/tetron-wm}"
BRANCH="${BRANCH:-main}"
RAW="https://raw.githubusercontent.com/$REPO/$BRANCH"

# ── Output helpers (match scripts/install-kmscon.sh) ────────────────────────────
if [ -t 1 ]; then B=$'\033[1m'; Y=$'\033[33m'; R=$'\033[31m'; G=$'\033[32m'; N=$'\033[0m'; else B= Y= R= G= N=; fi
log()  { printf '%s==>%s %s\n'  "$G" "$N" "$*"; }
warn() { printf '%sWARN:%s %s\n' "$Y" "$N" "$*" >&2; }
die()  { printf '%sERROR:%s %s\n' "$R" "$N" "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

usage() { sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//; s/^#$//' | sed '$d'; exit "${1:-0}"; }

# Stock Debian often has wget but not curl; accept either.
fetch() {
  if have curl; then curl -fsSL "$1"
  elif have wget; then wget -qO- "$1"
  else die "need curl or wget to download"
  fi
}

# ── Args ─────────────────────────────────────────────────────────────────────
ASSUME_YES=0
WANT_WM=""; WANT_CONFIG=""; WANT_KMSCON=""; WANT_TTY1=""
while [ $# -gt 0 ]; do
  case "$1" in
    -y|--yes)     ASSUME_YES=1 ;;
    --all)        WANT_WM=y; WANT_CONFIG=y; WANT_KMSCON=y; WANT_TTY1=y ;;
    --wm)         WANT_WM=y ;;     --no-wm)     WANT_WM=n ;;
    --config)     WANT_CONFIG=y ;; --no-config) WANT_CONFIG=n ;;
    --kmscon)     WANT_KMSCON=y ;; --no-kmscon) WANT_KMSCON=n ;;
    --tty1)       WANT_TTY1=y ;;   --no-tty1)   WANT_TTY1=n ;;
    -h|--help)    usage 0 ;;
    *) die "unknown option: $1 (see --help)" ;;
  esac
  shift
done

OS="$(uname -s)"
IS_LINUX=0; [ "$OS" = "Linux" ] && IS_LINUX=1

# ── Privilege: sudo only where a step needs it (kmscon pkgs, tty1 unit edits) ───
if [ "$(id -u)" -eq 0 ]; then SUDO=""; else
  if have sudo; then SUDO="sudo"; else SUDO=""; fi
fi

# yes/no prompt with a default; reads /dev/tty so it still works under curl|bash.
# $1 = question, $2 = default (y|n). Honors -y (take default) and no-tty (default).
confirm() {
  local q="$1" def="$2" ans
  if [ "$ASSUME_YES" = 1 ] || [ ! -r /dev/tty ]; then
    [ "$def" = y ]; return
  fi
  if [ "$def" = y ]; then printf '%s%s [Y/n] %s' "$B" "$q" "$N" >&2
  else                    printf '%s%s [y/N] %s' "$B" "$q" "$N" >&2; fi
  read -r ans </dev/tty || ans=""
  ans="${ans:-$def}"
  case "$ans" in y|Y|yes|YES) return 0 ;; *) return 1 ;; esac
}

# Resolve a WANT_ flag: keep a value set on the command line, else ask (or take
# the default under -y / no tty). $1 = var name, $2 = question, $3 = default.
resolve() {
  local cur; eval "cur=\${$1}"
  [ -n "$cur" ] && return 0
  if confirm "$2" "$3"; then eval "$1=y"; else eval "$1=n"; fi
}

# ── Step 1: tetron-wm ──────────────────────────────────────────────────────────
install_wm() {
  log "installing tetron-wm (latest prebuilt release)…"
  local tmp; tmp="$(mktemp)"
  fetch "$RAW/install.sh" > "$tmp" || die "could not download install.sh"
  # Under -y we are non-interactive, so opt into install.sh's optional deps
  # (gpm/sshpass/chafa/Nerd Font) explicitly; otherwise let it prompt on /dev/tty.
  [ "$ASSUME_YES" = 1 ] && export TETRON_WM_INSTALL_DEPS=1
  if [ -r /dev/tty ]; then sh "$tmp" </dev/tty; else sh "$tmp"; fi
  rm -f "$tmp"
}

# Where the binary landed, for later steps (install.sh default: ~/.local/bin).
wm_bin() { command -v tetron-wm 2>/dev/null || echo "${TETRON_WM_BIN_DIR:-$HOME/.local/bin}/tetron-wm"; }

# ── Step 2: config dir ─────────────────────────────────────────────────────────
seed_config() {
  local cfg="$HOME/.config/tetron-wm"
  log "setting up $cfg (config.toml, themes/, wallpapers/)…"
  mkdir -p "$cfg/themes" "$cfg/wallpapers"
  if [ -f "$cfg/config.toml" ]; then
    log "config.toml already exists — leaving it untouched"
  else
    # The example is every option at its default, commented out: a safe starter.
    if fetch "$RAW/config.example.toml" > "$cfg/config.toml" 2>/dev/null && [ -s "$cfg/config.toml" ]; then
      log "wrote $cfg/config.toml (all defaults, commented — edit to taste)"
    else
      rm -f "$cfg/config.toml"
      warn "could not fetch config.example.toml; run with no config (defaults apply) and add one later"
    fi
  fi
  log "drop images in $cfg/wallpapers/ and set wallpaper=... ; editable themes: tetron-wm theme --dump <name>"
}

# ── Step 3: kmscon ─────────────────────────────────────────────────────────────
install_kmscon() {
  log "installing kmscon (truecolor + mouse console)…"
  local tmp; tmp="$(mktemp)"
  fetch "$RAW/scripts/install-kmscon.sh" > "$tmp" || die "could not download install-kmscon.sh"
  # The suite already captured consent ("Install kmscon?"), so always pass -y: a
  # second prompt here defaults to No and would abort on an Enter-through run.
  # install-kmscon.sh still logs the source build, and still uses sudo itself.
  bash "$tmp" -y
  rm -f "$tmp"
}

# ── Step 4: tty1 autologin into tetron-wm ──────────────────────────────────────
# Mirrors docs/SETUP_tetron-wm_bare_console_kmscon.md "Persist on tty1":
#   systemd → kmsconvt@tty1 → agetty --autologin USER → login shell → exec tetron-wm
setup_tty1() {
  have systemctl || { warn "no systemd (systemctl) — skipping tty1 setup"; return 0; }
  have kmscon    || { warn "kmscon is not installed — run the kmscon step first; skipping tty1"; return 0; }
  local wm; wm="$(wm_bin)"
  [ -x "$wm" ] || { warn "tetron-wm not found on PATH — install it first; skipping tty1"; return 0; }

  warn "This edits your tty1 login console. Keep a second VT (Ctrl+Alt+F2) or an"
  warn "SSH session open so a mistake can never lock you out. To undo later:"
  warn "  sudo systemctl disable --now kmsconvt@tty1 && sudo systemctl enable --now getty@tty1"
  confirm "Make tty1 boot straight into tetron-wm?" n || { log "skipping tty1 setup"; return 0; }

  local user kbin prof term='$TERM'
  user="$(logname 2>/dev/null || echo "${SUDO_USER:-$USER}")"
  kbin="$(command -v kmscon)"
  case "$(basename "${SHELL:-sh}")" in zsh) prof="$HOME/.zprofile" ;; *) prof="$HOME/.bash_profile" ;; esac

  log "1/3 point tty1 at kmscon instead of the kernel getty"
  $SUDO systemctl disable --now getty@tty1 2>/dev/null || true
  $SUDO systemctl enable  --now kmsconvt@tty1

  log "2/3 autologin $user on tty1 (drop-in override)"
  local ovrd="/etc/systemd/system/kmsconvt@tty1.service.d"
  $SUDO mkdir -p "$ovrd"
  # %%I -> %I (systemd instance); $TERM stays literal for agetty.
  printf '[Service]\nExecStart=\nExecStart=%s "--vt=%%I" --seats=seat0 --no-switchvt --login -- /sbin/agetty --autologin %s --noclear - %s\n' \
    "$kbin" "$user" "$term" | $SUDO tee "$ovrd/override.conf" >/dev/null
  $SUDO systemctl daemon-reload
  $SUDO systemctl restart kmsconvt@tty1

  log "3/3 exec tetron-wm from the tty1 login shell ($prof)"
  if grep -q 'TETRON_WM_ACTIVE' "$prof" 2>/dev/null; then
    log "login snippet already present in $prof"
  else
    cat >> "$prof" <<'EOF'

# Launch tetron-wm on tty1 login (added by install-tetron-wm-suite.sh).
# Drop `exec` to land on a shell when you quit the desktop.
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$TETRON_WM_ACTIVE" ]; then
  export TETRON_WM_ACTIVE=1
  exec tetron-wm
fi
EOF
    log "appended tty1 launch snippet to $prof"
  fi

  # Keep apps alive across UI reloads/updates by running the apphost as a service.
  "$wm" service install >/dev/null 2>&1 && log "installed the tetron-wm apphost service" || true
  log "tty1 is set. Reboot (or: sudo systemctl restart kmsconvt@tty1 ; then Ctrl+Alt+F1)."
}

# ── Orchestration ──────────────────────────────────────────────────────────────
main() {
  printf '%stetron-wm suite installer%s  (repo: %s, branch: %s)\n\n' "$B" "$N" "$REPO" "$BRANCH"

  resolve WANT_WM     "Install tetron-wm (latest release)?" y
  resolve WANT_CONFIG "Set up ~/.config/tetron-wm/ (config.toml, themes/, wallpapers/)?" y
  if [ "$IS_LINUX" = 1 ]; then
    resolve WANT_KMSCON "Install kmscon (truecolor + mouse console; compiles from source except on Debian trixie)?" y
    resolve WANT_TTY1   "Set up tty1 autologin into tetron-wm (console appliance)?" n
  else
    [ -z "$WANT_KMSCON" ] && WANT_KMSCON=n
    [ -z "$WANT_TTY1" ]   && WANT_TTY1=n
    [ "$WANT_KMSCON" = y ] && warn "kmscon is Linux-only — skipping"
    [ "$WANT_TTY1" = y ]   && warn "tty1 setup is Linux-only — skipping"
    WANT_KMSCON=n; WANT_TTY1=n
  fi

  echo
  [ "$WANT_WM" = y ]     && install_wm
  [ "$WANT_CONFIG" = y ] && seed_config
  [ "$WANT_KMSCON" = y ] && install_kmscon
  [ "$WANT_TTY1" = y ]   && setup_tty1

  echo
  log "done."
  if [ "$WANT_TTY1" != y ]; then
    case ":$PATH:" in
      *":$HOME/.local/bin:"*) log "run it with:  tetron-wm" ;;
      *) log "add ~/.local/bin to PATH, then run:  tetron-wm" ;;
    esac
  fi
}

main "$@"
