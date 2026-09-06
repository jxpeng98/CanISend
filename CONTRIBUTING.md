# Contributing to Rust-Native CanISend

## Toolchain

Use the toolchain pinned in `rust-toolchain.toml`. CanISend's product, build, tests, and release
artifacts do not depend on Python. Retained historical development scripts under `.trellis/` are
optional; no Trellis installation, hook, or task runner is required to contribute.

## CLI-first development

From the repository root, Cargo selects `canisend-cli` by default:

```sh
cargo build --locked
cargo run --locked -- --help
cargo run --locked -- doctor --json
cargo test --locked
```

These commands build the native CLI and its Rust dependencies, including embedded resources;
they require the pinned Rust toolchain and the platform's native C/C++ build tools, but no Node.js,
pnpm, frontend build, or desktop webview SDK. Default tests cover the CLI package; use an explicit
`-p` selection for another crate or `--workspace` for the full suite. The shared `canisend-app`
crate is the domain facade and remains a CLI dependency.

Desktop development is an explicit selection (`-p canisend-gui`) with its existing frontend and
platform prerequisites; follow [the desktop guide](docs/guides/desktop-gui.md). Full-workspace CI
continues to build and test the desktop. Existing Linux/Windows core CI additionally checks the
CLI-only default selection and dependency graph, builds it without frontend steps, and runs the
existing CLI/Host/MCP tests. These source checks do not qualify standalone release packages.

## Minimum sufficient checks

Choose the smallest row that owns the change. Do not run every row for every edit.

| Change | Local check before commit | Higher owner |
|---|---|---|
| Prose or historical note only | `git diff --check` | Documentation/source gate when the file is active release truth |
| Rust leaf behavior | affected test or test filter, `cargo fmt --all -- --check`, affected-package Clippy | Fast CI runs the workspace suite |
| Shared contract, schema, resource, CI, or release metadata | smallest affected test plus `cargo run -p xtask --locked -- release check` once on the final PR head | Fast CI |
| Desktop behavior | affected pnpm test/check plus production build only when bundling changed | Fast CI accessibility and macOS lanes |
| Package/runtime behavior | exact affected package smoke | Native candidate workflow |
| Release candidate | no ad hoc local matrix | Build-once native and public-verification workflows |

One invariant has one primary test owner. Other adapters receive a wiring/parity smoke, not copies of
the same business-rule test. A trust-boundary, consent, data-loss, recovery, or release-integrity
change keeps one positive and one negative regression at the lowest owning layer.

## Architecture

- Keep dependencies pointed inward according to ADR-RN-0002.
- Put public versioned JSON types in `canisend-contracts`.
- Keep domain rules and port traits in `canisend-core`.
- Keep SQLite/blob details in `canisend-store`.
- Keep HTTP, parsers, providers, and rendering in `canisend-io`.
- Do not let agent hosts write `.canisend/` internal state.
- Do not add a Python runtime or test dependency.

New dependencies require a documented purpose, compatible license, and evidence that they do not introduce an
unplanned end-user runtime.

## Changes and tracking

Update the Rust-native roadmap when a tracked task is completed. Add a dated note for phase transitions, dependency
decisions, material risks, and release evidence. Commits use Conventional Commits and should represent one auditable
milestone.

## Project control

Use [AGENTS.md](AGENTS.md), the [project control guide](.trellis/spec/guides/project-control.md),
and the existing plan for the current outcome. Record the scope, acceptance, smallest validation,
and next step once. Add a separate design only when an unresolved contract needs it; do not create
parallel task records, repeated approval rounds, or journal-only commits for routine work.

The owner removed project-local Trellis skills, hooks, and agents on 2026-09-04. Existing plans,
specs, and journals remain readable project material, including active plans under `.trellis/tasks/`.
Do not run `trellis init` or `trellis update` unless explicitly reinstalling that workflow.

Keep Roadmap IDs, GitHub Issues/milestones, exact evidence, and feature-freeze exceptions aligned.
For a local change without a public work item, record that projection as pending; publishing or
messaging externally requires the corresponding authority. Source completion, protected CI,
artifact qualification, and user validation remain separate facts.
