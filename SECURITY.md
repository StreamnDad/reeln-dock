# Security Policy

## Supported Versions

reeln-dock is pre-release software and has not yet published a public
release. Until version `0.1.0` ships, security fixes will land on the
`main` branch. Once releases begin, this table will be updated to track
the supported minor versions.

| Version | Supported          |
| ------- | ------------------ |
| `main` (pre-release) | :white_check_mark: |

## Scope

reeln-dock is a cross-platform desktop companion app for the reeln
ecosystem, built with Tauri 2 (Rust backend + Svelte frontend). It runs
locally on a user's machine and provides visual render profiles, clip
review, and game management on top of `reeln-cli` and `reeln-core`.

In-scope concerns include, but are not limited to:
- Tauri IPC command injection — unsafe handling of arguments passed from
  the frontend to Rust `#[command]` handlers
- Path traversal or unsafe file handling in game directories, render
  output paths, or configuration files
- Cross-site scripting (XSS) in the Svelte frontend via unescaped game
  metadata, clip titles, roster data, or plugin-supplied strings
- Unsafe `tauri-plugin-shell` or `tauri-plugin-dialog` invocations that
  could execute arbitrary commands or open unintended files
- Memory safety issues in the Rust backend (`src-tauri/`), including
  `unsafe` blocks and FFI boundaries with `reeln-core`
- Unsafe deserialization of game state, render manifests, or plugin
  profiles (JSON / TOML)
- Credential leakage from plugin profiles (OAuth tokens, API keys) via
  logs, error messages, or frontend state

Out of scope:
- Vulnerabilities in Tauri, Svelte, or third-party crates — report those
  to the respective upstream project
- Vulnerabilities in `reeln-cli`, `reeln-core`, or individual reeln
  plugins — report those to the respective repository
- Issues that require an attacker to already have local code execution
  on the user's machine

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub
issues, discussions, or pull requests.**

Report vulnerabilities using GitHub's private vulnerability reporting:

1. Go to the [Security tab](https://github.com/StreamnDad/reeln-dock/security)
   of this repository
2. Click **"Report a vulnerability"**
3. Fill in as much detail as you can: affected version or commit,
   reproduction steps, impact, and any suggested mitigation

If you cannot use GitHub's reporting, email **git-security@email.remitz.us**
instead.

### What to include

A good report contains:
- The commit SHA or branch you tested against (reeln-dock is pre-release,
  so there are no version numbers yet)
- Your operating system and architecture (macOS / Windows / Linux, arch)
- Steps to reproduce the issue
- What you expected to happen vs. what actually happened
- The potential impact (credential leakage, code execution, XSS, data
  loss, denial of service, etc.)
- Any proof-of-concept code, if applicable

### What to expect

reeln-dock is maintained by a small team, so all timelines below are
best-effort rather than hard guarantees:

- **Acknowledgement:** typically within a week of your report
- **Initial assessment:** usually within two to three weeks, including
  whether we consider the report in scope and our planned next steps
- **Status updates:** roughly every few weeks until the issue is resolved
- **Fix & disclosure:** coordinated with you. We aim to land a fix on
  `main` reasonably quickly for high-severity issues, with lower-severity
  issues addressed in a future commit or release. Credit will be given
  in the commit message, release notes, or CHANGELOG unless you prefer to
  remain anonymous.

If a report is declined, we will explain why. You are welcome to disagree
and provide additional context.
