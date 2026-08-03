<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/fast-ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/fast-ci.yml?branch=main&label=macOS%20Fast%20CI" alt="macOS Fast CI status"></a>
  <img src="https://img.shields.io/badge/Rust-1.97%2B-orange" alt="Rust 1.97+">
  <img src="https://img.shields.io/badge/protocol-Agent%20v2%20%2B%20v3-blue" alt="Agent protocols v2 and v3">
  <img src="https://img.shields.io/badge/license-GPL--3.0--only-green" alt="GPL-3.0-only license">
</p>

# 这也能投 / CanISend

CanISend is a local-first, evidence-constrained framework for preparing Applications and other
evidence-bound submissions. Its domain-neutral Rust kernel combines exact-bound workflow Packs
with user-controlled sources to support Requirements, confirmed Evidence, fit, planning,
Pack-defined Deliverables, review, rendering, export, backup, recovery, and auditable Agent
collaboration through one set of application services.

The source includes two embedded Packs. `org.canisend.generic-application` is the canonical
Workspace/Agent v3 default; `org.canisend.academic-job` preserves the established academic journey
through bounded Workspace/Agent v2 compatibility. A Workspace is bound to one exact Pack identity
and digest. The latest publicly qualified checkpoint is `v1.0.0-alpha.5`; post-tag changes on
`main` are not a published Alpha.6 or Alpha.7 until their exact artifacts pass release gates.

The active product no longer uses Python or Pytest. The final Python implementation remains available only through
the Git tag `archive/python-v0.6.0b1-final`.

The [generic framework 1.0 roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md) is the
top-level execution authority. The [transition plan](docs/superpowers/plans/2026-08-02-generic-framework-transition-plan.md)
defines the pack, migration, compatibility, and dual-pack implementation slices.

## User guides

- [Installation](docs/guides/installation.md)
- [Release verification](docs/guides/release-verification.md)
- [Rust-native 1.0 support policy](docs/release/support-policy.md)
- [Native release qualification ledger](docs/release/qualification-ledger.md)
- [Defensive assurance task routing](docs/development/defensive-assurance-routing.md)
- [Quick start](docs/guides/quick-start.md)
- [Agent integration](docs/guides/agent-integration.md)
- [Privacy and consent](docs/guides/privacy-and-consent.md)
- [Backup and recovery](docs/guides/backup-and-recovery.md)
- [Upgrade, rollback, and uninstall](docs/guides/upgrade-and-rollback.md)
- [Known limitations](docs/guides/known-limitations.md)
- [Troubleshooting](docs/guides/troubleshooting.md)
- [Desktop GUI preview](docs/guides/desktop-gui.md)
- [Synthetic generic Application examples](docs/testing/generic-application-examples.md)

## Current status

The checked-in source version is `1.0.0-alpha.6`; public identity remains separately controlled by
the exact tag, package manifest, and qualification evidence. The working `main` line contains
additional unqualified post-tag product bytes. The `0.7` evidence and every published Alpha.5
artifact remain immutable history. The current source contains the domain-neutral Pack kernel,
canonical Generic and Academic Packs, neutral Application contracts, Workspace v2→v3 migration,
bounded academic compatibility, and shared CLI/MCP/Tauri application services. This is candidate
readiness work, not an Alpha.6 support claim.

The desktop supports persistent English and Simplified Chinese interfaces, system CJK font
fallback, localized native accessibility names, exact Pack-driven vocabulary, and safe management
of the version-matched CLI in a user-owned terminal location. Public Windows and Linux GUI
distribution remains unqualified; the public macOS channel is Apple Silicon only.
The current CLI provides:

- Standalone `canisend` executable archives for five native targets.
- Validated UUIDv7, SHA-256, revision, UTC timestamp, and safe relative-path contract types.
- `canisend.agent/v2` success/error envelopes, stable error registry, and grouped exit policy.
- Product/version/build inspection.
- Forty deterministic Agent v2 Draft 2020-12 schemas generated from Rust types.
- A separately versioned `canisend.workflow-pack/v1` manifest Schema, bounded semantic validator,
  canonical bundle verifier, and exact-version in-memory registry; runtime Pack installation is
  not yet enabled.
- Seven deterministic `canisend.application-model/v3` schemas plus an additive transactional
  repository for neutral Opportunity, Application, Requirement, Plan, Deliverable, and exact Pack
  bindings. A dry-run-first, verified-backup Workspace v2→v3 migration service can activate that
  authority against an exact verified academic Pack. CLI and desktop expose the generic Pack flow,
  while Agent v3/MCP adds body-free context, exact next actions, guarded mutations, consented
  review/export, and snapshot-bound single-use approval for the exact generic Pack.
- A deterministic Pack stage-graph compiler with Pack-qualified stable `StageId` values; the
  current fixed Agent/Workspace v2 workflow remains the compatibility runtime.
- A Pack-qualified Deliverable catalog that binds cardinality, declaration order, templates,
  Renderers, and Validators; fixed academic `DocumentKind` values remain v2 compatibility types.
- A digest-bound Pack localization runtime that maps the existing `en`/`zh-CN` preference to exact,
  compatible, or Pack-default vocabulary and labels without leaking one Pack's selection to another.
- A pre-parse bounded Pack byte verifier with UTF-8 data-only resource policy and body-free Trust
  Reports; publisher authentication, signatures, and external installation remain unavailable.
- Seventy-five typed embedded schemas, prompts, templates, examples, and host assets with SHA-256 verification.
- A truthful capability registry that marks unfinished functions as `planned`.
- Agent context plus schema/resource diagnostics with deterministic JSON snapshots.
- Workspace discovery, explicit `--workspace` resolution, initialization, status, integrity checks, and repair.
- Bundled SQLite authority with immutable SHA-256 blobs, revisions, dependency invalidation, and audit events.
- Verified workspace backup and failure-cleaned restore with referenced-blob manifests and deterministic projection
  reconstruction.
- Transactional job creation, inspection, listing, archival, and revision history.
- Bounded UTF-8 Markdown/plain-text imports with separate original and normalized artifacts.
- Explicit user-supplied URL imports over Rustls with redirect-by-redirect SSRF protection and HTML normalization.
- Text-PDF imports with page limits and typed encrypted, malformed, and `pdf_text_unavailable` results.
- CSV, JSON, and normalized host-agent discovery imports with dry-run and row-level diagnostics.
- Public RSS/Atom, jobs.ac.uk, Greenhouse, and Lever adapters over the same bounded SSRF-safe transport.
- Durable lead identity, freshness, refresh receipts/cursors, retained history, suggestions, and job promotion.
- Body-free compiled capabilities/context for Codex, Claude, and generic agent hosts.
- Leased tasks with exact job/artifact revisions, expiry, cancellation, stale detection, and idempotent completion.
- Bounded candidate JSON from regular files or stdin with schema-first and semantic validation.
- Explicit-consent export of only declared private inputs into an external task directory.
- Self-contained versioned Codex, Claude, and generic host packs with prompts, examples, schemas, and SHA-256
  manifests.
- A durable ten-stage workflow DAG with body-free blockers, next actions, scoped rerun, and stale propagation.
- Revisioned profile evidence normalization, correction, exclusion, confirmation, and exact source spans.
- Revision-bound criterion-to-evidence matching with strength, gaps, prohibited claims, and core-owned identities.
- User-confirmed apply/hold/skip decisions, strategy fields, four-document plans, and derived blocker gates.
- Sequential Cover Letter, CV, Research Statement, and Teaching Statement tasks in host-agent or configured-provider
  mode, with exact plan, criterion, evidence, and profile revision binding.
- Core-owned structured section, claim, citation, placeholder, generation, and document identities, plus automatic
  current `document-set` assembly and upstream stale propagation.
- Agent-callable `document list`, `document show`, and `document set` inspection with a bundled bounded drafting
  prompt for Codex, Claude, and generic hosts.
- Exact-set Review tasks with deterministic citation, placeholder, unclaimed-content, literal prohibited-claim, and
  repeated-claim consistency checks plus bounded semantic host findings.
- Core-owned deterministic/human finding authority and user-only `review export/confirm/show` dispositions with
  stable finding IDs, revision tracking, and automatic stale propagation.
- Deterministic `package check/show` with exact plan, evidence, profile, document-set, document, and review revision
  binding; machine-readable readiness reasons; idempotent manifests; and a fail-closed Render gate.
- Explicit package contracts that keep `ready-to-export` separate from submission and structurally forbid a readiness
  operation from recording an application as submitted.
- Consent-gated `package export` projection of each current structured document into editable Markdown, JSON, and
  self-contained Typst plus a package manifest, with an exact revision-bound export receipt and generated/observed
  SHA-256 hashes.
- Managed projection reconciliation with current, edited, missing, and repair-required states; implicit overwrites of
  user edits and unmanaged files are rejected, while `replace` and `copy-as-new` provide explicit recovery choices.
- Pinned in-process Typst compilation with embedded default fonts, no filesystem or package resolver, no default
  system-font scan, body-free diagnostics, and bounded source/PDF sizes behind the private `canisend-io` adapter.
- A packaged `doctor` self-check that compiles the embedded Cover Letter template to PDF, proving the optimized
  standalone binary retains the renderer without requiring a Typst executable or network access.
- One embedded application-document template shared by all four supported document kinds, with defensive Typst
  string escaping, unresolved-field rejection, exact source metadata, and the same edit-safe reconcile lifecycle.
- Idempotent `render build/show` over exact package/document revisions, with trusted Typst and validated PDF outputs
  stored as immutable blobs and frozen in a typed render manifest by one SQLite transaction.
- Structural PDF validation for bounded size, parseability, encryption state, page tree, and page count before commit
  and again before export, with body-free render diagnostics.
- Consent-gated `render export` of create-new PDFs plus the exact render manifest under `jobs/JOB_ID/`; edited `.typ`
  projections are never trusted compilation inputs, and rendering/exporting never submits an application.
- Exact provider redirect allowlists, portable path hardening, a published T01–T16 threat model, and pinned advisory,
  license, and dependency-source policy gates.
- Cross-platform recovery contracts for interruption boundaries, missing/corrupt blobs, projection reconstruction,
  stale tasks, and concurrent idempotent host-agent completion.
- Main/release performance gates for startup, large-workspace status, HTML/PDF intake, complete workflow execution,
  embedded rendering, and binary size, with a committed reproducible baseline.

The `canisend-gui` macOS development app uses Svelte over the same Rust application facade as the
CLI. Its primary journey combines Overview, Job & criteria, Evidence & fit, Materials, and Review
& export inside one selected Application Workspace with persistent Dossier context and resumable
receipt routes. It also provides opportunity discovery, reviewed file/URL/PDF intake, reusable
profile evidence, Agent handoff/runtime controls, a body-free Content Catalog with consent-gated
ephemeral full-text search, recovery, updates, and managed terminal CLI installation. See the
[desktop GUI preview guide](docs/guides/desktop-gui.md) for current coverage and limits.

The native matrices verify embedded fonts, edge-case Unicode/layout, missing-system-font
isolation, bundled licenses/notices, render timing, package budgets, lifecycle, and release
integrity. Current work and release gates are governed by the
[CanISend generic framework 1.0 delivery roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md); the
[plan registry](docs/superpowers/plans/README.md) separates active, supporting, completed, and
historical plans.

## Build the native foundation

Install the pinned Rust toolchain, then run:

```text
cargo build --release --locked
./target/release/canisend version --json
./target/release/canisend doctor --json
./target/release/canisend agent capabilities --json
./target/release/canisend agent context --json
./target/release/canisend schema list --json
./target/release/canisend resource list --json
./target/release/canisend --workspace ./my-workspace workspace init \
  --pack academic-job --json
./target/release/canisend --workspace ./my-workspace job create \
  --title "Lecturer in Economics" --institution "University X" --json
./target/release/canisend --workspace ./my-workspace job import JOB_ID \
  --file ./job-advert.pdf --json
./target/release/canisend --workspace ./my-workspace job import JOB_ID \
  --url https://example.edu/job-advert --json
./target/release/canisend --workspace ./my-workspace job show JOB_ID --json
./target/release/canisend --workspace ./my-workspace application list --json
./target/release/canisend --workspace ./my-workspace application show \
  --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace content list \
  --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace content search Economics \
  --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace content search "teaching portfolio" \
  --job JOB_ID --include-private-bodies --allow-private-read --json
./target/release/canisend --workspace ./my-workspace profile source add \
  --file ./profile-evidence.json --json
./target/release/canisend --workspace ./my-workspace profile source list --json
./target/release/canisend discovery adapters --json
./target/release/canisend discovery import --file ./leads.csv \
  --source-name "University export" --dry-run --json
./target/release/canisend --workspace ./my-workspace discovery import \
  --file ./leads.csv --source-name "University export" --json
./target/release/canisend --workspace ./my-workspace discovery refresh \
  --adapter greenhouse --endpoint \
  "https://boards-api.greenhouse.io/v1/boards/BOARD/jobs" \
  --source-name "University X" --json
./target/release/canisend --workspace ./my-workspace discovery list --json
./target/release/canisend --workspace ./my-workspace discovery promote LEAD_ID --json
./target/release/canisend --workspace ./my-workspace agent assets install \
  --host codex --json
./target/release/canisend --workspace ./my-workspace agent assets status \
  --host codex --json
./target/release/canisend --workspace ./my-workspace agent assets uninstall \
  --host codex --json
./target/release/canisend agent assets export --host codex \
  --destination ./codex-canisend-pack --json
./target/release/canisend --workspace ./my-workspace agent context --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace task prepare \
  --job JOB_ID --operation job-parse --json
./target/release/canisend --workspace ./my-workspace task inputs TASK_ID \
  --destination ./agent-work --allow-private-read --json
./target/release/canisend --workspace ./my-workspace task complete \
  --file ./agent-work/completion.json --json
./target/release/canisend --workspace ./my-workspace criteria export \
  --job JOB_ID --destination ./agent-work/criteria.json --json
./target/release/canisend --workspace ./my-workspace criteria confirm \
  --job JOB_ID --file ./agent-work/criteria.json --json
./target/release/canisend --workspace ./my-workspace profile evidence export \
  --job JOB_ID --destination ./agent-work/evidence.json --json
./target/release/canisend --workspace ./my-workspace match show --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace plan export \
  --job JOB_ID --destination ./agent-work/application-plan.json --json
./target/release/canisend --workspace ./my-workspace plan confirm \
  --job JOB_ID --file ./agent-work/application-plan.json --json
./target/release/canisend --workspace ./my-workspace package check --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace package show --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace package export --job JOB_ID \
  --destination jobs/JOB_ID/application --allow-private-export --json
./target/release/canisend --workspace ./my-workspace package exports --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace package reconcile --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace package replace --job JOB_ID \
  --path jobs/JOB_ID/application/cover-letter.md --json
./target/release/canisend --workspace ./my-workspace package copy-as-new --job JOB_ID \
  --path jobs/JOB_ID/application/cover-letter.md \
  --destination jobs/JOB_ID/application/cover-letter-edited.md --json
./target/release/canisend --workspace ./my-workspace render build --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace render show --job JOB_ID --json
./target/release/canisend --workspace ./my-workspace render export --job JOB_ID \
  --destination jobs/JOB_ID/rendered --allow-private-export --json
./target/release/canisend --workspace ./my-workspace workspace check --json
./target/release/canisend --workspace ./my-workspace workspace backup ./my-backup --json
```

Representative capability output distinguishes implemented and planned work. Agent hosts must not treat a planned
capability as executable.

## Development checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p xtask -- schemas write
cargo run -p xtask -- schemas check
cargo run -p xtask -- resources check
cargo build --release --locked
./scripts/smoke_host_agent.sh ./target/release/canisend /tmp/canisend-host-smoke
```

No Python interpreter, virtual environment, PyPI package, or Pytest runner participates in these checks.

## Target architecture

```text
Codex / Claude / user / custom host
                 │
       CLI / GUI / MCP surfaces
                 │
     Agent v3 │ Agent v2 compatibility
                 │
┌──────────────────────────────────────┐
│ canisend-app shared application API  │
├──────────────────────────────────────┤
│ domain-neutral kernel + Pack runtime │
├──────────────────┬───────────────────┤
│ SQLite/blob store│ bounded I/O       │
├──────────────────┴───────────────────┤
│ verified embedded Packs/resources    │
└──────────────────────────────────────┘
```

The accepted architecture uses SQLite plus immutable content-addressed blobs for authoritative
local state. User documents are exported projections. Rust types generate the v3 canonical and v2
compatibility contracts, every surface calls the same application API, and Typst compilation is
embedded in the standalone executable.

Accepted decisions are under `docs/architecture/rust-native/decisions/`.
The Academic compatibility machine interface is documented in
[Agent Protocol v2](docs/contracts/agent-protocol-v2.md); Generic Agent v3 capabilities and context
are discovered from the exact Pack-bound binary at runtime.

## Product boundary

CanISend prepares application materials. It does not submit applications, create accounts, fill portals, answer
sensitive declarations, or run an uncontrolled crawler.

Direct local files, user-supplied links, and text-based PDFs remain required product inputs. Image-only PDF OCR is
outside the first Rust release.

## Python-era archive

The Python source, tests, schemas, resources, and historical workflow documentation are preserved at:

```text
archive/python-v0.6.0b1-final
```

See [the archive record](docs/history/python-era.md). The Rust product does not import old workspaces or run the
archived implementation as a dependency.

## License

CanISend is free software licensed under
[GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). Corresponding source for each
release is the matching Git tag in this repository. Historical tags retain the license stated by
their own source tree.

Native bundles include [third-party renderer and font notices](THIRD_PARTY_NOTICES.md), the exact
upstream `typst-assets` license and notice files, and machine-readable release SBOM evidence.
