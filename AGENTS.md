# AGENTS.md — deskBuddy

Guidance for AI agents working in this repo.

## Project

Desktop pet mascot (animated overlay + settings). Migrating from Python (tkinter) to Rust + Tauri 2 + SvelteKit-TS.

## Architecture

- `src-tauri/src/` — Rust backend (sprite parsing, petdex API, pet physics, tray, config)
- `src/lib/` + `src/routes/` — SvelteKit frontend (Settings window, Overlay window)
- Two Tauri windows: `settings` (hidden on boot) + `overlay` (transparent, always-on-top)
- Window routing: both load `index.html`; `+page.svelte` switches on `getCurrentWindow().label`

## Commands

### Lint / typecheck
```powershell
npm run check          # svelte-check (TS + svelte typecheck)
npm run lint           # (add eslint later if desired)
cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
```

### Test
```powershell
cargo test --manifest-path src-tauri/Cargo.toml --all
```

### Dev / build
```powershell
npm install            # first time only
npm run tauri dev      # dev build (opens overlay window)
npm run tauri build    # release bundle (NSIS/AppImage)
```

## Conventions

- Rust: no comments unless asked. Follow `cargo fmt` + `clippy -D warnings`.
- Frontend: TypeScript strict, Svelte 5 runes (`$state`, `$derived`).
- Branches: `feat/tauri-rewrite` is the migration trunk; per-phase sub-branches (e.g. `feat/tauri-scaffold`).
- Don't commit `src-tauri/target/`, `node_modules/`, `petdex_cache/`, `pet_config.json`.

## Migration phases (see GitHub issues #1–#8)

0. Scaffold (this) → 1. Pure ports → 2. Overlay MVP → 3. Settings → 4. Tray/lifecycle → 5. Release → 6. Linux
