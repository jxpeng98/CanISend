# Changelog

## Unreleased — toward `1.0.0-alpha.6`

- Reframed CanISend as a domain-neutral, local-first framework whose workflow Packs own domain
  vocabulary, stages, Deliverables, templates, and Validators.
- Added typed `canisend.workflow-pack/v1`, canonical embedded Academic and Generic Packs, stable
  Pack-qualified stage and Deliverable identifiers, exact Pack digests, and bounded registries.
- Added neutral Agent/Workspace v3 Application contracts and a dry-run-first, verified-backup,
  failure-atomic Workspace v2→v3 migration that preserves Academic Pack authority.
- Added Generic Pack CLI, MCP, and desktop flows while retaining bounded Agent v2, `job`, and
  `jobs/JOB_ID` compatibility for migrated academic Applications.
- Unified approval/preview storage, operation mapping, semantic parity, dependency-graph checks,
  cross-platform core CI, and browser keyboard/accessibility gates.
- Adopted `GPL-3.0-only` for current CanISend-authored source and future releases without changing
  the license facts of published `v1.0.0-alpha.5` or older tags.
- Added sequential Alpha/RC release planning, derived release status, workflow-default checks,
  snapshot-declared feedback Roadmaps, and exact candidate promotion without recompilation.

The checked-in version remains `1.0.0-alpha.5` until the Roadmap authorizes the atomic Alpha.6
transition. These entries describe unqualified post-tag source and are not Alpha.6 release notes.

## Published `v1.0.0-alpha.5`

`v1.0.0-alpha.5` is the latest publicly qualified checkpoint. Its exact release notes, manifest,
artifacts, license facts, and qualification evidence remain immutable at the tag.

## Historical Rust-native `0.7.0-alpha.1` development

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
