# Engineering Standards for AI Agents & Contributors (`AGENTS.md`)

This document defines the binding engineering rules for any human contributor, AI assistant, or automated workflow working on **DrakeStar-Launcher**.

## 1. Repository Architecture

- **Rust workspace (`crates/`):**
  - `drakestar-mc`: Minecraft versions, accounts (Microsoft / Ely.by / Offline), Java runtime management, Horizon native engine.
  - `drakestar-core`: Fabric/vanilla launching, instance profiles, Fused / local monolith integration.
  - `drakestar-net`: HTTP downloads, SHA-256 verification, release feeds.
  - `drakestar-app`: Application state, mod sync, settings, release orchestration.
  - `drakestar-cli`: Headless CLI entry point.
  - `drakestar-rust-client`: Native Horizon client bridge.
- **Desktop shell (`src-tauri/`):** Tauri v2 backend commands, signed updater (`minisign`), OS integrations.
- **Frontend (`ui/`):** TypeScript + Vite UI and Playwright Chromium E2E suite (`npm --prefix ui run test:browser`).
- **Release tooling (`tools/bump-release.mjs`):** Synchronizes versions across `Cargo.toml`, `src-tauri/tauri.conf.json`, and `ui/package.json`.

## 2. Mandatory Verification Before Opening or Merging a PR

Every change must pass local verification matching CI (`.github/workflows/ci.yml`):

1. **Rust tests:**
   ```bash
   cargo test -p drakestar-mc -p drakestar-core -p drakestar-net -p drakestar-app -p drakestar-cli
   ```
2. **Frontend build & browser tests (when touching `ui/` or IPC):**
   ```bash
   npm --prefix ui ci
   npm --prefix ui run build
   npm --prefix ui run test:browser
   ```
3. **Lockfile integrity:** Never add a root `package-lock.json`-less `package.json` that breaks `npm ci`, and keep `Cargo.lock` synced so `--locked` builds succeed.
4. **Preserve existing auth & account variants:** Never remove or alter account providers (`Microsoft`, `Elyby`, `Offline`) or secret-handling formats without explicit migration and tests.

## 3. Git, Commits & Releases

- Keep commits small, focused, and verified. Do not add AI `Co-Authored-By:` trailers.
- **Never build release binaries by hand:** bump versions with `node tools/bump-release.mjs X.Y.Z` and let GitHub Actions build and sign artifacts for Windows, Linux, and macOS (see [docs/PUBLICAR.md](docs/PUBLICAR.md)).
- If `graphify-out/` is present, query the knowledge graph before scanning large files.

## 4. Security & Privacy Rules

- Never log, commit, or transmit tokens, `accounts/secrets.json`, or private signing keys.
- Any external download (Java, Fabric, mods, Horizon binaries) must verify checksums (SHA-256) before extraction or execution.
