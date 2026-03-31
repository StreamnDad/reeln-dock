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

## Frontend Views

| View | Purpose |
|---|---|
| **Games** | Game list grouped by tournament, clip review panel, event tagging, render options |
| **Queue** | Render queue — batch staging area with per-item rendering and progress |
| **Plugins** | Plugin manager — enable/disable per config profile, settings editor |
| **Registry** | Plugin registry browser — discover and install plugins |
| **Settings** | Dock config, teams, tournaments, event types, rendering defaults, logs |

## Plugin-Driven UI

The dock is **plugin-first** — plugins declare what UI fields they contribute, and the dock
renders them dynamically based on what's installed and enabled.

### How It Works

1. Plugins declare `ui_contributions` in the registry (`registry/plugins.json`)
2. Each contribution targets a screen: `render_options`, `settings`, or `clip_review`
3. The dock loads the registry, cross-references with installed/enabled plugins
4. `DynamicPluginFields.svelte` renders active fields per screen
5. Field values flow into `RenderOverrides` (render_options) or event metadata (clip_review)

### Registry Schema for UI Contributions

```json
{
  "name": "openai",
  "ui_contributions": {
    "render_options": {
      "fields": [
        {
          "id": "smart",
          "label": "Smart Zoom",
          "type": "boolean",
          "default": false,
          "description": "AI-powered smart crop tracking",
          "maps_to": "smart"
        },
        {
          "id": "zoom_frames",
          "label": "Zoom Frames",
          "type": "number",
          "min": 1,
          "max": 30,
          "step": 1,
          "description": "Keyframes for smart zoom path",
          "maps_to": "zoom_frames"
        }
      ]
    }
  }
}
```

### Field Types

| Type | Renders as | Props |
|---|---|---|
| `boolean` | Checkbox | `default` |
| `number` | Range slider or number input | `min`, `max`, `step`, `default` |
| `string` | Text input | `default` |
| `select` | Dropdown | `options: [{value, label}]`, `default` |

### Key Files

| File | Purpose |
|---|---|
| `src/lib/types/plugin.ts` | `PluginUIField`, `PluginUIScreen`, `PluginUIContributions` types |
| `src/lib/stores/pluginUI.svelte.ts` | Loads registry + enabled plugins, computes active fields per screen |
| `src/lib/components/content/DynamicPluginFields.svelte` | Generic field renderer for plugin-declared fields |
| `src-tauri/src/commands/plugins.rs` | `RegistryPlugin` passes `ui_contributions` as raw JSON |

### Render Modes

Two render modes, selectable per-clip and configurable as default in Settings > Rendering:

| Mode | Behavior |
|---|---|
| **Short** | Crops/scales to profile dimensions (vertical shorts for social media) |
| **Apply** | Full-frame, no crop/scale — only applies speed, LUT, overlay from profile |

## Stores Architecture

| Store | Persistence | Purpose |
|---|---|---|
| `config.svelte.ts` | Tauri IPC (DockSettings JSON) | App config, dock settings |
| `renderQueue.svelte.ts` | `render-queue.json` in app data dir | Render queue items across sessions |
| `uiPrefs.svelte.ts` | In-memory (session) | Auto-play, auto-advance, filters, section toggles |
| `pluginUI.svelte.ts` | In-memory (loaded from registry) | Active plugin UI contributions per screen |
| `jobs.svelte.ts` | In-memory | Render job progress tracking |
| `games.ts` | Writable stores | Game list, selected game/event, filters |
| `navigation.ts` | Writable stores | Current view, sidebar mode |

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
