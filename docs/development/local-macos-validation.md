# Local-only macOS builds and tests

Owner decision, 2026-09-15: all macOS builds and tests run locally, including npm,
PyPI, native CLI archives, desktop ZIP/DMG, Intel compilation, Homebrew, upgrade,
and WKWebView checks. Do not substitute a self-hosted Actions runner. Linux and
Windows retain their existing GitHub Actions jobs and pause/enable settings.

## Development and CLI packages

Use the pinned Rust toolchain and native Xcode command-line tools, plus `jq`.
Desktop work additionally uses Node 26.5.0 and pnpm 11.17.0. npm requires Node/npm;
PyPI requires Python's packaging tools (`maturin==1.15.0`, `twine==6.2.0`) in an
explicitly prepared local virtual environment, not a Python product runtime.

From the repository root:

```sh
bash scripts/check_macos_local.sh             # formatting, Clippy, source, CLI/shared tests and Host/MCP smokes
bash scripts/check_macos_local.sh desktop     # opt-in UI and native GUI development checks
bash scripts/check_macos_local.sh package     # signed CLI archive plus exact extracted-archive lifecycle
bash scripts/check_macos_local.sh npm         # archive plus offline npm install, byte comparison and MCP/upgrade tests
bash scripts/check_macos_local.sh pypi        # ARM64 wheel, isolated install and MCP/upgrade tests
```

Package modes require a clean committed tree so the embedded source archive and
binary refer to the same source. Results, logs, source revision/patch and package
hashes are retained in a fresh `dist/local-macos-*/` directory. No mode publishes,
installs globally, changes a real Workspace, or produces a GitHub qualification
record. `cli` also works on a local Intel Mac; ARM64 checks do not prove Intel
runtime support. Do not set `CARGO_BUILD_TARGET` for these native checks.

## Deferred desktop and native qualification

GUI work remains paused. When explicitly resumed, use the existing local tools:

- Native Intel GUI compile: run the `desktop` mode on an Intel Mac, then
  `cargo build --locked --release -p canisend-gui --features canisend-gui/custom-protocol`.
  Cross-compilation from ARM64 is compile-only evidence, never an Intel runtime pass.
- ARM64 desktop packages: after `desktop`, build with
  `cargo build --locked --release -p canisend-gui --features canisend-gui/custom-protocol`,
  then `bash scripts/package_macos_gui_release.sh target/release/canisend-gui NEW_OUTPUT`.
  Use `scripts/smoke_macos_gui_release_archive.sh ZIP NEW_SMOKE_DIR` and
  `scripts/smoke_macos_gui_dmg.sh DMG NEW_SMOKE_DIR` on those exact bytes.
- Native preview: follow [native PDF preview](../../tools/native-preview/README.md), on the local
  WKWebView host. Browser tests do not replace native preview evidence.
- Homebrew and archive upgrade: the existing `qualify_homebrew_packages.sh` and
  `qualify_archive_upgrade.sh` still require GitHub-bound evidence identity.
  Their local qualification adapter is pending. Do not fabricate `GITHUB_RUN_ID`
  or label a local machine `macos-15`; Homebrew lifecycle tests also need an
  explicitly authorized disposable host to avoid replacing a user's installation.

## Remote and release boundaries

Fast CI still runs Linux/Windows core tests. Common formatting, Clippy, source
invariants and the CLI/shared workspace suite run on Ubuntu. Their legacy names
`macos-quality` and `macos-tests` are retained solely because the protected branch
requires those exact check contexts; they are **not macOS evidence**. Desktop
macOS jobs are hard-disabled, not switchable through repository variables.

Mixed matrices contain only Linux/Windows entries. Retained macOS-only jobs use
literal `if: ${{ false }}`; their bodies document the previous procedures and
cannot allocate runners. The source gate rejects runnable macOS runner entries.
Archived snapshots and historical release evidence remain unchanged.

Local artifact ingestion and provenance for formal release qualification are not
yet implemented. `release.yml` fails its local-macOS prerequisite and prevents
promotion/publication; old npm/PyPI macOS publishers are disabled. Existing
five-target and native-preview completeness checks stay strict, so incomplete
Linux/Windows-only evidence cannot qualify a full release. No support target,
security check, real-user acceptance requirement, or evidence record is waived.

Before the next full native publication, implement and test exact local-artifact/evidence
handoff; automated promotion and formal macOS qualification are **not ready**. An independently
authorized registry-only testing prerelease can use the bounded local publish-only path: first
review an exact clean-tree candidate, run
`bash scripts/publish_local_macos_candidate.sh verify npm|pypi CANDIDATE_DIRECTORY`, then set
`CANISEND_PUBLISH=yes` and replace `verify` with `publish`. The publish command rechecks source
identity and candidate hashes, publishes only that archive, verifies the registry digest, and
performs a fresh registry install. It does not create a tag, GitHub release, full qualification,
or Host evidence. Committing these changes alone does not update remote workflows;
protected PR integration is required, and already-running jobs are unaffected.
