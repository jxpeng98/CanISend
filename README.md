<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/ci.yml?branch=rewrite%2Frust-native&label=Rust%20CI" alt="Rust CI status"></a>
  <img src="https://img.shields.io/badge/Rust-1.92%2B-orange" alt="Rust 1.92+">
  <img src="https://img.shields.io/badge/protocol-canisend.agent%2Fv2-blue" alt="Agent protocol v2">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license">
</p>

# 这也能投 / CanISend

CanISend is being rebuilt as a standalone Rust-native CLI for evidence-backed academic and professional application
preparation.

The active product no longer uses Python or Pytest. The final Python implementation remains available only through
the Git tag `archive/python-v0.6.0b1-final`.

## Current status

The Rust rebuild has completed R8 plus R9.1–R9.2, including the full evidence-backed material pipeline, a restricted
in-process Typst compiler, and safe editable Typst projections. R9.3 PDF artifact output is now active. The current
binary provides:

- Native `canisend` executable scaffolding.
- Validated UUIDv7, SHA-256, revision, UTC timestamp, and safe relative-path contract types.
- `canisend.agent/v2` success/error envelopes, stable error registry, and grouped exit policy.
- Product/version/build inspection.
- Thirty-eight deterministic Draft 2020-12 schemas generated from Rust types.
- Forty-nine typed embedded schemas, prompts, templates, examples, and host assets with SHA-256 verification.
- A truthful capability registry that marks unfinished functions as `planned`.
- Agent context plus schema/resource diagnostics with deterministic JSON snapshots.
- Workspace discovery, explicit `--workspace` resolution, initialization, status, integrity checks, and repair.
- Bundled SQLite authority with immutable SHA-256 blobs, revisions, dependency invalidation, and audit events.
- Verified workspace backup and restore with referenced-blob manifests.
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

PDF artifact persistence and package PDF export are not yet available through the production workflow. Their
execution order and acceptance gates are defined in the
[Rust-native roadmap](docs/superpowers/plans/2026-07-17-rust-native-greenfield-roadmap.md).

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
./target/release/canisend --workspace ./my-workspace workspace init --json
./target/release/canisend --workspace ./my-workspace job create \
  --title "Lecturer in Economics" --institution "University X" --json
./target/release/canisend --workspace ./my-workspace job import JOB_ID \
  --file ./job-advert.pdf --json
./target/release/canisend --workspace ./my-workspace job import JOB_ID \
  --url https://example.edu/job-advert --json
./target/release/canisend --workspace ./my-workspace job show JOB_ID --json
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
                 ▼
       canisend.agent/v2 JSON
                 │
                 ▼
┌──────────────────────────────────────┐
│ canisend-cli                         │
├──────────────────────────────────────┤
│ canisend-core     canisend-contracts │
├──────────────────┬───────────────────┤
│ canisend-store   │ canisend-io       │
├──────────────────┴───────────────────┤
│ canisend-resources                   │
└──────────────────────────────────────┘
```

The accepted architecture uses SQLite plus immutable content-addressed blobs for authoritative local state. User
documents are exported projections. Rust types generate v2 schemas, agents complete bounded tasks through the CLI,
and Typst will be embedded into the final executable.

Accepted decisions are under `docs/architecture/rust-native/decisions/`.
The machine interface is documented in [Agent Protocol v2](docs/contracts/agent-protocol-v2.md).

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

MIT. Embedded third-party resources and fonts will be listed in native release notices before distribution.
