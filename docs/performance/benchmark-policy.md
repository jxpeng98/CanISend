# Performance benchmark policy

**Baseline format:** `canisend.performance-baseline/v1`

**Initial baseline:** [baseline-v1.json](baseline-v1.json)

**macOS GUI Alpha baseline:** [macos-gui-alpha-baseline.json](macos-gui-alpha-baseline.json)

**Desktop payload baseline:**
[desktop-size-aarch64-apple-darwin.json](desktop-size-aarch64-apple-darwin.json)

CanISend performance gates measure the optimized native product, not Cargo compilation. A cold release build is
allowed to populate the shared Cargo cache before measurement. The benchmark contract is ignored by normal `cargo
test` runs and is activated explicitly by main and release workflows.

Alpha candidate gates run with the named `release-alpha` profile (`lto = false`,
`codegen-units = 16`). Beta, RC, and Stable retain the canonical `release` profile
(`lto = "thin"`, `codegen-units = 1`). Before/after measurements must name the profile so results
from these two optimization policies are not silently mixed.

## Measured paths

| Metric | Fixture and method | Threshold |
| --- | --- | ---: |
| `version_startup_median_ms` | Seven warm release-binary process launches | 100 ms |
| `host_status_startup_median_ms` | Seven warm clean-v4 `host status --host codex --json` launches | 150 ms |
| `status_100_applications_median_ms` | Five launches against one Workspace v4 containing 100 Pack-bound Applications | 500 ms |
| `html_1_mib_intake_median_ms` | Three 1 MiB HTML normalization and bounded-file-write operations, excluding network | 2,000 ms |
| `pdf_50_page_intake_median_ms` | Three consented v4 local-file preview/commit operations for a generated 50-page text PDF | 5,000 ms |
| `typst_render_median_ms` | Median of three embedded `doctor` render probes | 1,000 ms |
| `release_binary_bytes` | Stripped/LTO release CLI executable | 67,108,864 bytes |
| `full_synthetic_workflow_ms` | Exact service workflow from intake through four documents, review, package, PDF render, export, and invalidation | 15,000 ms |
| `macos_gui_maximum_startup_ms` | Five isolated-user launches of the exact signed App until the native AccessKit Overview control exists | 2,000 ms |
| `macos_unified_host_bytes` | Ad-hoc-signed GUI/CLI/MCP host inside the App | 67,108,864 bytes |
| `macos_application_payload_bytes` | App payload containing exactly one full native host | 75,497,472 bytes |
| `dossier_list_max_ms` | Five in-process reads of 128 application Dossiers after warm-up | 2,000 ms |
| `indexed_search_max_ms` | Five deterministic metadata-index rebuilds over at least 256 Catalog entries | 1,000 ms |

Durations are rounded up to whole milliseconds. Medians reduce scheduler noise without hiding
persistent regressions. The HTML test calls the same public parser used after safe HTTP transport
and writes its normalized bounded result; network and DNS time are deliberately excluded. The PDF
test includes private-read consent, local-file validation, parsing, 50-page text extraction,
Pack-qualified Requirement proposal, canonical Source/Application storage, and revision mutation.

The initial baseline retains its historical `capabilities_startup_median_ms` and
`status_100_jobs_median_ms` field names. Current Alpha.7 measurements use the clean-v4 names above;
they are not silently compared as the same semantic fixture.

The synthetic workflow gate measures only test execution. Cargo test discovery, compilation, and link time are not
included. It covers the complete revision-bound material path and therefore catches performance regressions in
SQLite transactions, candidate validation, projections, Typst compilation, PDF validation, and invalidation.

The Dossier/Catalog baseline is a scheduled source-level tripwire rather than a release-binary
benchmark. Fixture construction and compilation are excluded. Search remains metadata-only for
this baseline, so it measures deterministic Catalog projection and index rebuilding without
reading private bodies or requiring consent.

## Running the gates

```console
cargo test --profile release-alpha -p canisend-cli --locked \
  --test performance_contract -- --ignored --nocapture

CANISEND_PERFORMANCE_GATE=1 cargo test --profile release-alpha -p canisend-store --locked \
  --test store_contract \
  evidence_and_match_tasks_enforce_stable_revision_bound_identities \
  -- --exact --nocapture

cargo test -p canisend-app --locked \
  --test read_model_performance -- --ignored --nocapture
```

Set `CANISEND_PERFORMANCE_OUTPUT` to write the release-binary metrics as JSON. CI stages that file as
`PERFORMANCE.json` inside the Linux native evidence bundle.

The macOS GUI gate runs separately against a staged, verified, ad-hoc-signed App:

```console
./scripts/measure_macos_gui_startup.sh \
  /path/to/CanISend.app \
  /path/to/macos-gui-performance.json \
  release-alpha
```

An isolated size-profile experiment whose workspace version belongs to another release stage may
append `--nonpublishing-profile-candidate`. That explicit mode records
`authoritative_release_evidence: false`; it cannot replace the stage-matched release measurement.

Each launch gets a new disposable `HOME`. The timer stops only after macOS exposes the Overview navigation control,
so the result includes process startup, system CJK font loading, window creation, and first usable UI state. The
measurement script also binds the signed unified-host hash and App payload size and rejects either the startup or size budget.
The [macOS App size strategy](macos-app-size-strategy.md) preserves the historical two-executable
baseline and records the measured single-host cutover.

Standard desktop packages also use the cross-platform payload recorder:

```console
cargo run -p xtask --locked -- desktop size-record \
  TARGET PROFILE FORMAT HOST PAYLOAD FRONTEND_OR_DASH ARTIFACT_OR_DASH OUTPUT.json
```

The standard-package recorder rejects symlinks; the portable-package recorder accepts only
symlinks that resolve inside the extracted payload. Both require the declared PE/ELF/Mach-O host
inside the measured payload, fail if another CanISend host is present, and record frontend and
artifact bytes separately. Windows offline installers and Linux AppImage containers require their
own runtime-inclusive records; they are not compared with the standard 72 MiB application-payload
budget. Runtime-inclusive extracted payloads are capped at 384 MiB, AppImage artifacts at 128 MiB,
and offline Windows installers at 256 MiB. Portable payload symlinks must resolve inside the
extracted root, and the record must still identify exactly one named CanISend host.

The scheduled desktop-platform qualification workflow currently evaluates a release candidate with
`CARGO_PROFILE_RELEASE_OPT_LEVEL=z`. Size records expose this as
`build_optimization.rust_opt_level`; records without an override use `profile-default`. The
candidate does not change the production release profile until native Windows/Linux qualification
and the exact macOS GUI startup gate pass. See the
[further size reduction plan](unified-host-further-size-reduction-plan.md).

## Threshold changes

A threshold may change only with all of the following:

1. a recorded before/after baseline on the same target and release profile;
2. an explanation of whether the change is product work, dependency/toolchain movement, or CI-host variance;
3. confirmation that URL, path, validation, integrity, privacy, and render controls remain enabled;
4. an updated baseline document and roadmap/note entry in the same commit.

Thresholds are intentionally looser than the initial reference measurements. They are regression tripwires, not a
reason to remove security checks or weaken artifact verification.
