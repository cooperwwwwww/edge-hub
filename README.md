<p align="center"><img src="app/icon.png" width="96" alt="Edge Hub icon"></p>

<h1 align="center">Edge Hub</h1>
<p align="center">A fast, free controller control center for Windows: remap, tune sticks and triggers, macros, gyro, light bar and more.</p>

<p align="center"><b><a href="../../releases/latest">⬇ Download EdgeHub.exe</a></b> · Windows 10 / 11 · 1.3 MB · no install</p>

---

## Supported controllers

| Controller | Connection | Notes |
|---|---|---|
| PS4 DualShock 4 and PS4-style pads (e.g. Hex Gaming) | USB / Bluetooth | buttons, sticks, triggers, gyro, touchpad, battery, light bar, rumble |
| PS5 DualSense / DualSense Edge | USB / Bluetooth | same as above |
| Xbox One / Series / 360, GameSir, 8BitDo, PowerA… in X-input mode | USB / wireless | full 16-bit stick precision, rumble |

## Features

- **Remap any button** to an Xbox or PlayStation button, a keyboard key, a **key combo** (e.g. `Win+Alt+G` to save a clip), a mouse click, scroll or media key
- **Turbo**, **toggle** (press once to hold), and a **shift layer** (hold a paddle to give every button a second job)
- **Macros**: record them straight from your controller, edit steps visually, or start from templates
- **Sticks**: dead zones (radial / axial / hybrid), anti-dead zone, response curves you can **drag and add points to**, sensitivity, **hi-res stick** (recovers detail between the stick's 256 steps), **anti-snapback**, aim (L2) sensitivity, angle snapping, drift fix
- **Triggers**: dead zone, max range, curves, hair trigger
- **Gyro aiming**, **stick-as-mouse**, **touchpad-as-trackpad**
- **Light bar** colours and effects, vibration strength, low-battery warning
- **Profiles** with automatic switching per game, and share codes to send a profile to a friend
- **Output as Xbox or PlayStation**: games see a virtual controller of your choice (via ViGEmBus)
- **Controller visualizer & OBS overlay**: real-photo skins (PS5 DualSense, Xbox Series, or a photo of *your own* controller, cut out and lined up for you) where every button lights up, the sticks move and the triggers fill, plus drawn models with 18 skins; one link for an OBS Browser source
- **Tester** with a live polling-rate meter (report timing, jitter), ready for 2–8 kHz controllers
- **Always on**: runs in the system tray, starts with Windows, single instance, auto-reconnect watchdog
- **Hide your real controller from games** (HidHide) so games only ever see Edge Hub's controller, with no double input and no wrong-controller problems
- **Status checklist** on the Home page showing what's working and what needs fixing, with one-click fixes
- **Restore original setup**: one click to hand the controller back to Windows' standard drivers

Every report is processed the moment it arrives (≈0.01 ms), with no smoothing unless you turn it on.

## Getting started

1. Download `EdgeHub.exe` from [Releases](../../releases/latest) and run it.
   If Windows shows *"Windows protected your PC"*, click **More info → Run anyway** (the app isn't code-signed).
2. Plug in or pair your controller.
3. To use your settings in games: **Settings → Install driver** (installs [ViGEmBus](https://github.com/nefarius/ViGEmBus) once), then turn on **Remapping**.
   Keyboard / mouse output, the tester and the light bar work without it.

4. Recommended: **Settings → Hide my controller** (installs [HidHide](https://github.com/nefarius/HidHide) once), so games only see Edge Hub's controller.
5. Optional: **Settings → Start with Windows**. Edge Hub then lives in the tray and your settings are always active.

**Why not save settings onto the controller itself?** PS4/PS5 controllers (and their clones) have no firmware channel for storing mappings, so remaps live in Edge Hub. With *Start with Windows* + *tray* it behaves the same: plug in and play.

### 1000 Hz over USB (optional)

PS4-style controllers report at 250 Hz over USB. With the free [hidusbf](https://github.com/LordOfMice/hidusbf) filter driver (use the Microsoft-signed *NoPatch* driver; Secure Boot and Memory Integrity can stay on) they can be polled at 1000 Hz. 1000 Hz is the maximum for full-speed USB controllers. Edge Hub's Tester shows the real rate.

## Building from source

Requires Rust (MSVC toolchain) and the Windows SDK.

```bash
cd app
cargo build --release
```

The exe lands in `app/target/release/EdgeHub.exe` (the UI in `ui/index.html` is embedded at build time).
If the project sits in a very long folder path, point the build output somewhere short: `set CARGO_TARGET_DIR=C:\ehbuild`.

`cargo test` runs the engine tests (dead zones, curves, macros, combos, DualSense / XInput parsing, etc.).

## How it's built

- **Engine** (`app/src/engine.rs`): parses controller reports and applies the profile, all pure functions of the current sample
- **App** (`app/src/main.rs`): raw HID / XInput input on a high-priority thread, virtual pad via [vigem-client](https://crates.io/crates/vigem-client), keyboard/mouse via `SendInput`
- **UI** (`ui/index.html`): one HTML file rendered in a WebView2 window ([wry](https://github.com/tauri-apps/wry) + [tao](https://github.com/tauri-apps/tao))

---

## Photo credits

The built-in photo skins are cut-outs of photos from Wikimedia Commons, used under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). The edited images (`ui/photos/ps5.png`, `ui/photos/xbox.png`) are shared under the same license.

- DualSense: photo by Chris Woodrich, CC BY-SA 4.0
- Xbox Series controller: photo by uKER, CC BY-SA 4.0

---

Edge Hub is an independent project and isn't affiliated with Sony, Microsoft, GameSir or Hex Gaming.
