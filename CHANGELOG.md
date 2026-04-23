# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CI pipeline with GitHub Actions (frontend checks, Rust lint/test, multi-platform)
- Release workflow with tag-triggered builds for macOS, Linux, and Windows
- SHA-256 checksum generation for all release artifacts
- Windows `.ico` icon for platform bundling
- Comprehensive CLI logging — all `reeln` CLI commands, stdout, stderr, and exit codes piped to log viewer
- Backend-to-frontend log bridge via Tauri events (`dock:log`)
- Help tooltips with ReadTheDocs links on render profile settings and rendering options
- `HelpLink` component for inline contextual help throughout the UI
- Help text registry (`src/lib/help.ts`) mapping settings to documentation URLs
- SECURITY.md, CONTRIBUTING.md, issue templates, PR template
- GitHub Discussions enabled, branch protection on `main`, Dependabot alerts

### Fixed

- All `svelte-check` type errors (unused imports, test fixture types)
- All `cargo clippy` warnings (`field_reassign_with_default`, collapsible `if`, clamp pattern)
- Rust formatting across the codebase

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

[Unreleased]: https://github.com/StreamnDad/reeln-dock/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/StreamnDad/reeln-dock/releases/tag/v0.1.0
