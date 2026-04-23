# Contributing to reeln-dock

Thanks for your interest in contributing to reeln-dock! This document
covers the basics for getting started.

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
Be respectful and constructive.

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
- FFmpeg development libraries (see below)
- [reeln-core](https://github.com/StreamnDad/reeln-core) cloned alongside this repo

### System Dependencies

**macOS:**
```bash
brew install ffmpeg pkg-config
```

**Ubuntu/Debian:**
```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
  libavcodec-dev libavformat-dev libavfilter-dev libavdevice-dev \
  libavutil-dev libswscale-dev libswresample-dev pkg-config
```

### Development

```bash
npm install          # Install frontend dependencies
cargo tauri dev      # Start dev mode (frontend + backend with hot reload)
```

### Running Checks

```bash
# Frontend
npm run check        # svelte-check + TypeScript
npm run test         # Vitest unit tests

# Backend (from src-tauri/)
cargo fmt --check    # Formatting
cargo clippy -- -D warnings  # Linting
cargo test           # Unit tests
```

## Pull Requests

1. Fork the repo and create a branch from `main`
2. Make your changes with tests
3. Ensure all checks pass (`npm run check`, `npm run test`, `cargo clippy`, `cargo test`)
4. Write a clear PR description explaining what and why
5. Keep PRs focused — one feature or fix per PR

### Commit Messages

We use [conventional commits](https://www.conventionalcommits.org/):

```
feat: add new render mode
fix: correct overlay positioning on 4K displays
test: add coverage for queue edge cases
docs: update build instructions for Windows
```

## Architecture

See [CLAUDE.md](./CLAUDE.md) for detailed architecture docs, domain
concepts, and coding conventions.

**Key constraints:**
- The dock is a GUI wrapper around reeln-cli — CLI parity is mandatory
- All state mutations go through `reeln-state` functions, never direct field access
- Frontend uses Svelte 5 runes (`$state`, `$derived`, `$effect`)

## Reporting Issues

- **Bugs:** Use the [Bug Report](https://github.com/StreamnDad/reeln-dock/issues/new?template=bug_report.yml) template
- **Features:** Use the [Feature Request](https://github.com/StreamnDad/reeln-dock/issues/new?template=feature_request.yml) template
- **Security:** See [SECURITY.md](./SECURITY.md) — do not use public issues

## License

By contributing, you agree that your contributions will be licensed under
the [AGPL-3.0-only](./LICENSE) license.
