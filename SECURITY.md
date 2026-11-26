# Security Policy

## Supported Versions

Only the latest signed release published under [GitHub Releases](https://github.com/SlimefunNewHorizons/DrakeStar-Launcher/releases/latest) receives security updates.

## Security & Privacy Guarantees

- **Credential Isolation:** Microsoft / Ely.by authentication tokens are stored separately from account metadata (`accounts/secrets.json`, user-only permissions) and are never written to application logs or crash reports.
- **Sanitized Telemetry:** Automatic crash reports (opt-out under *Settings → Privacy*) redact local paths, email addresses, IP addresses, and secrets before submission.
- **Signed Releases:** All official binaries (`.exe`, `.msi`, `.deb`, `.rpm`, `.AppImage`, `.dmg`) are built exclusively via GitHub Actions with `--locked` dependencies and signed with `minisign`.

## Reporting a Vulnerability

Please **do not** open public issues for security vulnerabilities.

1. Use [GitHub Private Vulnerability Reporting](https://github.com/SlimefunNewHorizons/DrakeStar-Launcher/security/advisories/new) on this repository, or
2. Contact the maintainers (`JackStar6677-1` / `DrgX89`) privately via the official DrakesCraft Discord.

Include reproduction steps, affected version, OS, and (if possible) a minimal test case. We aim to acknowledge reports within 48 hours.
