<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/fast-ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/fast-ci.yml?branch=main&label=Fast%20CI" alt="Fast CI status"></a>
  <img src="https://img.shields.io/badge/Rust-1.97%2B-orange" alt="Rust 1.97+">
  <img src="https://img.shields.io/badge/protocol-Agent%20v4-blue" alt="Agent protocol v4">
  <img src="https://img.shields.io/badge/license-GPL--3.0--only-green" alt="GPL-3.0-only license">
</p>

# 这也能投 / CanISend

CanISend is a local-first framework for preparing evidence-bound Applications and submissions.
Its domain-neutral Rust kernel enforces evidence, consent, review, export, recovery, and audit
rules. Declarative workflow Packs own domain vocabulary, stages, Deliverables, templates, and
validators.

The two built-in reference Packs are `org.canisend.generic-application` and
`org.canisend.academic-job`. One neutral Workspace can contain Applications from both Packs at
the same time; each Application owns its exact Pack identity and state.

CanISend never logs in, uploads, or submits an Application. The user owns every source, approval,
export, and external submission decision.

## Current status

The checked-in source version is `1.0.0-alpha.8`. The latest publicly qualified checkpoint is `v1.0.0-alpha.8`
([Release](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.8)), built from
`35e7c822ea2f469ab726a31b5d08e622f6810c55`. Later `main` changes are not part of those published
bytes. Earlier release facts remain immutable at their tags.

Published Alpha.8 provides:

- `canisend.workspace/v4` with clean initialization, check, backup, restore, repair, and explicit
  unsupported-legacy refusal;
- Application-level Pack binding, mixed academic/generic Applications, connected source intake,
  explicit Profile Source/Evidence associations, and independent revisions;
- App-led bootstrap plus standalone CLI initialization, host setup/status/remove, basic-data
  import/read, recovery, and MCP stdio;
- sidebar Workspace/Application context, project or global Agent Skill installation, Typst Profile
  import, and verified starter resources for a new Workspace;
- `canisend.agent/v4`, schema version `4.0.0`, and generated integrity-managed Codex and Claude
  Code Skills from one canonical resource source;
- digest-bound preview/approval/commit operations for Requirement, Plan, and Deliverable work in
  one persistent App or MCP process;
- English and Simplified Chinese desktop interfaces, keyboard and 200% text support, embedded
  Typst rendering, immutable content-addressed Blobs, SQLite authority, and no default telemetry;
- five standalone CLI release targets and an Apple Silicon macOS App candidate channel.

Alpha.6-or-earlier Skills, Agent v2/v3 requests, job aliases, host-resource layouts, and Workspace
v2/v3 migration are not Alpha.7-or-later compatibility targets. They fail before mutation and
direct users to initialize a clean v4 Workspace.

The [1.0 delivery roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md) is the
execution authority. [ADR-RN-0020](docs/architecture/rust-native/decisions/0020-adopt-a-neutral-multi-application-workspace-and-new-agent-surface.md)
defines the breaking v4 boundary.

## Quick start

Build the pinned Rust source, initialize a neutral Workspace, and create Applications with exact
Pack bindings:

```console
cargo build --release --locked
./target/release/canisend version --json
./target/release/canisend doctor --json
./target/release/canisend --workspace ./my-applications workspace init --json
./target/release/canisend --workspace ./my-applications application create \
  --pack org.canisend.generic-application --candidate ./generic.json --json
./target/release/canisend --workspace ./my-applications application create \
  --pack org.canisend.academic-job --candidate ./academic.json --json
./target/release/canisend --workspace ./my-applications application list --json
```

Use the [documented quick start](docs/guides/quick-start.md) for candidate formats, shared basic
data, backup, restore, and the full guarded lifecycle.

## Codex, Claude Code, and headless use

The App is optional after initialization. Install current v4 Skills and keep one MCP process alive
for guarded mutations:

```console
./target/release/canisend --workspace ./my-applications host setup --host codex --json
./target/release/canisend --workspace ./my-applications host setup --host claude --json
./target/release/canisend --workspace ./my-applications host status --host codex --json
./target/release/canisend --workspace ./my-applications mcp serve
```

The canonical Agent sequence is:

```text
orient -> propose -> preview -> approve -> commit -> verify
```

CanISend remains the state authority. Host conversations, credentials, plugins, search, and
retention stay with the selected Agent host. See [Agent integration](docs/guides/agent-integration.md).

## User and operator guides

- [Installation](docs/guides/installation.md)
- [Release verification](docs/guides/release-verification.md)
- [Quick start](docs/guides/quick-start.md)
- [Desktop App](docs/guides/desktop-gui.md)
- [Agent integration](docs/guides/agent-integration.md)
- [Privacy and consent](docs/guides/privacy-and-consent.md)
- [Backup and recovery](docs/guides/backup-and-recovery.md)
- [Upgrade, rollback, and uninstall](docs/guides/upgrade-and-rollback.md)
- [Known limitations](docs/guides/known-limitations.md)
- [Support policy](docs/release/support-policy.md)
- [Release qualification ledger](docs/release/qualification-ledger.md)
- [Defensive assurance routing](docs/development/defensive-assurance-routing.md)

## Development checks

Use the smallest verification tier that proves a change. The final shared-contract source gate is:

```console
cargo fmt --all -- --check
cargo run -p xtask --locked -- release check
```

Fast CI owns the complete Rust, desktop UI, accessibility, Linux, Windows, and Apple Silicon macOS
suite. Native release workflows own exact packaged binaries on their declared targets.

No Python interpreter, virtual environment, PyPI package, or Pytest runner participates in the
active product.

## Architecture

```text
Codex / Claude Code / user / conforming MCP client
                       |
                 App / CLI / MCP
                       |
          Agent v4 + generated Skills
                       |
       shared Rust application facade
                       |
 domain-neutral kernel + declarative Packs
                       |
 SQLite authority + immutable Blobs + bounded I/O
```

User documents are projections or exports, not authority. Every surface uses the same application
facade and approval rules. Typst rendering is embedded in the native product.

## Product boundary

CanISend prepares local material. It does not create accounts, fill portals, acquire credentials,
send email, bypass platform controls, upload files, or submit Applications. Image-only PDF OCR,
external Pack installation, Pack marketplaces, and Windows/Linux public GUI packages are outside
the current 1.0 scope.

The archived Python implementation remains available at tag
`archive/python-v0.6.0b1-final`; the Rust product does not run it or import its Workspaces.

## License

CanISend is free software licensed under the
[GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). Corresponding source for a
release is its matching Git tag. Historical tags retain the license stated by their own source
tree.

Native bundles include [third-party notices](THIRD_PARTY_NOTICES.md), license texts, SBOM data,
checksums, and release provenance.
