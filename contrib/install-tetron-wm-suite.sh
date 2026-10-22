#!/usr/bin/env bash
# install-tetron-wm-suite.sh — one script to stand up (or remove) a tetron-wm desktop.
#
# Runs the individual installers for you and, optionally, turns a bare Linux box
# into a console that boots straight into the desktop. Every step is opt-in and
# prompted (or chosen with flags); nothing happens without a yes. Pipe it:
#
#   curl -fsSL https://raw.githubusercontent.com/ErikAllanKincaid/tetron-wm/main/contrib/install-tetron-wm-suite.sh | bash
#
# Three common setups (pick one from the menu, or pass --profile):
#   gui        a machine with a desktop: tetron-wm in a terminal window.
#              Installs the binary + config only.
#   console    a VC tty with kmscon (truecolor + mouse) and a normal login
#              prompt. You log in and run `tetron-wm` yourself.
#   appliance  a VC tty with kmscon that logs in and starts tetron-wm for you
#              at boot (autologin + autostart).
#   custom     answer each question yourself.
#
# The pieces (all idempotent: safe to re-run, it converges, never duplicates):
#   1. tetron-wm      — the latest prebuilt binary (this is install.sh)         [all OSes]
#   2. config dir     — ~/.config/tetron-wm/{config.toml,themes/,wallpapers/}   [all OSes]
#      extras         — recommended tools: chafa, sshpass, htop, btop, upower,   [Linux, macOS]
#                       alsa-utils, and xclip / wl-clipboard on a desktop
#      services       — PipeWire audio (volume, outputs), NetworkManager (Wi-Fi),
#                       BlueZ (Bluetooth). Default NO: they change how the
#                       machine manages sound, network and Bluetooth
#                       (docs/DEPENDENCIES_tetron-wm_recommended_installs.md)
#   3. kmscon         — the truecolor + mouse console for a bare VT             [Linux]
#   4. console        — run kmscon on a VT in place of the kernel getty         [Linux + systemd]
#      autologin      — that VT logs your user in without a password prompt
#      autostart      — logging in on a kmscon console launches tetron-wm
#
# The console/autologin/autostart steps edit your login console: they are off
# unless you choose them, always confirmed, and `--uninstall` reverses them.
#
# Usage:  install-tetron-wm-suite.sh [options]
#   -y, --yes        assume the default answer to every prompt (non-interactive).
#                    Defaults: wm=yes, config=yes, kmscon=yes on a console box
#                    (no on a GUI box), console/autologin/autostart=NO unless
#                    selected by a flag or --profile.
#   --profile P      gui | console | appliance (see above)
#   --all            wm + config + kmscon + console + autologin + autostart (=appliance)
#   --wm / --no-wm             force the tetron-wm step on/off
#   --config / --no-config     force the config-dir step on/off
#   --extras / --no-extras     force the recommended-tools step on/off
#   --services / --no-services force the system-services step on/off
#   --kmscon / --no-kmscon     force the kmscon step on/off
#   --console / --no-console   run kmscon on a VT (replaces getty there)
#   --vt N                     which VT (default 1, the console that opens at boot)
#   --autologin / --no-autologin   log in on that VT without a prompt (implies --console)
#   --autostart / --no-autostart   launch tetron-wm on login at that VT only (--vt)
#   --tty1                     shorthand for the full appliance on tty1 (older flag)
#   --uninstall      undo the console / autologin / autostart changes and the
#                    apphost service (prompted; keeps the binary, config, kmscon)
#   --dry-run        print the plan the choices add up to, change nothing
#   --purge          with --uninstall: also remove the binary and ~/.config/tetron-wm
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
WANT_WM=""; WANT_CONFIG=""; WANT_EXTRAS=""; WANT_SERVICES=""; WANT_KMSCON=""; WANT_CONSOLE=""; WANT_AUTOLOGIN=""; WANT_AUTOSTART=""
VT=""; PROFILE=""; UNINSTALL=0; PURGE=0; DRY_RUN=0
while [ $# -gt 0 ]; do
  case "$1" in
    -y|--yes)     ASSUME_YES=1 ;;
    --profile)    shift; PROFILE="${1:-}" ;;
    --all)        PROFILE=appliance ;;
    --wm)         WANT_WM=y ;;     --no-wm)     WANT_WM=n ;;
    --config)     WANT_CONFIG=y ;; --no-config) WANT_CONFIG=n ;;
    --extras)     WANT_EXTRAS=y ;; --no-extras) WANT_EXTRAS=n ;;
    --services)   WANT_SERVICES=y ;; --no-services) WANT_SERVICES=n ;;
    --kmscon)     WANT_KMSCON=y ;; --no-kmscon) WANT_KMSCON=n ;;
    --console)    WANT_CONSOLE=y ;;   --no-console)   WANT_CONSOLE=n ;;
    --autologin)  WANT_AUTOLOGIN=y ;; --no-autologin) WANT_AUTOLOGIN=n ;;
    --autostart)  WANT_AUTOSTART=y ;; --no-autostart) WANT_AUTOSTART=n ;;
    --vt)         shift; VT="${1:-}" ;;
    --tty1)       WANT_CONSOLE=y; WANT_AUTOLOGIN=y; WANT_AUTOSTART=y; VT=1 ;;
    --no-tty1)    WANT_CONSOLE=n; WANT_AUTOLOGIN=n; WANT_AUTOSTART=n ;;
    --uninstall)  UNINSTALL=1 ;;
    --purge)      PURGE=1 ;;
    --dry-run)    DRY_RUN=1 ;;
    -h|--help)    usage 0 ;;
    *) die "unknown option: $1 (see --help)" ;;
  esac
  shift
done
case "$PROFILE" in ""|gui|console|appliance) ;; *) die "--profile must be gui, console or appliance" ;; esac
case "$VT" in ""|[1-9]|[1-5][0-9]|6[0-3]) ;; *) die "--vt must be a VT number 1-63" ;; esac

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
wm_bin() { command -v tetron-wm 2>/dev/null || echo "${TETRON_WM_BIN_DIR:-/usr/local/bin}/tetron-wm"; }

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

# kmscon supports truecolor but does not set COLORTERM, so tetron-wm would
# auto-downsample subtle themes (nord → teal/navy). Force truecolor on in
# config.toml for the kmscon appliance (top level, before any [table] header).
ensure_truecolor() {
  local f="$HOME/.config/tetron-wm/config.toml"
  if [ ! -f "$f" ]; then
    mkdir -p "$(dirname "$f")"; printf 'truecolor = true\n' > "$f"
    log "created config.toml with truecolor = true (kmscon)"; return 0
  fi
  grep -qE '^[[:space:]]*truecolor[[:space:]]*=' "$f" && return 0
  local tmp; tmp="$(mktemp)"
  awk 'BEGIN{ins=0} /^\[/ && ins==0 {print "truecolor = true"; ins=1} {print} END{if(ins==0) print "truecolor = true"}' "$f" > "$tmp" \
    && mv "$tmp" "$f" && log "set truecolor = true in config.toml (kmscon supports it; no COLORTERM)"
}

# ── Step 2b: recommended extras and system services ────────────────────────────
# Each entry is "command:debian-pkg:fedora-pkg:arch-pkg:brew-pkg" (an empty
# field = not offered there). Only entries whose command is missing install,
# so re-running converges. What each one is for: the DEPENDENCIES doc.
EXTRAS="chafa:chafa:chafa:chafa:chafa
sshpass:sshpass:sshpass:sshpass:
htop:htop:htop:htop:htop
btop:btop:btop:btop:btop
upower:upower:upower:upower:
alsamixer:alsa-utils:alsa-utils:alsa-utils:
blueutil::::blueutil"
# Desktop clipboard sync, only on a machine with a desktop session.
EXTRAS_GUI="xclip:xclip:xclip:xclip:
wl-copy:wl-clipboard:wl-clipboard:wl-clipboard:"
SERVICES="wpctl:wireplumber:wireplumber:wireplumber:
nmcli:network-manager:NetworkManager:networkmanager:
bluetoothctl:bluez:bluez:bluez-utils:"

# The package manager column to read: 2 apt, 3 dnf, 4 pacman, 5 brew.
pm_col() {
  if have apt-get; then echo 2; elif have dnf; then echo 3; elif have pacman; then echo 4
  elif have brew; then echo 5; else echo 0; fi
}

# Packages from a list (above) whose command is missing, for this package manager.
missing_pkgs() {
  local col line cmd pkg out=""; col="$(pm_col)"
  [ "$col" = 0 ] && return 0
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    cmd="${line%%:*}"; pkg="$(printf '%s' "$line" | cut -d: -f"$col")"
    if [ -n "$pkg" ] && ! have "$cmd"; then out="$out $pkg"; fi
  done <<EOF_LIST
$1
EOF_LIST
  printf '%s' "${out# }"
}

pm_install() {  # $1 = space-separated packages
  [ -n "$1" ] || return 0
  log "installing: $1"
  # shellcheck disable=SC2086
  case "$(pm_col)" in
    2) $SUDO apt-get install -y $1 ;;
    3) $SUDO dnf install -y $1 ;;
    4) $SUDO pacman -S --needed --noconfirm $1 ;;
    5) brew install $1 ;;
  esac || warn "some packages did not install: $1 (see docs/DEPENDENCIES_tetron-wm_recommended_installs.md)"
}

extras_list() {
  local l="$EXTRAS"
  is_graphical && l="$l
$EXTRAS_GUI"
  missing_pkgs "$l"
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

# ── Console state (so --uninstall and re-runs know what we changed) ────────────
STATE="$HOME/.config/tetron-wm/suite-state"
state_get() { [ -f "$STATE" ] && sed -n "s/^$1=//p" "$STATE" | tail -1; }
state_set() {  # $1=key $2=value (replaces any previous value)
  mkdir -p "$(dirname "$STATE")"
  { [ -f "$STATE" ] && grep -v "^$1=" "$STATE"; echo "$1=$2"; } > "$STATE.new" 2>/dev/null || true
  mv "$STATE.new" "$STATE"
}

OVR_MARK='# managed by install-tetron-wm-suite.sh'
SNIP_BEGIN='# >>> tetron-wm autostart (managed by install-tetron-wm-suite.sh) >>>'
SNIP_END='# <<< tetron-wm autostart <<<'

profile_file() {
  case "$(basename "${SHELL:-sh}")" in zsh) echo "$HOME/.zprofile" ;; *) echo "$HOME/.bash_profile" ;; esac
}

# Remove our autostart block from a profile file — the marked block, and the
# unmarked snippet older versions of this script wrote. No-op if absent.
strip_autostart() {
  local f="$1" tmp; [ -f "$f" ] || return 0
  grep -qE 'tetron-wm autostart|TETRON_WM_ACTIVE' "$f" || return 0
  tmp="$(mktemp)"
  awk -v b="$SNIP_BEGIN" -v e="$SNIP_END" '
    index($0,b)==1 {skip=1; marked=1}
    !skip && /^# Launch tetron-wm on the primary console/ {skip=1; legacy=1}
    skip {
      if (marked && index($0,e)==1) {skip=0; marked=0}
      else if (legacy && $0=="fi") {skip=0; legacy=0}
      next
    }
    {print}' "$f" > "$tmp" && mv "$tmp" "$f"
}

# ── Step 4a: kmscon on a VT (optionally with autologin) ────────────────────────
#   systemd → kmsconvt@ttyN (→ agetty --autologin USER, if autologin)
setup_console() {
  local vt="$1" autologin="$2"
  have systemctl || { warn "no systemd (systemctl) — skipping console setup"; return 0; }
  have kmscon    || { warn "kmscon is not installed — run the kmscon step first; skipping console"; return 0; }

  warn "This edits the login console on tty$vt. Keep a second VT (Ctrl+Alt+F2) or an"
  warn "SSH session open so a mistake can never lock you out. To undo later:"
  warn "  install-tetron-wm-suite.sh --uninstall"
  confirm "Run kmscon on tty$vt in place of the kernel login prompt?" y || { log "skipping console setup"; return 0; }

  # TERM must name what kmscon emulates (xterm-256color). Do NOT use $TERM here:
  # in a systemd unit it expands to the console's "linux", which makes tetron-wm
  # think it is on a kernel VT — no truecolor, mouse via gpm, clunky rendering.
  local user kbin term='xterm-256color'
  user="$(logname 2>/dev/null || echo "${SUDO_USER:-$USER}")"
  kbin="$(command -v kmscon)"

  log "1/2 point tty$vt at kmscon instead of the kernel getty"
  $SUDO systemctl disable --now "getty@tty$vt" 2>/dev/null || true
  $SUDO systemctl enable  --now "kmsconvt@tty$vt"

  local ovrd="/etc/systemd/system/kmsconvt@tty$vt.service.d"
  if [ "$autologin" = y ]; then
    log "2/2 autologin $user on tty$vt (drop-in override)"
    $SUDO mkdir -p "$ovrd"
    # %%I -> %I (systemd instance). The final arg is the login TERM: a literal
    # xterm-256color (what kmscon emulates), NOT $TERM — see the note above.
    { printf '%s\n' "$OVR_MARK"
      printf '[Service]\nExecStart=\nExecStart=%s "--vt=%%I" --seats=seat0 --no-switchvt --login -- /sbin/agetty --autologin %s --noclear - %s\n' \
        "$kbin" "$user" "$term"; } | $SUDO tee "$ovrd/override.conf" >/dev/null
  else
    log "2/2 normal login prompt on tty$vt (no autologin)"
    # Converge: drop an autologin override a previous run left behind.
    if $SUDO grep -qF "$OVR_MARK" "$ovrd/override.conf" 2>/dev/null; then
      $SUDO rm -f "$ovrd/override.conf"; $SUDO rmdir "$ovrd" 2>/dev/null || true
    fi
  fi
  # Under kmscon the mouse comes from kmscon itself; a running gpm is redundant
  # and can fight it for the input device. Disable it on the console.
  if have gpm && systemctl is-enabled gpm >/dev/null 2>&1; then
    $SUDO systemctl disable --now gpm >/dev/null 2>&1 && { log "disabled gpm (kmscon provides the mouse)"; state_set gpm_disabled 1; } || true
  fi
  $SUDO systemctl daemon-reload
  $SUDO systemctl restart "kmsconvt@tty$vt"
  state_set vt "$vt"; state_set autologin "$autologin"
  ensure_truecolor
  log "tty$vt is set. Reboot (or switch with Ctrl+Alt+F$vt) to see it."
}

# ── Step 4b: autostart tetron-wm on kmscon login ───────────────────────────────
setup_autostart() {
  local vt="$1" wm wmdir prof; wm="$(wm_bin)"; wmdir="$(dirname "$wm")"; prof="$(profile_file)"
  [ -x "$wm" ] || { warn "tetron-wm not found on PATH — install it first; skipping autostart"; return 0; }
  strip_autostart "$prof"   # converge: replace any earlier (marked or legacy) block
  # Launch only on the chosen VT. kmscon runs the login shell on a PTS, not
  # /dev/ttyN, so a `tty = /dev/ttyN` test never matches under kmscon; walk the
  # process tree up to the kmscon process and read its --vt=ttyN argument
  # instead (also accept the raw VT). Exclude SSH. Absolute binary path
  # + PATH prepend: this profile runs before the interactive rc, where the
  # install dir may not be on PATH yet. `$wm`/`$wmdir` expand now; the rest stays
  # literal for login time.
  cat >> "$prof" <<EOF

$SNIP_BEGIN
# Launch tetron-wm on tty$vt (kmscon, or the raw VT), not over SSH, in nested
# shells, or on other VTs. Drop \`exec\` to land on a shell when you quit.
if [ -z "\$TETRON_WM_ACTIVE" ] && [ -z "\$SSH_CONNECTION" ]; then
  _tw=0
  [ "\$(tty)" = "/dev/tty$vt" ] && _tw=1
  _p=\$PPID
  while [ "\$_tw" = 0 ] && [ "\${_p:-0}" -gt 1 ]; do
    case "\$(cat /proc/\$_p/comm 2>/dev/null)" in
      kmscon) case "\$(tr '\\0' ' ' </proc/\$_p/cmdline 2>/dev/null)" in *"--vt=tty$vt "*) _tw=1 ;; esac ;;
    esac
    _p=\$(awk '/^PPid:/{print \$2}' /proc/\$_p/status 2>/dev/null)
  done
  if [ "\$_tw" = 1 ]; then
    export TETRON_WM_ACTIVE=1
    export PATH="$wmdir:\$PATH"
    exec "$wm"
  fi
  unset _tw _p
fi
$SNIP_END
EOF
  state_set autostart_profile "$prof"; state_set autostart_vt "$vt"
  ensure_truecolor
  # Keep apps alive across UI reloads/updates by running the apphost as a service.
  "$wm" service install >/dev/null 2>&1 && { log "installed the tetron-wm apphost service"; state_set apphost_service 1; } || true
  log "autostart set in $prof (tetron-wm starts on tty$vt only)"
}

# ── Uninstall ──────────────────────────────────────────────────────────────────
do_uninstall() {
  printf '%stetron-wm suite uninstall%s\n\n' "$B" "$N"
  local wm; wm="$(wm_bin)"

  # 1. autostart snippet
  local prof; for prof in "$HOME/.zprofile" "$HOME/.bash_profile"; do
    if [ -f "$prof" ] && grep -qE 'tetron-wm autostart|TETRON_WM_ACTIVE' "$prof"; then
      if confirm "Remove the tetron-wm autostart block from $prof?" y; then strip_autostart "$prof"; log "removed autostart from $prof"; fi
    fi
  done

  # 2. console: every VT we set up (state file), else any marked override
  local v vts=""; vts="$(state_get vt)"
  local d; for d in /etc/systemd/system/kmsconvt@tty*.service.d; do
    [ -d "$d" ] || continue
    if $SUDO grep -qF "$OVR_MARK" "$d/override.conf" 2>/dev/null; then
      v="${d##*kmsconvt@tty}"; v="${v%.service.d}"
      case " $vts " in *" $v "*) ;; *) vts="$vts $v" ;; esac
    fi
  done
  for v in $vts; do
    if confirm "Give tty$v back to the normal login prompt (stop kmscon there)?" y; then
      $SUDO rm -f "/etc/systemd/system/kmsconvt@tty$v.service.d/override.conf"
      $SUDO rmdir "/etc/systemd/system/kmsconvt@tty$v.service.d" 2>/dev/null || true
      $SUDO systemctl daemon-reload
      $SUDO systemctl disable --now "kmsconvt@tty$v" 2>/dev/null || true
      $SUDO systemctl enable  --now "getty@tty$v" 2>/dev/null || true
      log "tty$v restored to getty"
    fi
  done
  if [ "$(state_get gpm_disabled)" = 1 ] && have gpm; then
    confirm "Re-enable gpm (the raw-console mouse daemon we disabled)?" y \
      && $SUDO systemctl enable --now gpm >/dev/null 2>&1 && log "gpm re-enabled"
  fi

  # 3. apphost service
  if [ -x "$wm" ] && confirm "Remove the tetron-wm apphost service?" y; then
    "$wm" service uninstall >/dev/null 2>&1 && log "removed the apphost service" || true
  fi

  rm -f "$STATE"

  # 4. optional: binary + config
  if [ "$PURGE" = 1 ]; then
    if [ -x "$wm" ] && confirm "Remove the tetron-wm binary ($wm)?" n; then
      rm -f "$wm" 2>/dev/null || $SUDO rm -f "$wm"; log "removed $wm"
    fi
    if [ -d "$HOME/.config/tetron-wm" ] && confirm "Delete ~/.config/tetron-wm (config, themes, wallpapers)?" n; then
      rm -rf "$HOME/.config/tetron-wm"; log "removed ~/.config/tetron-wm"
    fi
  else
    log "kept the binary, ~/.config/tetron-wm and kmscon (add --purge to remove the first two)"
  fi
  log "done."
}

# ── Interactive choices (the same options as the flags) ─────────────────────────
HAVE_TTY=0; [ "$ASSUME_YES" != 1 ] && [ -r /dev/tty ] && HAVE_TTY=1

# Pick from a numbered menu; default on Enter. $1=question, $2=default number,
# rest = options. Sets CHOICE to the chosen number.
choose() {
  local q="$1" def="$2" n=0 o ans; shift 2
  printf '%s%s%s\n' "$B" "$q" "$N" >&2
  for o in "$@"; do n=$((n+1)); printf '  %d) %s\n' "$n" "$o" >&2; done
  printf '%sChoice [%s]: %s' "$B" "$def" "$N" >&2
  read -r ans </dev/tty || ans=""
  ans="${ans:-$def}"
  case "$ans" in [1-9]) [ "$ans" -le "$n" ] && { CHOICE="$ans"; return 0; } ;; esac
  warn "not a valid choice — using $def"; CHOICE="$def"
}

# Ask for the VT number (default 1) unless set by --vt / non-interactive.
resolve_vt() {
  [ -n "$VT" ] && return 0
  VT=1
  [ "$HAVE_TTY" = 1 ] || return 0
  local ans; printf '%sWhich VT? tty1 opens at boot [1]: %s' "$B" "$N" >&2
  read -r ans </dev/tty || ans=""
  case "${ans:-1}" in [1-9]|[1-5][0-9]|6[0-3]) VT="${ans:-1}" ;; *) warn "not a VT number — using 1" ;; esac
}

# Is this a machine with a desktop session (so kmscon is not what you want)?
is_graphical() {
  [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ] && return 0
  case "${XDG_SESSION_TYPE:-}" in x11|wayland) return 0 ;; esac
  have systemctl && [ "$(systemctl get-default 2>/dev/null)" = "graphical.target" ]
}

apply_profile() {  # set any WANT_ flag the user has not already set on the command line
  local k v
  set_default() { eval "[ -n \"\$$1\" ] || $1=$2"; }
  case "$1" in
    gui)       set_default WANT_KMSCON n; set_default WANT_CONSOLE n; set_default WANT_AUTOLOGIN n; set_default WANT_AUTOSTART n ;;
    console)   set_default WANT_KMSCON y; set_default WANT_CONSOLE y; set_default WANT_AUTOLOGIN n; set_default WANT_AUTOSTART n ;;
    appliance) set_default WANT_KMSCON y; set_default WANT_CONSOLE y; set_default WANT_AUTOLOGIN y; set_default WANT_AUTOSTART y ;;
  esac
}

# ── Orchestration ──────────────────────────────────────────────────────────────
main() {
  if [ "$UNINSTALL" = 1 ]; then do_uninstall; return; fi
  printf '%stetron-wm suite installer%s  (repo: %s, branch: %s)\n\n' "$B" "$N" "$REPO" "$BRANCH"

  local gui_default=2
  if [ "$IS_LINUX" = 1 ]; then
    is_graphical && gui_default=1 || gui_default=3
    # Menu only when no profile and nothing console-related was given as a flag.
    if [ -z "$PROFILE" ] && [ "$HAVE_TTY" = 1 ] && [ -z "$WANT_KMSCON$WANT_CONSOLE$WANT_AUTOLOGIN$WANT_AUTOSTART" ]; then
      choose "What are you setting up?" "$gui_default" \
        "A desktop machine: tetron-wm in a terminal window  (gui)" \
        "A VC tty with kmscon, normal login prompt, start tetron-wm yourself  (console)" \
        "A console appliance: kmscon + autologin + tetron-wm starts at boot  (appliance)" \
        "Custom: ask me about each piece"
      case "$CHOICE" in 1) PROFILE=gui ;; 2) PROFILE=console ;; 3) PROFILE=appliance ;; esac
    fi
    [ -n "$PROFILE" ] && apply_profile "$PROFILE"
  else
    PROFILE=gui; apply_profile gui
  fi

  resolve WANT_WM     "Install tetron-wm (latest release)?" y
  resolve WANT_CONFIG "Set up ~/.config/tetron-wm/ (config.toml, themes/, wallpapers/)?" y
  # Only ask about packages that are actually missing; nothing to ask = skip.
  local extras services=""
  extras="$(extras_list)"
  [ "$IS_LINUX" = 1 ] && services="$(missing_pkgs "$SERVICES")"
  if [ -n "$extras" ]; then
    resolve WANT_EXTRAS "Install recommended tools ($extras)?" y
  else
    WANT_EXTRAS=n
  fi
  if [ -n "$services" ]; then
    resolve WANT_SERVICES "Install system services for the tray ($services)? They change how this machine manages sound, network or Bluetooth" n
  else
    WANT_SERVICES=n
  fi
  if [ "$IS_LINUX" = 1 ]; then
    local kdef=y; is_graphical && kdef=n
    resolve WANT_KMSCON "Install kmscon (truecolor + mouse console; compiles from source except on Debian trixie)?" "$kdef"
    [ "$WANT_AUTOLOGIN" = y ] && [ -z "$WANT_CONSOLE" ] && WANT_CONSOLE=y
    resolve WANT_CONSOLE   "Run kmscon on a VT in place of the kernel login prompt?" n
    if [ "$WANT_CONSOLE" = y ]; then
      resolve_vt
      resolve WANT_AUTOLOGIN "Log in automatically on tty$VT (no password prompt)?" n
    else
      [ -z "$WANT_AUTOLOGIN" ] && WANT_AUTOLOGIN=n
    fi
    resolve WANT_AUTOSTART "Start tetron-wm automatically when you log in on a kmscon console?" n
    [ "$WANT_AUTOSTART" = y ] && resolve_vt
  else
    WANT_KMSCON=n; WANT_CONSOLE=n; WANT_AUTOLOGIN=n; WANT_AUTOSTART=n
  fi
  [ -n "$VT" ] || VT=1

  echo
  if [ "$DRY_RUN" = 1 ]; then
    log "plan (dry run, nothing changed): wm=$WANT_WM config=$WANT_CONFIG extras=$WANT_EXTRAS${extras:+ ($extras)} services=$WANT_SERVICES${services:+ ($services)} kmscon=$WANT_KMSCON console=$WANT_CONSOLE vt=$VT autologin=$WANT_AUTOLOGIN autostart=$WANT_AUTOSTART"
    return 0
  fi
  [ "$WANT_WM" = y ]        && install_wm
  [ "$WANT_CONFIG" = y ]    && seed_config
  [ "$WANT_EXTRAS" = y ]    && pm_install "$extras"
  [ "$WANT_SERVICES" = y ]  && pm_install "$services"
  [ "$WANT_KMSCON" = y ]    && install_kmscon
  [ "$WANT_CONSOLE" = y ]   && setup_console "$VT" "$WANT_AUTOLOGIN"
  [ "$WANT_AUTOSTART" = y ] && setup_autostart "$VT"

  echo
  log "done."
  if [ "$WANT_AUTOSTART" != y ]; then
    log "run it with:  tetron-wm   (install.sh noted the PATH if the dir is not already on it)"
  fi
}

main "$@"
