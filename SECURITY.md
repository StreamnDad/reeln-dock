# Security Policy

## Supported Versions

reeln-dock is pre-1.0 software. Security fixes are published against the
latest release only. We recommend always running the most recent version
from the [Releases page](https://github.com/StreamnDad/reeln-dock/releases).

| Version        | Supported          |
| -------------- | ------------------ |
| 0.x.x (latest) | :white_check_mark: |
| older          | :x:                |

## Scope

reeln-dock is a Tauri v2 desktop application (Rust backend + Svelte 5
frontend) that runs locally on a livestreamer's machine. It communicates
with `reeln-core` Rust crates and optionally shells out to `reeln-cli`
for rendering and publishing. It does not expose any network listeners
of its own.

In-scope concerns include, but are not limited to:
- Command injection via game metadata, config values, or render overrides
  passed to `reeln-cli`, `ffmpeg`, or other subprocesses
- Path traversal or unsafe file handling in game state, render queues,
  output directories, or config overrides
- Credential leakage — OAuth tokens, API keys, or refresh tokens written
  to logs, caches, or error messages in plain text
- IPC boundary violations — Tauri command handlers that bypass validation
  or expose unintended filesystem access
- Unsafe loading or deserialization of state, render queue, or config
  files (JSON)
- CSP bypass or XSS in the Svelte frontend webview
- Arbitrary code execution via the plugin discovery mechanism

Out of scope:
- Vulnerabilities in individual plugins (`reeln-plugin-*`) — report those
  to the respective plugin repository
- Vulnerabilities in `reeln-cli` or `reeln-core` — report those to their
  respective repositories
- Vulnerabilities in third-party APIs (YouTube, Meta, TikTok, OpenAI,
  Cloudflare) or in tools the app invokes (`ffmpeg`, `obs`)
- Issues that require an attacker to already have local code execution
  on the user's machine

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub
issues, discussions, or pull requests.**

Report vulnerabilities using GitHub's private vulnerability reporting:

1. Go to the [Security tab](https://github.com/StreamnDad/reeln-dock/security)
   of this repository
2. Click **"Report a vulnerability"**
3. Fill in as much detail as you can: affected version, reproduction steps,
   impact, and any suggested mitigation

If you cannot use GitHub's reporting, email **git-security@email.remitz.us**
instead.

### What to include

A good report contains:
- The version of reeln-dock you tested against
- Your operating system and architecture (macOS / Windows / Linux, arch)
- Steps to reproduce the issue
- What you expected to happen vs. what actually happened
- The potential impact (credential leakage, code execution, data loss,
  denial of service, etc.)
- Any proof-of-concept code, if applicable

### What to expect

reeln-dock is maintained by a small team, so all timelines below are
best-effort rather than hard guarantees:

- **Acknowledgement:** typically within a week of your report
- **Initial assessment:** usually within two to three weeks, including
  whether we consider the report in scope and our planned next steps
- **Status updates:** roughly every few weeks until the issue is resolved
- **Fix & disclosure:** coordinated with you. We aim to ship a patch
  release reasonably quickly for high-severity issues, with lower-severity
  issues addressed in a future release. Credit will be given in the
  release notes and CHANGELOG unless you prefer to remain anonymous.

If a report is declined, we will explain why. You are welcome to disagree
and provide additional context.
