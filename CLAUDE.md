# reeln-dock — Development Context

## Project Overview

**reeln-dock** is the cross-platform desktop application for the reeln ecosystem, built
with Tauri v2 + Svelte 5 + TypeScript. It provides a modern GUI for livestreamers to
manage games, process segments, render highlights, and configure their workflow — all
powered by `reeln-core` Rust crates via Tauri's IPC command system.

- **License:** AGPL-3.0-only
- **Org:** StreamnDad | **Homepage:** https://streamn.dad
- **Stack:** Tauri v2 (Rust backend) + Svelte 5 + TypeScript (frontend)

## Ecosystem Context

```
reeln-cli  (Python CLI)  ← user-facing terminal tool, uses reeln-core via PyO3
reeln-core (Rust crates) ← shared business logic (media, overlay, state, config, plugin)
reeln-dock (THIS REPO)   ← desktop GUI, uses reeln-core directly as Rust dependencies
```

### reeln-core Crates (all phases 1–6 complete, 584 Rust tests)

| Crate | Purpose |
|---|---|
| `reeln-media` | Probe, concat, render, filter chains via libav* (no ffmpeg binary) |
| `reeln-overlay` | Template engine: JSON templates → RGBA PNG frames via tiny-skia/cosmic-text |
| `reeln-sport` | Sport registry, segment naming, validation |
| `reeln-state` | Game state machine, JSON persistence, directory management |
| `reeln-config` | XDG config paths, layered merge, env overrides |
| `reeln-plugin` | Hook system, capability traits, plugin registry |
| `reeln-ffi` | C ABI for OBS plugin (not used by this app — Tauri links crates directly) |

### Key Domain Concepts

| Concept | Description |
|---|---|
| **Segment** | Time division of a game (period, quarter, half, inning). Sport aliases map to segment numbers. |
| **Game State** | JSON-persisted state machine tracking segments processed, events, renders, livestreams. |
| **Overlay Template** | JSON-defined visual overlay (rects, text, images, gradients) rasterized to PNG, composited onto video. |
| **Renderer** | Media processing pipeline: scale, codec, CRF, overlay compositing. |
| **Plugin** | Hook-based lifecycle + capability interfaces (Uploader, Notifier, MetadataEnricher). |
| **Scope** | Hierarchy: tournament > game > segment. Determines directory structure and merge boundaries. |

## Architecture

```
reeln-dock/
├── src-tauri/              # Rust backend (Tauri app)
│   ├── Cargo.toml          # depends on reeln-media, reeln-state, reeln-config, etc.
│   ├── src/
│   │   ├── main.rs         # Tauri app entry point
│   │   └── commands.rs     # #[tauri::command] IPC handlers
│   └── tauri.conf.json     # Tauri config (window, security, bundle)
├── src/                    # Svelte 5 + TypeScript frontend
│   ├── App.svelte
│   ├── lib/                # Shared components, stores, utilities
│   └── routes/             # Page components
├── static/                 # Static assets
├── package.json
├── svelte.config.js
├── tsconfig.json
├── vite.config.ts
└── CLAUDE.md
```

### Tauri IPC Pattern

Backend commands in `src-tauri/src/commands.rs` call reeln-core crates directly:
```rust
#[tauri::command]
fn init_game(sport: &str, home: &str, away: &str, date: &str) -> Result<String, String> {
    // calls reeln_state + reeln_sport
}
```

Frontend invokes via `@tauri-apps/api`:
```typescript
import { invoke } from '@tauri-apps/api/core'
const state = await invoke<GameState>('get_game_state', { gameDir })
```

## Dev Commands

```bash
# Frontend
npm install             # install JS dependencies
npm run dev             # vite dev server (hot reload)
npm run build           # production frontend build
npm run check           # svelte-check + tsc
npm run lint            # eslint
npm run format          # prettier

# Tauri
cargo tauri dev         # full dev mode (frontend + backend, hot reload)
cargo tauri build       # production build (bundle .dmg/.msi/.AppImage)

# Rust backend only
cd src-tauri && cargo check
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
```

## Conventions

- **Frontend:** Svelte 5 runes (`$state`, `$derived`, `$effect`), TypeScript strict mode
- **Styling:** Tailwind CSS (or CSS modules — decide during scaffolding)
- **State management:** Svelte stores for UI state, Tauri IPC for persistent state
- **Errors:** Tauri commands return `Result<T, String>` — frontend displays user-friendly messages
- **Config:** JSON format, same schema as reeln-cli (`REELN_*` env overrides)
- **File paths:** Always use Tauri's path APIs (`appDataDir`, `appConfigDir`) — never hardcode

## Frontend Pages (planned)

| Page | Purpose |
|---|---|
| **Dashboard** | Active games, recent highlights, quick actions |
| **Game View** | Segment timeline, drag-and-drop clip reordering, event log |
| **Render Queue** | Progress bars, render history, cancel/retry |
| **Settings** | Config editor, plugin manager, profile switcher |
| **Sport Profiles** | Custom sport definitions, segment naming |
| **Overlay Editor** | Live template preview, variable binding, color picker |

## Dependency on reeln-core

The Tauri backend (`src-tauri/Cargo.toml`) depends on reeln-core crates via path:
```toml
[dependencies]
reeln-media = { path = "../../reeln-core/crates/reeln-media" }
reeln-overlay = { path = "../../reeln-core/crates/reeln-overlay" }
reeln-sport = { path = "../../reeln-core/crates/reeln-sport" }
reeln-state = { path = "../../reeln-core/crates/reeln-state" }
reeln-config = { path = "../../reeln-core/crates/reeln-config" }
reeln-plugin = { path = "../../reeln-core/crates/reeln-plugin" }
```

## Testing Strategy

- **Frontend:** Vitest for unit tests, Playwright for E2E
- **Backend:** `cargo test` in `src-tauri/` (integration tests calling reeln-core)
- **Coverage:** 100% line + branch (per global preference)
