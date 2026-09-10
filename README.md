<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/fast-ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/fast-ci.yml?branch=main&label=Fast%20CI" alt="Fast CI status"></a>
  <img src="https://img.shields.io/badge/license-GPL--3.0--only-green" alt="GPL-3.0-only license">
</p>

# 这也能投 / CanISend

Turn an application brief and your own records into reviewed documents. Work with **Codex or
Claude Code** to understand requirements, prepare drafts supported by your evidence, and export
local files for you to submit.

CanISend supports academic jobs and general applications such as grants and proposals. Your
workspace keeps sources, drafts and review history together so you can resume later. PDF
rendering, templates and fonts are built in.

## Install

### npm — macOS Apple Silicon

Requires Node.js **22.14 or newer**. Install the current testing release:

```sh
npm install -g canisend@next
canisend version
canisend doctor
```

`next` currently provides **1.0.0-beta.5**; the default `latest` tag may be older.
The package includes the native CLI and its resources; no Rust toolchain is needed.

### Download a binary — macOS, Linux or Windows

These direct downloads are from [GitHub Release v1.0.0-beta.1](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-beta.1),
an older native release than npm `next`. They require **no Node.js, Python or Rust installation**.

| Platform | Download |
| --- | --- |
| macOS Apple Silicon | [ARM64 · tar.gz](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/canisend-1.0.0-beta.1-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [x64 · tar.gz](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/canisend-1.0.0-beta.1-x86_64-apple-darwin.tar.gz) |
| Linux x64, glibc | [x64 · tar.gz](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/canisend-1.0.0-beta.1-x86_64-unknown-linux-gnu.tar.gz) |
| Linux x64, musl / static | [x64 · tar.gz](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/canisend-1.0.0-beta.1-x86_64-unknown-linux-musl.tar.gz) |
| Windows x64 | [x64 · zip](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/canisend-1.0.0-beta.1-x86_64-pc-windows-msvc.zip) |

Download [SHA256SUMS](https://github.com/jxpeng98/CanISend/releases/download/v1.0.0-beta.1/SHA256SUMS)
from the same release and follow the [verification steps](docs/guides/release-verification.md).
Extract the archive, put `canisend` (`canisend.exe` on Windows) on your `PATH`, then run
`canisend version` and `canisend doctor`. See [installation details](docs/guides/installation.md)
for checksum commands, platform selection and signing information.

## Start with Codex or Claude Code

Use a private directory for your applications. These commands work with both download channels:

```sh
canisend --workspace ./applications workspace init --json
canisend --workspace ./applications host setup --host codex
```

For Claude Code, replace `--host codex` with `--host claude`.
**Run the MCP registration command printed by setup**, then open `./applications` in your Host
and reconnect to load the CanISend tools. Setup installs Skills; registration connects the tools.
The desktop App is optional. See [Agent integration](docs/guides/agent-integration.md) if needed.

Provide the opportunity text and your relevant records, then ask:

> Use CanISend to prepare my application for this opportunity. Identify the requirements and
> missing evidence, draft the requested documents using my records, review them, and export
> PDFs in this workspace. Ask me for missing facts and required approvals. Show me the final
> files and any unresolved gaps.

For academic jobs, include the advert and your CV or profile records. You should receive a
requirements summary, supported drafts, and local files to review and submit yourself.
Return to the same workspace to continue or revise an application.

CanISend never logs in, uploads or submits applications. Workspace data stays local; material
you share with an AI Host is subject to that Host's data settings.

## Upgrade

Back up your workspace, then install the newer npm version or replace the downloaded binary.
Refresh Skills using the new executable:

```sh
canisend --workspace ./applications workspace backup ./applications-backup
# Install the new version, then:
canisend --workspace ./applications host setup --host codex
canisend --workspace ./applications workspace check
```

Use a new backup directory each time. Reconnect your Host and update its MCP registration if
the executable path changed. Setup preserves locally edited Skills. Built-in templates update
with the CLI; existing applications retain their bound template version and history.
See [upgrade and recovery](docs/guides/upgrade-and-rollback.md).

## Current status

The checked-in source version is `1.0.0-beta.6`, prepared locally but not published.
The latest publicly qualified checkpoint is `v1.0.0-beta.1`; npm testing releases are a
separate channel. CLI development is active; GUI work and full native qualification are paused.
See [release status](RELEASE.md) for exact versions, artifacts and remaining gates.

## More

- [Detailed CLI walkthrough](docs/guides/quick-start.md) · [Troubleshooting](docs/guides/troubleshooting.md)
- [Privacy and consent](docs/guides/privacy-and-consent.md) · [Backup and recovery](docs/guides/backup-and-recovery.md)
- [Known limitations](docs/guides/known-limitations.md) · [Report a problem](https://github.com/jxpeng98/CanISend/issues)
- [Contributing](CONTRIBUTING.md) · [1.0 roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md)

<details>
<summary>Development</summary>

<img src="https://img.shields.io/badge/Rust-1.97%2B-orange" alt="Rust 1.97+">

A domain-neutral Rust kernel enforces evidence, consent and recovery rules.
The built-in Packs are `org.canisend.generic-application` and `org.canisend.academic-job`.
Build only the CLI with `cargo build --release --locked -p canisend`; use
`cargo run -p xtask --locked -- source check` for the shared source gate.

</details>

Licensed under [GPL-3.0-only](LICENSE). Downloads include license texts and
[third-party notices](THIRD_PARTY_NOTICES.md).
