# Changelog

## Unreleased — Rust-native `0.7.0-alpha.1`

- Started a greenfield Rust replacement with no Python runtime, Pytest, old-workspace, or agent-v1 compatibility.
- Archived the final Python implementation at `archive/python-v0.6.0b1-final`.
- Added the six-crate Cargo workspace and Rust `xtask` automation.
- Added the `canisend.agent/v2` response envelope and truthful compiled capability registry.
- Added native `version`, `doctor`, and `agent capabilities` commands.
- Added an embedded resource manifest with SHA-256 verification.
- Added Rust-native dependency spikes for bundled SQLite, generated schemas, PDF extraction, embedded Typst, and
  Rustls. The complete spike passed on Ubuntu, macOS, and Windows in GitHub Actions run `29608591519`.
- Replaced the active Python package, Pytest suite, schemas, resources, and publication automation with the
  Rust-native product foundation.

## Python release history

The complete historical changelog is available from the archive tag:

```text
git show archive/python-v0.6.0b1-final:CHANGELOG.md
```

The previous published beta was `v0.6.0b1`. It is not compatible with the Rust-native workspace or agent protocol.
