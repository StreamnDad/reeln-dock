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

### Config Ownership

The main reeln config file (`~/.config/reeln/config/*.json`) is **owned by reeln-cli**.
The dock reads and writes to this file but must preserve compatibility with the CLI's
config parser and validator. Any changes to config structure or new fields must be
coordinated with the reeln-cli project (`/Users/jremitz/workspace/reeln-cli/reeln/core`).

- **Validator:** `reeln-cli/reeln/core/config.py` — `validate_config()` checks the raw JSON
- **Deserializer:** Same file — converts JSON to `ReelnConfig` dataclass
- **Schema:** Both string and object formats are valid for `event_types`; both must be
  accepted by the CLI validator before the dock writes them

### CLI-Parity Mandate (CRITICAL)

**The dock is a GUI wrapper around reeln-cli. Every dock action must produce identical
results to the equivalent CLI command.** This is a non-negotiable architectural constraint.

Rules:

1. **No proprietary logic.** If a feature exists in the dock, it must map 1:1 to a CLI
   command or flag. The dock must never invent render behavior, config interpretation,
   or iteration logic that doesn't exist in the CLI.
2. **User selections are authoritative.** When the user selects profiles, overrides, or
   options in the dock UI, those exact values must be passed to the CLI. Never delegate
   decision-making back to the CLI (e.g., via `--iterate`) when the dock already has the
   user's explicit choices.
3. **Same config, same result.** Given the same config file and input clip, `reeln render`
   from the terminal and "Render" from the dock must produce byte-equivalent output.
4. **CLI flags are the contract.** Every Tauri IPC command that shells out to the CLI must
   construct the same flags a user would type. Document which CLI command each IPC handler
   maps to.
5. **Test parity.** When adding render/config IPC, include a test that verifies the
   constructed CLI args match expected `reeln` invocations.
6. **Dock-only settings are GUI-only.** Window size, sidebar mode, auto-play — these are
   dock-specific and don't touch CLI behavior. Everything else (render profiles, iteration
   mappings, event types, plugin config) must round-trip through the CLI's config format.

Violation of CLI parity is a **critical bug** — treat it with the same urgency as data loss.

### State Mutation Boundary (CRITICAL)

**The dock MUST NOT directly mutate `GameState` fields.** All state mutations go through
`reeln-state` functions. The dock is a thin UX layer — it routes user actions to
`reeln-state` (and other `reeln-core` crates), then reflects the result in the UI.

```
WRONG:  state.finished = true;
        state.finished_at = chrono::Utc::now().to_rfc3339();
        reeln_state::save_game_state(&state, path)?;

RIGHT:  reeln_state::mark_finished(&mut state);
        reeln_state::save_game_state(&state, path)?;
```

#### What the Dock DOES Own

- Tauri IPC routing (`#[tauri::command]` handlers)
- Frontend stores, reactivity, and UI components
- Progress reporting and Tauri event emission
- Render queue management (dock-only persistence in `render-queue.json`)
- Window/sidebar/view state (session-only, no `game.json` impact)
- Orchestration sequencing (e.g., "find videos, then concat, then update state")
  — but each state mutation step calls a `reeln-state` function

#### What the Dock MUST NOT Own

- Event field mutation logic (use `reeln_state::update_event_field()`)
- Event type tagging (use `reeln_state::tag_event()`)
- Render entry creation (use `reeln_state::add_render()`)
- Render pruning (use `reeln_state::clear_renders()`)
- Segment processed tracking (use `reeln_state::mark_segment_processed()`)
- Highlight merge state (use `reeln_state::mark_highlighted()`)
- Game finish state (use `reeln_state::mark_finished()`)
- Tournament assignment (use `reeln_state::set_tournament()`)

#### Migration Checklist (existing violations)

These dock functions currently mutate `GameState` directly and must be refactored
to call `reeln-state` mutation functions once they exist:

| Dock Location | Violation | Target `reeln-state` Function |
|---|---|---|
| `commands/games.rs::update_game_event` | Mutates event fields directly | `update_event_field()` |
| `commands/games.rs::set_game_tournament` | Sets `game_info.tournament` directly | `set_tournament()` |
| `commands/games.rs::bulk_update_event_type` | Loops and sets `event.event_type` | `bulk_update_event_type()` |
| `commands/games.rs::prune_renders` | Calls `state.renders.clear()` | `clear_renders()` |
| `commands/games.rs::quick_tag_event` | Sets `event_type` + metadata directly | `tag_event()` |
| `orchestration/game_ops.rs::process_segment` | Pushes events, segments_processed, segment_outputs | `add_event()`, `mark_segment_processed()`, `set_segment_output()` |
| `orchestration/game_ops.rs::merge_highlights` | Sets `highlighted` + `highlights_output` | `mark_highlighted()` |
| `orchestration/game_ops.rs::finish_game` | Sets `finished` + `finished_at` | `mark_finished()` |
| `commands/render.rs::render_short` | Pushes to `game_state.renders` | `add_render()` |
| `commands/render.rs::render_iteration` | Extends `game_state.renders` | `add_render()` |
| `orchestration/render_ops.rs` | Pushes to `state.renders` | `add_render()` |

Violation of state mutation boundaries is a **critical bug** — same urgency as data loss.

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
