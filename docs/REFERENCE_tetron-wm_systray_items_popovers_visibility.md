# Systray items: icon, label, popover, links, visibility, label logic

Source: `src/tray.rs` (`tray_segments_with`, `render_*`, `segment_app`) and `src/iconset.rs`. Icons shown are the default ASCII set; Nerd Font and Emoji sets are selectable in `src/iconset.rs`. Items are listed left to right.

| Item | Icon | Label | Popover contents | Popover links do | Appears when | Label logic |
|---|---|---|---|---|---|---|
| CPU | ⊙ | `⊙ 12%` | "CPU", `12% load` | `▸ Open htop` (or `top`) runs in a window | Always | Rounded `cpu_pct` |
| Memory | ▤ | `▤ 41%` | "Memory", `41% used` | `▸ Open free -h` (or `vm_stat` on macOS) runs in a window and waits for Enter. No row if neither exists. | Always | `mem_pct(used, total)` |
| Battery | ▯ (↯ when charging) | `▯ 87%` | "Battery", `87%` plus ↯ if charging, or `no battery` | `▸ Open jolt` runs in a window. No row without `jolt`. | `state.battery` is `Some` | Percent, glyph by charging state |
| Bluetooth | ≋ | `≋ bt` | `Bluetooth [on]/[off]`, up to 6 paired devices (● connected, ○ not), `(no paired devices)`, or `install blueutil to control` | Click `[on]/[off]` to toggle power. Click a device to connect or disconnect. | `caps.bluetooth` or Bluetooth is enabled | Fixed text `bt` |
| WiFi | Signal bars (`bars_glyph`) | `▂▄▆ Home-5` | `Wi-Fi [on]/[off]`, up to 6 known networks (● marks the current one) | Click `[on]/[off]` to toggle WiFi. Click a network to connect (`WifiConnectKnown`). | `state.wifi` is `Some` | Bars by signal, then the SSID cut to 6 characters (`WIFI_LABEL_MAX`), or `wifi` if the SSID is empty |
| Tetron | ⇄ | `⇄ 3 ●` | `Tetron [on]/[off]`, one row per network (up to 8), or `(no networks)` | Click `[on]/[off]` to toggle all networks. Click a network row to toggle that network. `⌨ Open terminal` opens a terminal with `tetron status` and a shell. | Only when the tetron daemon answers (`tetron.reachable`) | Total connected peers across networks, then ● if any is direct, ○ if all relayed, or `off` if standby |
| Select | none | `SELECT` | None | None | `select_mode` is on | Fixed text |
| Clipboard | ⎘ | `⎘ 12` or `⎘ ‖` | None. A click opens the session `ClipMenu` overlay instead (`toggle_clipmenu`). | The overlay handles it | Clipboard history is enabled | Entry count, or `‖` when paused |
| Bell | ⍾ | `⍾ 2` | "Notifications", up to 8 rows (`time text`), `(none)`, and `clear all` | Click a row to focus that window. Click `clear all` to dismiss all. | `bells > 0` | Pending notification count |
| Clock | none | `09:41` | Month grid with ◂ ▸ navigation, today highlighted, event days underlined, up to 3 upcoming events from khal, footer `time  up Nh`. Before the first poll it shows the date and uptime as plain text. | ◂ and ▸ change the month. Events are display only. | Always (never dropped) | Time only (`HH:MM`) |
| Volume | ♪ low (<66), ♫ high, ✗ muted or 0 | `♫  75` | `Volume 75 (muted)`, bar `◂ ▮▮▮▮▯▯ ▸`, speaker glyph | ◂ lowers volume, ▸ raises it, the speaker toggles mute. Scrolling on the segment also changes volume. | Always (never dropped), far right past the clock | Glyph by level or mute, then the level padded to 3 wide so the width never shifts |

## Space shedding

When the menubar is too narrow, segments are dropped in this order until the rest fit: CPU, Memory, Battery, Bell, Clipboard, Tetron, WiFi, Bluetooth. Select, Clock and Volume are never dropped.

## Gaps against the proposed ideas

- The WiFi popover has no IP, interface or Open row, and shows no active interface when WiFi is absent.
- Battery has only the `jolt` Open row, with no `upower` or macOS fallback.
