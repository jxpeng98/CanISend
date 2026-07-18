# Alpha Issue Collection

CanISend does not enable default telemetry, analytics, crash upload, or background network reporting. Alpha evidence
comes from explicit GitHub issues and deliberately sanitized dogfood notes.

Before filing an issue:

1. Reproduce with the exact released archive and include its target triple and CanISend version.
2. Run `canisend doctor --json` and include only public capability/diagnostic fields.
3. Replace job titles, institutions, URLs, application text, profile evidence, file-system paths, and IDs with
   synthetic values.
4. Never attach a workspace, backup, task-input export, application package, provider request, token, or credential.
5. State whether the problem could cause data loss, a privacy/security boundary failure, protocol incompatibility,
   incorrect evidence attribution, or rendering corruption.

Maintainers triage those five blocker classes before beta. Feature requests remain lower priority during release
hardening.
