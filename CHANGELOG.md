# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2026-04-23

### Added

- Guided setup wizard — create a new config with sport picker, directory setup, and summary (no more "locate existing config" as the only option)
- 3 new Tauri commands: `list_available_sports_init`, `create_initial_config`, `check_config_exists`

### Fixed

- Plugin install/update errors now scoped to the correct plugin (no longer leaks to other expanded cards)

## [0.1.2] - 2026-04-23

### Added

- Fetch plugin registry from GitHub when not found locally, with offline cache

### Fixed

- Detect reeln CLI in uv default venv (`~/.venv/bin/`) and uv tool paths
- Recommend `uv tool install reeln` in CLI not found error message
- Help tooltips no longer clipped by app header or parent containers (uses viewport-level rendering)
- Remove duplicate native browser tooltip overlapping custom tooltip
- Clamp tooltip position to stay within window edges

## [0.1.1] - 2026-04-23

### Added

- In-app plugin update via `reeln plugins update` — registry badges trigger upgrades directly
- "Update All" button on registry page when plugin updates are available
- In-app plugin uninstall via `reeln plugins uninstall` with two-click confirmation
- Search/filter on the plugin registry page (by name and description)
- Robust cross-platform CLI detection — searches pyenv, uv, pipx, Homebrew, cargo, and login shell PATH
- Windows CLI detection — checks USERPROFILE, APPDATA, LOCALAPPDATA Python paths
- CLI binary validation with `--version` before accepting (catches broken pyenv shims)
- CI pipeline with GitHub Actions (frontend checks, Rust lint/test, multi-platform including Windows)
- Release workflow with tag-triggered builds for macOS, Linux, and Windows
- SHA-256 checksum generation for all release artifacts
- Comprehensive CLI logging — all `reeln` CLI commands, stdout, stderr, and exit codes piped to log viewer
- Help tooltips with ReadTheDocs links across every screen (72 entries)
- Native menu bar with Help (docs, issues, discussions), View (Cmd+1-4), keyboard shortcuts
- About dialog with project info, icon, and credits
- Automatic update checker for dock, CLI, and installed plugins (once per day)
- Plugin update badges on registry page with version info
- SECURITY.md, CONTRIBUTING.md, issue templates, PR template, GitHub Discussions

### Fixed

- CLI detection on macOS — Tauri apps don't inherit shell PATH; now searches common install locations directly
- User-friendly "CLI not found" error message with `pip install` instructions and docs link
- All `svelte-check` type errors and `cargo clippy` warnings
- macOS Gatekeeper note in release instructions

## [0.1.0] - 2026-04-21

### Added

- Initial Tauri v2 + Svelte 5 desktop application
- Game management: init, list, select, finish, delete, prune
- Event tagging with 1-click quick-tag and expanded clip review
- Segment processing and highlight merge orchestration
- Render pipeline: short mode (crop/scale) and apply mode (full-frame)
- Render queue with per-item rendering, progress tracking, and disk persistence
- CLI bridge for rendering with full plugin feature support
- CLI-backed publish queue with per-item rendering and progress
- Plugin-driven UI architecture with dynamic field rendering from registry
- Plugin manager: enable/disable plugins per config profile
- Plugin auth status display with per-profile caching
- Render profile editor with live preview
- Custom video player with on-demand proxy for MKV/AVI/TS/FLV playback
- Render mode toggle (Short vs Apply) with auto-show results
- Queue item details, edit-from-queue navigation, duplicate detection
- CLI validation gate: detect reeln-cli, version check, feature gating
- Keyboard navigation for clips and sidebar game sort by date
- Persistent UI settings, plugin profile selector, render override defaults
- Event type configuration from reeln config
- Tournament management with drag-and-drop game assignment
- Team settings with logo display and roster management
- State mutation boundary enforcement via reeln-state crate
- Comprehensive test suite: 290 Rust tests + 110 frontend tests

### Fixed

- Render mode and player overlay data flow through full pipeline
- CLI render output discovery in correct location
- Speed/scale sliders default to 1.0 instead of browser midpoint
- Skip builtin overlay templates to prevent composite crash
- Pass `--no-branding` to CLI for native backend compatibility
- Create render entry when CLI iteration path doesn't save to game.json
- Render iteration per-profile via CLI enforcing CLI-parity mandate

[Unreleased]: https://github.com/StreamnDad/reeln-dock/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/StreamnDad/reeln-dock/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/StreamnDad/reeln-dock/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/StreamnDad/reeln-dock/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/StreamnDad/reeln-dock/releases/tag/v0.1.0
