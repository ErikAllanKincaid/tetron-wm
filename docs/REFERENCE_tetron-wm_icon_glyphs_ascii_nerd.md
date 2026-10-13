# Icon glyph sets: ascii and nerd

Source of truth: `src/iconset.rs`. The `icon_style` config key (`ascii` | `nerd` | `emoji`) picks the set for the menubar tray and for the file-type icons shown where Kitty graphics are not available. Where Kitty graphics are available, the desktop and file manager draw image icons and these glyphs sit underneath as the fallback.

The `ascii` set is Unicode geometric symbols from the Basic Multilingual Plane, not 7-bit ASCII. It renders in nearly every monospace font. The `nerd` set lives in the Private Use Area and shows as tofu unless a Nerd Font is installed.

## Menubar tray and status indicators

Wifi is not listed because it is a level meter from `system::bars_glyph` and ignores `icon_style`.

| Indicator | ascii | code | nerd | code |
|---|---|---|---|---|
| Cpu | ⊙ | U+2299 |  | U+F2DB microchip |
| Mem | ▤ | U+25A4 |  | U+F1C0 database |
| BatteryCharging | ↯ | U+21AF |  | U+F0E7 bolt |
| Battery | ▯ | U+25AF |  | U+F240 battery-full |
| VolMute | ✗ | U+2717 |  | U+F026 volume-off |
| VolLow | ♪ | U+266A |  | U+F027 volume-down |
| VolHigh | ♫ | U+266B |  | U+F028 volume-up |
| Bluetooth | ≋ | U+224B |  | U+F293 bluetooth |
| Net | ⇄ | U+21C4 |  | U+F0EC exchange |
| Bell | ⍾ | U+237E |  | U+F0F3 bell |
| Clipboard | ⎘ | U+2398 |  | U+F0EA paste |

## File-type icons (desktop and file manager fallback)

| Role | ascii | code | nerd | code |
|---|---|---|---|---|
| Directory | ■ | U+25A0 |  | U+F07B folder |
| Executable | ● | U+25CF |  | U+F120 terminal |
| Text | ▤ | U+25A4 |  | U+F0F6 file-text |
| Code | ◇ | U+25C7 |  | U+F1C9 file-code |
| Image | ◆ | U+25C6 |  | U+F1C5 file-image |
| Audio | ♪ | U+266A |  | U+F1C7 file-audio |
| Video | ▶ | U+25B6 |  | U+F1C8 file-video |
| Archive | ▣ | U+25A3 |  | U+F1C6 file-archive |
| Pdf | ▦ | U+25A6 |  | U+F1C1 file-pdf |
| Other | □ | U+25A1 |  | U+F016 file |

## Wallpaper frame cost measurement

Measured at 250x70 in a release build with a throwaway test, solid desktop versus chafa wallpaper. The daemon-side cost is the same in both modes, so wallpaper slowness on macOS comes from the terminal repainting many unique truecolor cells, not from tetron-wm.

| | solid | wallpaper |
|---|---|---|
| Per frame (build, composite, diff) | ~540-640 µs | ~570-655 µs |
| Full redraw | 17,500 cells | 17,500 cells |
| `SessionCore::new` | ~110-150 ms | ~120-160 ms |

## Notes

- A true 7-bit fallback style (for example `[D] [x] [t]`) does not exist. It would help Android terminals and fonts that lack the geometric shapes.
- The `emoji` style also exists and is intended for GUI terminals only.
