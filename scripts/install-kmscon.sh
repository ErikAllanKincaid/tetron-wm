#!/usr/bin/env bash
# install-kmscon.sh — install the truecolor + mouse kmscon console for tetron-wm.
#
# tetron-wm runs on a bare Linux VT (no X/Wayland) through kmscon, a KMS/DRM
# userspace console. It needs the MAINTAINED line of kmscon (Aetf's fork, which
# carries truecolor + mouse-report passthrough) rather than the stock console
# that general distros still ship. Debian packages the maintained line in
# trixie-backports (numbered kmscon 10.x / libtsm4 4.7.x there); upstream the
# same code is Aetf/kmscon + Aetf/libtsm, whose own releases are numbered 9.x /
# 4.x. We treat kmscon >= 9.1 (the maintained fork) as adequate.
#
# This script only INSTALLS that pair. It does NOT touch your getty, VTs, or
# autologin — so it is safe to run on a normal GUI machine where you just want a
# truecolor VC tty to switch to. Making kmscon your login console is a separate,
# deliberate step (see the note the script prints at the end, and tetron-os's
# firstboot.sh).
#
#   Debian (trixie): kmscon 10 + libtsm4 4.7.1 are in trixie-backports — a clean
#                    apt install. The script enables backports (idempotent).
#   Ubuntu/Mint:     apt only has the too-old pair, so the script BUILDS kmscon
#                    10 + libtsm 4.7.1 from the upstream source (Aetf/kmscon,
#                    Aetf/libtsm) — behind a confirmation prompt, since that
#                    pulls a compiler toolchain and compiles two projects.
#
# Usage:  install-kmscon.sh [-y] [--source] [--apt] [--no-fonts] [-h]
#   -y, --yes     do not prompt (assume yes to the source build)
#   --source      force the from-source build even where apt would do
#   --apt         only try apt; never build from source
#   --no-fonts    skip installing console fonts (fonts-terminus, DejaVu)
#   -h, --help    show this help
#
# Env overrides (advanced): PREFIX (default /usr/local), KMSCON_REPO/KMSCON_REF,
# LIBTSM_REPO/LIBTSM_REF.
set -euo pipefail

# ── Config ───────────────────────────────────────────────────────────────────
PREFIX="${PREFIX:-/usr/local}"
KMSCON_REPO="${KMSCON_REPO:-Aetf/kmscon}"
LIBTSM_REPO="${LIBTSM_REPO:-Aetf/libtsm}"
KMSCON_REF="${KMSCON_REF:-}"   # empty = resolve latest release, else default branch
LIBTSM_REF="${LIBTSM_REF:-}"
# Minimum acceptable kmscon version. 9.1 is the maintained fork's current line
# (truecolor + mouse); Debian's backports package (10.x) compares >= this, while
# the stock 9.0.0 some distros ship compares below it and triggers a rebuild.
MIN_KMSCON="9.1"

ASSUME_YES=0; FORCE_SOURCE=0; APT_ONLY=0; WANT_FONTS=1

# Fonts with full block/box glyphs so tetron-wm's half-block UI renders on kmscon.
FONT_PKGS="fonts-dejavu-core fonts-terminus"

# ── Output helpers ─────────────────────────────────────────────────────────────
if [ -t 1 ]; then B=$'\033[1m'; Y=$'\033[33m'; R=$'\033[31m'; G=$'\033[32m'; N=$'\033[0m'; else B= Y= R= G= N=; fi
log()  { printf '%s==>%s %s\n'  "$G" "$N" "$*"; }
warn() { printf '%sWARN:%s %s\n' "$Y" "$N" "$*" >&2; }
die()  { printf '%sERROR:%s %s\n' "$R" "$N" "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

usage() { sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//; s/^#$//' | sed '$d'; exit "${1:-0}"; }

# ── Args ───────────────────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
	case "$1" in
		-y|--yes) ASSUME_YES=1 ;;
		--source) FORCE_SOURCE=1 ;;
		--apt)    APT_ONLY=1 ;;
		--no-fonts) WANT_FONTS=0 ;;
		-h|--help) usage 0 ;;
		*) die "unknown option: $1 (see --help)" ;;
	esac
	shift
done
[ "$FORCE_SOURCE" = 1 ] && [ "$APT_ONLY" = 1 ] && die "--source and --apt are mutually exclusive"

# ── Privilege: run apt / make install through sudo when not already root ────────
if [ "$(id -u)" -eq 0 ]; then SUDO=""; else
	have sudo || die "need root or sudo to install packages"
	SUDO="sudo"
fi

# ── Distro detection ────────────────────────────────────────────────────────────
[ -r /etc/os-release ] || die "cannot read /etc/os-release — unsupported system"
# shellcheck disable=SC1091
. /etc/os-release
ID="${ID:-}"; ID_LIKE="${ID_LIKE:-}"; CODENAME="${VERSION_CODENAME:-}"

is_in() { case " $2 " in *" $1 "*) return 0;; *) return 1;; esac; }

# debian-family (apt + .deb) vs the ubuntu-family (ubuntu, mint, pop, …).
FAMILY=""
case "$ID" in
	debian) FAMILY="debian" ;;
	ubuntu|linuxmint|pop|elementary|zorin|neon|tuxedo) FAMILY="ubuntu" ;;
	*)
		if is_in ubuntu "$ID_LIKE"; then FAMILY="ubuntu"
		elif is_in debian "$ID_LIKE"; then FAMILY="debian"
		fi ;;
esac
[ -n "$FAMILY" ] || die "unsupported distro '$ID' (need a Debian- or Ubuntu-family system)"
log "detected: ${PRETTY_NAME:-$ID $VERSION_ID}  (family: $FAMILY, codename: ${CODENAME:-?})"

# ── Version helpers ─────────────────────────────────────────────────────────────
ver_ge() { dpkg --compare-versions "$1" ge "$2"; }

# kmscon ships NO --version flag, so we cannot read a version from the binary.
# Decide adequacy by provenance instead: a dpkg-managed kmscon must be a new
# enough PACKAGE; a kmscon that no package owns (e.g. a /usr/local source build
# of the maintained fork) is taken as adequate.
kmscon_owner_pkg() {   # echoes the dpkg package owning `kmscon`, or nothing
	have kmscon || return 0
	dpkg -S "$(readlink -f "$(command -v kmscon)")" 2>/dev/null | cut -d: -f1
}
kmscon_pkg_version() { # bare upstream version of a package (epoch/revision stripped)
	dpkg-query -W -f='${Version}' "$1" 2>/dev/null | sed -E 's/^[0-9]+://; s/[-~].*//'
}
kmscon_adequate() {
	have kmscon || return 1
	local pkg; pkg="$(kmscon_owner_pkg)"
	if [ -n "$pkg" ]; then
		local v; v="$(kmscon_pkg_version "$pkg")"
		[ -n "$v" ] && ver_ge "$v" "$MIN_KMSCON"
	else
		return 0   # not package-managed → a source build of the maintained fork
	fi
}
kmscon_describe() {    # human description of the installed kmscon
	local bin pkg v; bin="$(command -v kmscon 2>/dev/null)" || { echo "not installed"; return; }
	pkg="$(kmscon_owner_pkg)"
	if [ -n "$pkg" ]; then v="$(kmscon_pkg_version "$pkg")"; echo "$bin ($pkg $v, apt)"
	else echo "$bin (built from source, maintained fork)"; fi
}

# ── Fonts ────────────────────────────────────────────────────────────────────────
install_fonts() {
	[ "$WANT_FONTS" = 1 ] || return 0
	log "installing console fonts ($FONT_PKGS)"
	$SUDO apt-get install -y --no-install-recommends $FONT_PKGS
}

# ── Debian path: kmscon 10 + libtsm4 4.7.1 from trixie-backports ────────────────
enable_backports() {
	local list=/etc/apt/sources.list.d/trixie-backports.sources
	if [ -f "$list" ]; then log "trixie-backports already configured"; return 0; fi
	log "enabling trixie-backports (kmscon 10 + libtsm4 4.7.1 live there)"
	$SUDO tee "$list" >/dev/null <<-EOF
		Types: deb
		URIs: http://deb.debian.org/debian
		Suites: trixie-backports
		Components: main
		Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg
	EOF
	$SUDO apt-get update -qq
}

apt_install_debian() {
	[ "$CODENAME" = "trixie" ] || {
		warn "Debian '$CODENAME' is not trixie; backports may not carry kmscon $MIN_KMSCON."
		return 1
	}
	enable_backports
	log "installing kmscon + libtsm4 from trixie-backports"
	# -t pins just these to backports so libtsm4 4.7.1 is chosen to satisfy
	# kmscon 10 (plain trixie ships 4.0.2, which kmscon 10 rejects).
	$SUDO apt-get install -y -t trixie-backports kmscon libtsm4
	install_fonts
}

# ── Source build (Ubuntu/Mint, or forced): Aetf/libtsm + Aetf/kmscon ────────────
confirm_source() {
	if [ "$ASSUME_YES" = 1 ]; then
		log "building kmscon from source: installs a build toolchain (apt) and compiles libtsm + kmscon into $PREFIX"
		return 0
	fi
	cat >&2 <<-EOF
		${B}apt on this system only has the stock kmscon, which predates the
		truecolor + mouse support tetron-wm needs.${N}
		To get the maintained fork, this will:
		  • install a build toolchain + dev libraries (apt),
		  • compile and install libtsm ($LIBTSM_REPO) and kmscon ($KMSCON_REPO) into $PREFIX.
		Nothing else is changed (no getty / VT / autologin edits).
	EOF
	printf '%sProceed with the source build? [y/N] %s' "$B" "$N" >&2
	local a; read -r a </dev/tty || a=""
	case "$a" in y|Y|yes|YES) return 0;; *) return 1;; esac
}

install_build_deps() {
	log "refreshing apt index + installing build dependencies"
	# A stale index 404s on point-release deps (e.g. libpciaccess-dev), so update first.
	$SUDO apt-get update -qq
	# meson/ninja toolchain + the dev libs kmscon links (mirrors its runtime
	# Depends: drm, egl/gles, gbm, glib, pango, pixman, systemd, udev, xkbcommon).
	$SUDO apt-get install -y --no-install-recommends \
		build-essential meson ninja-build pkg-config git ca-certificates \
		libdrm-dev libegl-dev libgles-dev libgbm-dev libglib2.0-dev \
		libpango1.0-dev libpixman-1-dev libsystemd-dev libudev-dev libxkbcommon-dev
}

# Resolve the latest release tag of a GitHub repo via the (unauthenticated,
# not-rate-limited) releases/latest redirect; empty when the repo has no release.
latest_tag() {
	local loc
	loc="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$1/releases/latest" 2>/dev/null)" || return 0
	case "$loc" in */tag/*) printf '%s' "${loc##*/tag/}" ;; esac
}

# build_one <owner/repo> <ref-or-empty> <friendly-name> — clone + build + install.
build_one() {
	local repo="$1" ref="$2" name="$3" url="https://github.com/$1" dir="$BUILD/$3"
	if [ -z "$ref" ]; then ref="$(latest_tag "$repo" || true)"; fi
	if [ -n "$ref" ]; then
		log "cloning $name @ $ref"
		git clone --depth 1 --branch "$ref" "$url" "$dir" 2>/dev/null \
			|| { warn "tag '$ref' not found; cloning default branch"; git clone --depth 1 "$url" "$dir"; }
	else
		log "cloning $name (default branch)"
		git clone --depth 1 "$url" "$dir"
	fi
	cd "$dir"
	if [ -f meson.build ]; then
		log "building $name (meson)"
		# -Dc_args=-Wno-error: the tagged upstream builds with -Werror and trips
		# newer GCC's stricter format-truncation warnings (Debian carries the
		# fixes in its later package; we just stop treating warnings as errors).
		# -Dtests=false drops the unit-test subdir (needs libcheck); fall back to
		# a plain setup for a project that has no 'tests' option.
		local common=(--prefix="$PREFIX" --buildtype=release -Dc_args=-Wno-error)
		meson setup build "${common[@]}" -Dtests=false 2>/dev/null \
			|| { rm -rf build; meson setup build "${common[@]}"; }
		ninja -C build
		$SUDO ninja -C build install
	elif [ -x ./autogen.sh ] || [ -f configure.ac ]; then
		log "building $name (autotools)"
		[ -x ./autogen.sh ] && NOCONFIGURE=1 ./autogen.sh || true
		./configure --prefix="$PREFIX"
		make -j"$(nproc)"
		$SUDO make install
	else
		die "$name: no meson.build or autotools files — cannot build"
	fi
	cd - >/dev/null
}

source_build() {
	confirm_source || die "aborted (no changes made)."
	install_build_deps
	BUILD="$(mktemp -d)"; trap 'rm -rf "$BUILD"' EXIT
	# libtsm first; kmscon links against it. Point pkg-config/loader at $PREFIX so
	# the freshly built libtsm is found during and after the kmscon build.
	build_one "$LIBTSM_REPO" "$LIBTSM_REF" libtsm
	$SUDO ldconfig
	local m; m="$(uname -m)"
	export PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig:$PREFIX/lib/$m-linux-gnu/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
	export LD_LIBRARY_PATH="$PREFIX/lib:$PREFIX/lib/$m-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
	build_one "$KMSCON_REPO" "$KMSCON_REF" kmscon
	$SUDO ldconfig
	install_fonts
}

# ── Orchestration ────────────────────────────────────────────────────────────────
main() {
	if kmscon_adequate && [ "$FORCE_SOURCE" != 1 ]; then
		log "adequate kmscon already installed: $(kmscon_describe) — nothing to do."
		exit 0
	fi

	if [ "$FORCE_SOURCE" = 1 ]; then
		source_build
	elif [ "$FAMILY" = "debian" ] && apt_install_debian; then
		: # done via apt
	elif [ "$APT_ONLY" = 1 ]; then
		die "apt cannot provide kmscon >= $MIN_KMSCON here, and --apt forbids a source build."
	else
		source_build
	fi

	# Verify we actually ended up with an adequate kmscon.
	if kmscon_adequate; then
		log "done: $(kmscon_describe)"
	else
		warn "kmscon not found on PATH after install; check the log above."
	fi
	cat <<-EOF

	${B}kmscon is installed.${N} This script changed no login/VT settings.
	To try it now on a free VT (e.g. VT 3), from a root shell:
	    ${B}kmscon --vt 3 --xkb-layout us -- /bin/login${N}
	then switch with Ctrl+Alt+F3. To make kmscon your boot console, swap the
	getty for kmsconvt@ on a VT (see tetron-os firstboot.sh), e.g.:
	    ${B}systemctl disable --now getty@tty3 && systemctl enable --now kmsconvt@tty3${N}
	EOF
}

main "$@"
