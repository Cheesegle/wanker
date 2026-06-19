# Wankle Client

An optimized desktop client for **Wankle3D** (`wankle.online` / `wanshot.lol`),
built on [tauri-apps/wry](https://github.com/tauri-apps/wry).

The game runs untouched in a native webview — nothing is injected into the page.
All client controls live in a separate, draggable/resizable overlay window you
toggle with a hotkey. Performance practices are adapted from
[glorp](https://github.com/slavcp/glorp) (Chromium flag tuning, ANGLE
rendering-API select, CPU throttle, process priority), but this is **not** a
Krunker client — there are no game-specific cheats or mods, only engine/perf
controls.

## Features

- **Launcher-first flow** — pick your settings, then launch the game with them applied.
- **Floating menu overlay** — borderless, transparent, always-on-top. Drag it by the
  title bar, resize from any edge/corner. Toggle with **F1** (OS-level hotkey, so it
  works even while the game has focus). No page injection.
- **Rendering API select** *(Windows)* — Auto / D3D11 / D3D9 / OpenGL / Vulkan /
  SwiftShader, via the ANGLE backend.
- **Curated Chromium performance flags** *(Windows)* — generic GPU/compositor/V8/WASM
  tuning. Fully overridable with your own flags or by disabling defaults.
- **Unlock FPS** *(Windows)* — disables the vsync / frame-rate cap.
- **CPU throttle** *(Windows)* — live slider using the WebView2 DevTools protocol
  (`Emulation.setCPUThrottlingRate`); useful when you're GPU-bound.
- **Process priority** *(Windows)* — Idle … High, applied to the app and its WebView2
  processes.
- **Window modes** — Windowed / Maximized / Borderless Fullscreen.
- **Settings persisted** to `config.json` in your user config dir.

## Cross-platform

| Feature | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Game + menu, drag/resize, hotkey, window modes, persistence | ✅ | ✅ | ✅ |
| Chromium flags / rendering-API / FPS unlock | ✅ | — | — |
| CPU throttle / process priority | ✅ | — | — |

wry uses Chromium (WebView2) on Windows but WebKit on macOS/Linux, which doesn't
expose Chromium flags or the DevTools protocol. The build runs everywhere; the
Windows-only controls are greyed out (with a note) on other platforms.

## Build & run

Prerequisites: [Rust + Cargo](https://rustup.rs/). On Windows you also need the
Edge **WebView2 Runtime** (preinstalled on Windows 11). On Linux: WebKitGTK
(`libwebkit2gtk-4.1-dev`) and GTK dev packages.

```sh
cargo run            # debug
cargo build --release
```

The optimized binary is at `target/release/wankle-client(.exe)`.

## How settings apply

- **Live** (no reload): CPU throttle, process priority, DevTools.
- **On launch / relaunch**: anything that changes Chromium flags (rendering API,
  GPU/FPS toggles, extra flags) or the target URL. WebView2 only honors one set of
  browser flags per process, so the client relaunches itself to apply them cleanly
  (glorp does the same).

## Config location

- Windows: `%APPDATA%\wankle-client\config.json`
- macOS: `~/Library/Application Support/wankle-client/config.json`
- Linux: `~/.config/wankle-client/config.json`

## Project layout

| File | Purpose |
| --- | --- |
| `src/main.rs` | Event loop, game + overlay windows, hotkey, IPC dispatch, launcher/relaunch flow |
| `src/config.rs` | Persisted settings (serde JSON) |
| `src/flags.rs` | Curated Chromium arg builder + rendering-API select |
| `src/platform/windows.rs` | CPU throttle (CDP) + process priority |
| `src/platform/other.rs` | No-op stubs for macOS/Linux |
| `src/menu.html` | Self-contained settings UI (HTML/CSS/JS) |
