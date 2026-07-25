# mouse-macro

Hold a mouse button to repeatedly simulate keyboard presses. Configurable per-key timing, supports all standard keys plus side mouse buttons, numpad, and modifiers.

## Requirements

- **macOS**: Rust toolchain + Accessibility permission
- **Linux**: Rust toolchain + X11 (`rdev` backend)
- **Windows**: Rust toolchain (`rdev` backend)

## Quick start

```bash
# Build
cargo build --release

# Discover your mouse button IDs (especially useful for side buttons)
cargo run -- --identify

# Run the macro (loads config from ~/.config/mouse-macro/config.toml)
cargo run
```

### macOS — grant Accessibility permission

1. Open **System Settings → Privacy & Security → Accessibility**
2. Click **+** and add your terminal emulator (Terminal.app, iTerm, etc.)
3. Enable the toggle next to it
4. Re-run `cargo run`

Without this, the app cannot listen for global mouse events or simulate keyboard input.

## Configuration

On first run a default config is created at:

```
~/.config/mouse-macro/config.toml
```

### Full example

```toml
# Which mouse button triggers the macro.
# Named: "Left", "Right", "Middle"
# Side/extra buttons: use the number from `--identify`, e.g. "4"
mouse_button = "Left"

# Global defaults — used when per-key values are omitted below.
interval_ms = 100          # default gap between keys (after release, before next)
press_duration_ms = 30     # default hold time per key

# ── Key sequence ─────────────────────────────────────────────────────
#
# Two styles, mix freely:
#   Style 1 — bare key name, uses globals
#   Style 2 — table with per-key press_ms / delay_ms overrides

[[keys]]
key = "1"
press_ms = 5
delay_ms = 15

[[keys]]
key = "2"
press_ms = 5
delay_ms = 15

[[keys]]
key = "3"
press_ms = 5
delay_ms = 100   # longer pause before repeating

[[keys]]
key = "return"   # uses global press_duration_ms + interval_ms
```

### Simple format (all keys use globals)

```toml
mouse_button = "Left"
keys = ["a", "b", "return", "tab"]
interval_ms = 80
press_duration_ms = 30
```

## Key reference

### Letters & numbers

| Config   | Key        |
|----------|------------|
| `a`–`z`  | A – Z      |
| `0`–`9`  | Number row |

### Numpad

| Config                     | Key          |
|----------------------------|--------------|
| `kp0` – `kp9`              | Numpad 0–9   |
| `kpreturn`, `kpenter`      | Numpad Enter |
| `kpminus`, `kp-`           | Numpad −     |
| `kpplus`, `kp+`            | Numpad +     |
| `kpmultiply`, `kp*`        | Numpad ×     |
| `kpdivide`, `kp/`          | Numpad ÷     |
| `kpdelete`, `kpdel`, `kp.` | Numpad Del   |

### Function keys

`f1` `f2` `f3` `f4` `f5` `f6` `f7` `f8` `f9` `f10` `f11` `f12`

### Navigation & editing

| Config                      | Key              |
|-----------------------------|------------------|
| `up` `down` `left` `right`  | Arrow keys       |
| `home` `end`                | Home / End       |
| `pageup`/`pgup`             | Page Up          |
| `pagedown`/`pgdn`           | Page Down        |
| `insert`/`ins`              | Insert           |
| `delete`/`del`              | Delete (forward) |
| `backspace`                 | Backspace        |
| `return`/`enter`            | Return / Enter   |
| `space`                     | Space            |
| `tab`                       | Tab              |
| `escape`/`esc`              | Escape           |

### Modifiers

| Config                                      | Key               |
|---------------------------------------------|-------------------|
| `control`/`ctrl`                            | Left Control      |
| `rightcontrol`/`rctrl`                      | Right Control     |
| `shift`                                     | Left Shift        |
| `rightshift`/`rshift`                       | Right Shift       |
| `alt`/`option`                              | Left Alt / Option |
| `altgr`                                     | AltGr             |
| `meta`/`command`/`cmd`/`super`/`windows`    | Left Meta / Cmd   |
| `rightmeta`/`rmeta`/`rcmd`/`rightcommand`   | Right Meta / Cmd  |
| `capslock`/`caps`                           | Caps Lock         |

### Symbols

| Config                     | Key           |
|----------------------------|---------------|
| `minus`/`-`                | - (hyphen)    |
| `equal`/`=`               | =             |
| `leftbracket`/`[`          | [             |
| `rightbracket`/`]`         | ]             |
| `semicolon`/`;`            | ;             |
| `quote`/`'`                | '             |
| `comma`/`,`                | ,             |
| `period`/`dot`/`.`         | .             |
| `slash`/`/`                | /             |
| `backslash`/`\`            | \             |
| `backquote`/`` ` ``        | `             |

### Misc

| Config                  | Key          |
|-------------------------|--------------|
| `printscreen`/`prtsc`   | Print Screen |
| `scrolllock`            | Scroll Lock  |
| `pause`/`break`         | Pause/Break  |
| `numlock`/`numlk`       | Num Lock     |

## Mouse button identification

```bash
cargo run -- --identify
```

Press any mouse button and the app prints its ID:

```
── Listening for mouse events ──

  PRESS  │  Left    →  put this in your config: mouse_button = "Left"
  RELEASE│  Left    →  config value: "Left"
  PRESS  │  Unknown(4)  →  put this in your config: mouse_button = "4"
  RELEASE│  Unknown(4)  →  config value: "4"
```

Side / extra mouse buttons show as `Unknown(N)` — use the number in your config.

## How it works

1. The app listens globally for the configured mouse button via a macOS CGEvent tap (or `rdev` on other platforms).
2. While held, it spawns a thread that loops over the key sequence, posting keyboard events at the configured intervals.
3. Releasing the button stops the loop immediately.

## Project structure

```
mouseMarco/
├── Cargo.toml
├── README.md
├── .gitignore
└── src/
    ├── main.rs       # Config, key mapping, identify mode, macro engine
    └── platform.rs   # macOS CGEvent tap + complete keyboard simulation
```

## License

MIT
