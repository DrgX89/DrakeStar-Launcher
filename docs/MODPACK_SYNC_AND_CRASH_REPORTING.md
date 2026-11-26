# Server-Authoritative Modpack Sync, Client Audit & Privacy Specification

Technical and governance specification for **DrakeStar-Launcher** covering server-authoritative modpack synchronization, transparent client-added mod auditing, automated crash triage, and security/privacy guarantees.

> **Status:** Approved design specification for upcoming releases (`v0.7+`). Any contributor or AI agent implementing these features must follow this document and [AGENTS.md](../AGENTS.md).

---

## 1. Core Principle: Server as Single Source of Truth

Analogous to **Odysseia**'s live resource-pack reload (`pack.drakescraft.cl`), the server (future dedicated **Oracle** node or **Star** backend behind Cloudflare) is the single source of truth for the official modpack:

- **Strict version pinning:** If the server runs `LuckPerms v1` even when `v2` is published on Modrinth/CurseForge, the client downloads and keeps exactly `LuckPerms v1` from the server's configured mirror.
- **Zero manual modpack re-packaging:** Adding, patching, or **removing** a mod on the server only requires updating `mods-manifest.json`. On the next **PLAY** click, the client syncs automatically before launching Minecraft.
- **Separation of server-managed vs. player-added mods:**
  - The launcher tracks server-managed JARs locally (`.drakestar-managed.json` or a separate directory for player extras).
  - If a `.jar` was in the server manifest and was removed from the server, the launcher **deletes it**.
  - If a `.jar` was manually added by the player (e.g., cosmetics or performance) and was not server-managed, the launcher **keeps it**, classifies it as a **player-added mod**, and applies the policy in Section 3.

---

## 2. Sync Flow on PLAY

```text
[Click PLAY]
   │
   ▼
GET https://<modpack-host>/mods-manifest.json   (ETag / SHA-256 + bounded timeout)
   │
   ├── 1. Sync server-managed mods:
   │      • Download new/changed JARs to .tmp and verify SHA-256
   │      • Atomically move into mods/ and update .drakestar-managed.json
   │      • Remove managed JARs from mods/ that are no longer in the manifest
   │
   ├── 2. Inspect player-added mods (if any):
   │      • Compute SHA-256 + read mod metadata (mod_id, version) from JAR
   │      • Evaluate against policy.allowed / policy.blocked in manifest
   │
   └── 3. Warn / Block / Launch
```

### `mods-manifest.json` Contract

```json
{
  "schema": 1,
  "pack_id": "drakescraft-mods",
  "revision": "2026.11.26-01",
  "minecraft": "1.21.1",
  "loader": { "name": "neoforge", "version": "21.1.x" },
  "mods": [
    {
      "id": "create",
      "file": "create-1.21.1-x.y.z.jar",
      "sha256": "<hex-64>",
      "size": 12345678,
      "url": "https://cdn.drakescraft.cl/modpack/mods/create-1.21.1-x.y.z.jar",
      "side": "both"
    }
  ],
  "policy": {
    "allowed_client_sha256": ["<hash-iris>", "<hash-sodium>"],
    "blocked_ids": ["baritone", "meteor-client", "wurst", "aristois"],
    "blocked_sha256": ["<hash-xray>"]
  }
}
```

---

## 3. Player-Added Client Mods (Low Friction, Zero Alert Spam)

When extra JARs added by the player are detected outside the server pack:

| Classification | Behavior in DrakeStar-Launcher |
| :--- | :--- |
| **Allowed** (`allowed_client_sha256`) | Launches cleanly with no extra prompt (approved cosmetics, shaders, performance). |
| **Blocked** (`blocked_ids` / `blocked_sha256`) | **Blocks PLAY** to the official server and names the exact `.jar` to remove. |
| **Unknown** (not yet listed) | Shows a clear pre-launch warning: *"Server mod sync finished, but player-added mod(s) were detected. Official server compatibility and support are not guaranteed (e.g., some Litematica features require server-side support). Recommended for cosmetics/performance only."* |

### Deterministic Server-Side Audit (Star / Future Oracle — No AI by Default)

When launching into the **official server** (never in local/single-player mode) after accepting server terms, the launcher transmits only:
`username`, `account_type` (`premium` | `offline`), `pack_revision`, and the list of **player-added mods** (`file`, `mod_id`, `version`, `sha256`).

On the receiving server (**Star** behind Cloudflare Tunnel, or a future isolated **Oracle** node), resolution runs in **two stages without alert spam**:

1. **Stage 1 — 100% deterministic (zero AI):**
   - Check local cache by `sha256`.
   - On cache miss, query **Modrinth API** (`GET /v2/version_file/{sha256}?algorithm=sha256`) and **CurseForge API** (fingerprint/hash) to resolve the official project name, author, category, and source URL.
   - Persist the resolved metadata keyed by `sha256`.
2. **Stage 2 — Optional AI web lookup (cache-miss only, deduplicated):**
   - Only if the `sha256` is absent from both Modrinth and CurseForge (e.g., custom or manually compiled mod), SAORI may run **a single AI-assisted web lookup per unseen `sha256`** for context.
   - **No alert floods:** Never send emails or notifications per connection. Store entries in a consolidated, `sha256`-deduplicated digest log and surface only high-risk flags or entries correlated with an in-game infraction.

---

## 4. Error Logs & Crash Triage Pipeline

Extends DrakeStar's existing logging and crash reporting:

- **Explicit fault attribution:** Distinguishes **launcher** crashes (Tauri/Rust/UI), native **Horizon** engine (Rust) crashes, and **Minecraft / Mod** crashes (`exit != 0`, `crash-reports/`, `hs_err_pid*.log`).
- **Modified-client tagging:** If player-added mods were loaded during the crashed session, the payload sets `client_modified: true` with those mod IDs/hashes so modpack bugs and user-mod conflicts are never mixed up.
- **Purpose & destination:** Sanitized logs (user paths, emails, IPs, and tokens redacted) are sent to **Star (SAORI)** solely to diagnose and fix launcher/modpack issues, deduplicating by stack signature before creating or updating a GitHub issue.

---

## 5. Security & Hardening (Protecting Star / Future Oracle)

All client and server implementations must enforce:

1. **Zero secrets in the client:** DrakeStar-Launcher never embeds database credentials, GitHub tokens, private CurseForge/Modrinth keys, or Star admin tokens.
2. **Strict input validation:** The server ingest endpoint enforces strict JSON schema validation, max payload size (e.g., `<= 64 KB`), item caps, and strict regexes on `sha256` (`^[a-f0-9]{64}$`), `mod_id`, and `username`.
3. **Rate limiting & WAF:** Per-IP and per-account rate limits via Cloudflare WAF and server middleware to prevent log flooding or DoS.
4. **Untrusted-input isolation:** All client strings (`file`, `mod_id`, crash log lines) are treated as **untrusted data** — never interpolated into shell commands or SQL, escaped in UI/issues, and sanitized against prompt injection before any optional AI analysis.
5. **Production isolation:** Neither crash ingestion nor modpack sync has write access to Dallas production (`DrakesPurpur`).

---

## 6. Transparency, Privacy & Non-Affiliation (Launcher + GitHub + `drakescraft.cl`)

Launcher UI screens, this repository, and the legal pages on **`drakescraft.cl`** (`/terms.html`, `/terms-en.html`, `/privacidad.html`, `/privacidad-en.html`, `/reglas.html`) must stay aligned on:

1. **Non-profit & open-source:** DrakeStar-Launcher is a non-profit, open-source (**MIT**) community project; anyone can inspect the exact data transmitted on GitHub.
2. **Exact data disclosure:**
   - **Crash/error reports** (toggleable in *Settings → Privacy*): launcher/Minecraft version, OS, modified-client flag, and redacted log excerpts, used **solely** to fix bugs and improve stability.
   - **Extra-mod integrity check on official server launch** (not used in local/single-player): `username` (premium or offline, solely to identify the server account) plus filenames, versions, and SHA-256 hashes of extra player-added mods.
3. **Mojang / Microsoft non-affiliation:**
   - *DrakeStar-Launcher and DrakesCraft Labs are independent, non-profit community projects. **NOT an official Minecraft product. Not approved by or associated with Mojang AB or Microsoft Corporation.***
   - Applies to both standard Java Minecraft launching (Vanilla/Fabric/NeoForge) and the experimental Rust client (**Horizon**).
