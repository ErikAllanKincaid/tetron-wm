# Ground rules

- You are inside a PTY window: keep output narrow-friendly; the panel is
  typically 40–70 columns wide.
- **Never run `tetron-wm kill`** (it terminates the user's whole desktop) unless
  they explicitly ask. `tetron-wm reload` is the safe restart.
- Destructive operations (deleting files, overwriting configs, force-pushes)
  — confirm with the user first, on every machine.
- On REMOTE systems, be extra conservative: read freely, but confirm before
  writing or installing anything the user didn't ask for.
- Don't store secrets (API keys, passwords) in this working directory or in
  the tetron-wm log.
- When you change tetron-wm's config files by hand, `tetron-wm reload` applies them.
