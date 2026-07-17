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

The Rust rebuild has implemented R6, including durable direct intake, discovery, and bounded agent collaboration.
Clean-checkout acceptance is pending before R7 becomes active. The current binary provides:

- Native `canisend` executable scaffolding.
- Validated UUIDv7, SHA-256, revision, UTC timestamp, and safe relative-path contract types.
- `canisend.agent/v2` success/error envelopes, stable error registry, and grouped exit policy.
- Product/version/build inspection.
- Twenty deterministic Draft 2020-12 schemas generated from Rust types.
- Twenty-six typed embedded schemas, prompts, templates, examples, and host assets with SHA-256 verification.
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

Application workflow, evidence-backed drafting, and embedded PDF rendering are not yet available in the production
binary. Their execution order and acceptance gates are defined in the
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
  --job JOB_ID --operation job-criterion --json
./target/release/canisend --workspace ./my-workspace task inputs TASK_ID \
  --destination ./agent-work --allow-private-read --json
./target/release/canisend --workspace ./my-workspace task complete \
  --file ./agent-work/completion.json --json
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
