# deskBuddy

Desktop pet mascot — animated overlay that floats over your desktop, reacts with animations, and wanders while idle. Built with Rust + Tauri 2 + SvelteKit-TS.

## Features

- Transparent borderless overlay with 60fps sprite animation
- Roam AI: the pet pauses, walks, and mirrors direction naturally
- Drag-and-drop with gravity (drop above ground → falls)
- Settings window with petdex integration (browse 3000+ pets, download, delete)
- Size slider, roam toggle, on/off toggle
- System tray: Settings, autostart toggle, Quit
- Single-instance: second launch focuses the existing window
- Close settings → hides to tray (app keeps running)
- Autostart on boot (Windows, Linux)

## Prerequisites

- [Rust](https://rustup.rs) (stable, MSVC toolchain on Windows)
- [Node.js](https://nodejs.org) LTS
- Windows: Visual Studio Build Tools with "Desktop development with C++"
- Linux: `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf`

## Development

```powershell
npm install
npm run tauri dev
```

## Build

```powershell
npm run tauri build
```

Produces NSIS installer (Windows) or AppImage/deb (Linux) in `src-tauri/target/release/bundle/`.

## Commands

| Command | Description |
|---|---|
| `npm run check` | svelte-check (TS + Svelte typecheck) |
| `npm run lint` | (eslint — add later) |
| `cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check` | Rust formatting check |
| `cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings` | Rust lint |
| `cargo test --manifest-path src-tauri/Cargo.toml --all` | Rust tests |
| `npm run tauri dev` | Dev build (opens overlay) |
| `npm run tauri build` | Release bundle |

## Architecture

- `src-tauri/src/` — Rust backend (sprite parsing, petdex API, pet physics, tray, config)
- `src/lib/` + `src/routes/` — SvelteKit frontend (Settings window, Overlay window)
- Two Tauri windows: `settings` (hidden on boot) + `overlay` (transparent, always-on-top)
- Window routing: both load `index.html`; `+page.svelte` switches on `getCurrentWindow().label`
- Pet physics: Rust owns the state machine, ticks at 60fps via `tokio::interval`, emits `pet-frame` events
- Sprites: zero-2 spritesheet bundled as Tauri resource; petdex pets cached in `app_data_dir`

## Migration

This project was migrated from Python (tkinter) to Rust + Tauri 2. The legacy Python code is preserved in the `legacy-python` branch.

## License

MIT
