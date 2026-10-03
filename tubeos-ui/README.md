# Tube OS TV UI Shell

A cinematic, lightweight 10-foot Smart TV interface and session manager for Tube OS, powered by Openbox, Tauri/WebKitGTK, and native X11 integration.

## Key Features

- **Openbox TV Session**: Auto-starts in fullscreen without window borders, managing TV display settings and preventing blanking.
- **TV & Gamepad Remote Navigation**: 2D grid/shelf focus engine optimized for TV remotes (HDMI CEC) and gamepads (D-Pad and analog sticks).
- **Home Button Handler (`tubeos-home`)**: Pressing the Super/Windows key, keyboard Home, gamepad Guide/Home button, or HDMI CEC Root Menu gracefully closes active foreground apps and refocuses Tube OS UI.
- **Gboard-Style Virtual Keyboard (`tubeos-keyboard`)**:
  - D-Pad/remote navigable on-screen keyboard.
  - Multi-language support (Spanish with `Ñ`, English, Symbols `?123`).
  - Dismissible with **Back / Escape** or by **navigating UP past the top suggestion bar**.
- **HDMI CEC Daemon (`tubeos-cec-daemon`)**: Bridges TV remote control signals (D-Pad, Select, Exit, Root Menu, Play/Pause, Volume, Mute) directly to X11 key events.
- **Global Gamepad Listener (`tubeos-gamepad-daemon`)**: Listens globally for Guide/Home button presses across any running games or emulators.

## Quick Testing & Development

```bash
cd PKG/tubeos-ui

# 1. Dev mode with Vite hot-reload (in browser / dev server)
./run.sh dev

# 2. Standalone release mode (UI embedded)
./run.sh

# 3. Test standalone Gboard virtual keyboard
python3 usr/bin/tubeos-keyboard
```

## Building Assets

To compile the TypeScript/Vite bundle and sync assets to `usr/share/tubeos-ui/`:

```bash
./prepare-assets.sh
```
