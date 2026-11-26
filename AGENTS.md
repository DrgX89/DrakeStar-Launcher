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

## 5. Planned Modpack Sync, Client Audit & Transparency (`v0.7+`)

Before implementing or modifying mod synchronization (`drakestar-app/src/modsync.rs`), crash telemetry, or official-server integrity checks, **read [docs/MODPACK_SYNC_AND_CRASH_REPORTING.md](docs/MODPACK_SYNC_AND_CRASH_REPORTING.md)** and enforce these invariants:

- **Server-Authoritative Sync:** Never pull newer external mod versions over the server's `mods-manifest.json` pins; verify SHA-256 before moving any JAR into `mods/`, and prune removed server mods without deleting player-added client mods (warn on unknown extra mods, block known cheats).
- **Deterministic First, Zero Spam:** On the receiving backend (Star / future Oracle), resolve extra client mod `sha256` hashes deterministically via Modrinth/CurseForge APIs with local caching; only run an optional AI web lookup once per unseen `sha256` with deduplicated digest logging (no alert floods).
- **Hardened Ingestion & Zero Client Secrets:** Never embed server/API secrets in the launcher; enforce strict JSON schema/size limits, rate limiting, and untrusted-input sanitization (never interpolate client strings into shell/SQL/AI prompts).
- **Transparency & Non-Affiliation:** Keep launcher UI disclosures, [README_ES.md](README_ES.md)/[README.md](README.md), and `drakescraft.cl` (`/privacidad.html`, `/terms.html`, `/reglas.html`) aligned: open-source non-profit (MIT), exact disclosure of crash logs + extra-mod audit fields (`username` + hashes on official server launch only), and explicit Mojang/Microsoft non-affiliation for both Java and Horizon (Rust) engines.
