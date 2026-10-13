# Systray target design: network and battery popovers

Companion to `REFERENCE_tetron-wm_systray_items_popovers_visibility.md` (current behavior). This file is the desired state. Cells marked **CHANGED** or **NEW** differ from today; everything else is unchanged.

| Item | Icon | Label | Popover contents | Popover links do | Appears when | Label logic |
|---|---|---|---|---|---|---|
| CPU | ⊙ | `⊙ 12%` | "CPU", `12% load` | `▸ Open htop` (or `top`) in a window | Always | Rounded `cpu_pct` |
| Memory | ▤ | `▤ 41%` | "Memory", `41% used` | `▸ Open free -h` (or `vm_stat` on macOS) in a window, held until Enter | Always | `mem_pct(used, total)` |
| Battery | ▯ (↯ when charging) | `▯ 87%` | "Battery", `87%` plus ↯ if charging. **NEW:** time remaining or time to full when known. | **CHANGED:** `▸ Open jolt` if installed, else `upower -i <first BAT device from upower -e>` in a window held until Enter, else on macOS `pmset -g batt` held until Enter. No row if none exist. | `state.battery` is `Some` (hidden on desktops with no battery) | Percent, glyph by charging state |
| Bluetooth | ≋ | `≋ bt` | `Bluetooth [on]/[off]`, up to 6 paired devices (● connected, ○ not), `(no paired devices)`, or `install blueutil to control` | Click `[on]/[off]` to toggle power. Click a device to connect or disconnect. | `caps.bluetooth` or Bluetooth is enabled | Fixed text `bt` |
| Network (was WiFi) | WiFi signal bars (`bars_glyph`) on WiFi, `⇄` on wired. | **CHANGED:** `▂▄▆ wifi` on WiFi, `⇄ net` on wired. No SSID (the popover carries it). | **CHANGED:** header rows `SSID` (WiFi only), `IP (interface)`, then `Wi-Fi [on]/[off]` and up to 6 known networks (● current). **NEW:** with no WiFi, show the primary active interface and its IP (default-route interface). With no link at all, show `Offline`. | Click `[on]/[off]` to toggle WiFi. Click a network to connect. **NEW:** click the IP row to copy it to the clipboard. **NEW:** `▸ Open <tool>`: first installed of `wlctl`, `nmtui`, `iwctl`, else `ip -br addr` held until Enter. On macOS `networksetup -listallhardwareports` then `ifconfig`, held until Enter. | **CHANGED:** whenever any interface has a default route (WiFi, ethernet, tether, VPN), not only WiFi | Active link type: WiFi gives bars plus `wifi`, anything else (ethernet, tether, VPN) gives `⇄` plus `net`. Bars follow signal strength. |
| Tetron | ⇄ | `⇄ 3 ●` | `Tetron [on]/[off]`, one row per network (up to 8), or `(no networks)` | Click `[on]/[off]` to toggle all networks. Click a network row to toggle it. `⌨ Open terminal` opens `tetron status` and a shell. | Only when the tetron daemon answers | Connected peers, then ● direct, ○ relay, or `off` on standby |
| Select | none | `SELECT` | None | None | `select_mode` is on | Fixed text |
| Clipboard | ⎘ | `⎘ 12` or `⎘ ‖` | None, opens the `ClipMenu` overlay | The overlay handles it | Clipboard history is enabled | Entry count, or `‖` when paused |
| Bell | ⍾ | `⍾ 2` | "Notifications", up to 8 rows, `(none)`, `clear all` | Click a row to focus that window. Click `clear all` to dismiss all. | `bells > 0` | Pending notification count |
| Clock | none | `09:41` | Month grid with ◂ ▸, today highlighted, event days underlined, up to 3 khal events, footer `time  up Nh` | ◂ ▸ change month | Always | Time only |
| Volume | ♪ low, ♫ high, ✗ muted or 0 | `♫  75` | `Volume 75`, bar `◂ ▮▮▮▮▯▯ ▸`, speaker glyph | ◂ down, ▸ up, speaker toggles mute, scroll changes volume | Always, far right | Glyph by level, level padded to 3 wide |

## Implementation notes

- One helper picks the first installed command from an ordered candidate list and returns the window action (title, label, command, args, held or not). CPU, Memory, Battery and Network all use it. Extends `segment_app`, which already takes an `installed` closure so tests can stub `$PATH`.
- Network facts (SSID, interface, IP, link type) are gathered on the poller thread via `crate::system::run_capped`, never in the render path. Linux: `ip -4 route get 1.1.1.1` gives `dev` and `src`. macOS: `route -n get default` for the interface, `ipconfig getifaddr <if>` for the IP.
- Public IP is out of scope: it needs the network and would violate the never-block-render rule.
- Dropping the SSID from the label removes `WIFI_LABEL_MAX`. Shedding order is unchanged except the WiFi entry becomes Network.
- Rename `SegmentKind::Wifi` to `Network` only if the protocol or tests do not depend on the name; otherwise keep the variant and change the behavior.
- Battery device lookup uses `upower -e` rather than hardcoding `BAT0`.
