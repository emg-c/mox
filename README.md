# Mox

**[한국어](README.ko.md)** · English

Mox is a cross-platform mouse & keyboard macro recorder and player for **macOS** and **Windows**.
Record what you do, edit the timing, replay it, and export a standalone double-clickable program — all from a single tool, with a GUI and a CLI.

> Status: early (alpha). The core works and is hardware-tested on both macOS (Apple Silicon) and Windows 10 x64, but packaging and polish are still in progress.

## Features

- **Record** mouse moves, clicks (left/right/middle/back/forward), wheel, and keyboard input with millisecond timing.
- **Edit** the recorded log:
  - scale / cap / set delays (make playback faster or slower)
  - thin out mouse-move events (half / by interval / by distance)
  - remove moves (keep clicks) or all mouse events
  - absolute ↔ ratio coordinates (so a macro survives a different resolution)
  - remove key auto-repeat, swap Cmd ↔ Ctrl (port macros between macOS and Windows)
- **Play** with repeat count, speed multiplier, and an optional "keep original timing" mode. Stop anytime with a hotkey or by shoving the mouse into a screen corner (failsafe).
- **Export** a macro to a standalone executable — a Windows `.exe` or a macOS `.app` (zip) — that plays on a machine without Mox installed. You can export **both targets from either OS**.
- **GUI** (egui) with a live event table that fills in as you record, plus a **CLI** for scripting.
- Open, human-editable `.mox` file format (UTF-8 JSON).

## Platforms

| | Recording & playback | Standalone export |
|---|---|---|
| macOS 12+ (Apple Silicon / Intel, universal) | ✅ | `.app` (zip) |
| Windows 10/11 x64 | ✅ | `.exe` |

## Install & build

Mox is a Rust workspace. Build natively on each target OS with a recent Rust toolchain (1.90+).

```sh
# the main app (GUI + CLI). Run `mox` with no arguments for the GUI.
cargo build --release -p mox-app
```

The **export** feature embeds prebuilt player binaries into the app. To enable it, build the player for each target first, then point the app's build at them:

```sh
# build the headless players (per target; cross-compile or build on each OS)
cargo build --release -p mox-player --target x86_64-pc-windows-gnullvm   # → mox-player.exe
cargo build --release -p mox-player --target aarch64-apple-darwin        # (universal via lipo)

# build the app with the players embedded
MOX_WIN_PLAYER=/path/to/mox-player.exe \
MOX_MAC_PLAYER=/path/to/mox-player-macos-universal \
cargo build --release -p mox-app
```

If the env vars are omitted, the app still builds — export for that target is simply disabled with a clear message.

> For CLI-only builds (no GUI, e.g. on Linux CI): `cargo build -p mox-app --no-default-features`.

## CLI

```sh
mox                                   # launch the GUI
mox record -o work.mox                # record until the stop key
mox play work.mox --repeat 3 --speed 2
mox play work.mox --no-timing         # replay as fast as possible
mox edit work.mox scale-delay 0.5     # halve every delay
mox edit work.mox cap-delay 500       # clamp waits longer than 500 ms
mox edit work.mox thin-half           # drop every other mouse-move
mox edit work.mox to-ratio            # absolute → ratio coordinates
mox export work.mox --target windows -o work.exe
mox export work.mox --target macos   -o work.app.zip
mox selftest                          # diagnose the input layer
```

Run `mox <command> --help` for all options. Edit operations: `scale-delay`, `cap-delay`, `set-delay`, `thin-half`, `thin-interval`, `thin-distance`, `rm-moves`, `rm-mouse`, `to-ratio`, `to-abs`, `rm-key-repeats`, `swap-cmd-ctrl`.

## `.mox` file format

UTF-8 JSON, one event per line. Delays (`d`) are the gap in milliseconds from the previous event, so editing one delay does not shift the rest.

```json
{
  "format": "mox",
  "version": 1,
  "meta": { "name": "주간보고", "created": "2026-10-10T14:03:00Z", "recorded_on": "macos",
            "screen": { "width": 1512, "height": 982, "scale": 2.0, "unit": "point" } },
  "play": { "repeat": 1, "speed": 1.0, "timed": true, "min_gap_ms": 5 },
  "events": [
    {"d": 0,   "type": "move",     "x": 512, "y": 384},
    {"d": 120, "type": "down",     "button": "left", "x": 512, "y": 384},
    {"d": 800, "type": "key_down", "key": "KeyA"}
  ]
}
```

Key names follow the web `KeyboardEvent.code` convention (`KeyA`, `Enter`, `ShiftLeft`, `MetaLeft`, …), so macros are not tied to a keyboard layout or language.

## Permissions & platform notes

- **macOS**: recording needs **Input Monitoring**, playback needs **Accessibility** (System Settings → Privacy & Security). Mox opens the right settings pane when a permission is missing.
- **macOS exported apps** are ad-hoc signed, not notarized. The first time someone runs one, they must allow it in *System Settings → Privacy & Security → Open Anyway*, and grant Accessibility per app.
- **Windows**: input injection and low-level hooks only work in the **interactive desktop session**. Double-clicking an exported `.exe` works; running it over a bare SSH session does not.
- **Coordinates** are recorded as absolute pixels/points by default. Converting to ratio coordinates (menu or `to-ratio`) helps a macro replay on a different screen size.

## Architecture

A Cargo workspace of focused crates:

| crate | responsibility |
|---|---|
| `mox-format` | `.mox` data model, read/write, validation |
| `mox-edit` | pure editing functions + undo/redo |
| `mox-engine` | record/play logic over `InputBackend`/`Clock` traits (OS-agnostic, fully unit-tested) |
| `mox-input` | OS input layer: macOS via Core Graphics, Windows via `windows-sys` |
| `mox-payload` / `mox-export` | standalone-executable packaging & ad-hoc signing |
| `mox-player` | the headless player embedded into exported executables |
| `mox-app` | the `mox` binary: egui GUI + CLI |

OS-specific code lives only in `mox-input`; everything else is portable and tested on Linux.

## License

To be decided. Until a license is added, no usage rights are granted.
