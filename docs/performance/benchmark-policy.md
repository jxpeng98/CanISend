# Performance benchmark policy

**Baseline format:** `canisend.performance-baseline/v1`

**Initial baseline:** [baseline-v1.json](baseline-v1.json)

CanISend performance gates measure the optimized native product, not Cargo compilation. A cold release build is
allowed to populate the shared Cargo cache before measurement. The benchmark contract is ignored by normal `cargo
test` runs and is activated explicitly by main and release workflows.

## Measured paths

| Metric | Fixture and method | Threshold |
| --- | --- | ---: |
| `version_startup_median_ms` | Seven warm release-binary process launches | 100 ms |
| `capabilities_startup_median_ms` | Seven warm `agent capabilities --json` launches | 150 ms |
| `status_100_jobs_median_ms` | Five launches against a workspace containing at least 100 jobs | 500 ms |
| `html_1_mib_intake_median_ms` | Three 1 MiB HTML normalize-and-commit operations, excluding network | 2,000 ms |
| `pdf_50_page_intake_median_ms` | Three release CLI imports of a generated 50-page text PDF | 5,000 ms |
| `typst_render_median_ms` | Median of three embedded `doctor` render probes | 1,000 ms |
| `release_binary_bytes` | Stripped/LTO release CLI executable | 67,108,864 bytes |
| `full_synthetic_workflow_ms` | Exact service workflow from intake through four documents, review, package, PDF render, export, and invalidation | 15,000 ms |

Durations are rounded up to whole milliseconds. Medians reduce scheduler noise without hiding persistent
regressions. The HTML test calls the same public parser used after safe HTTP transport and commits the resulting
source to SQLite/blob authority; network and DNS time are deliberately excluded. The PDF test includes local file
validation, parsing, 50-page text extraction, canonical storage, and job revision mutation.

The synthetic workflow gate measures only test execution. Cargo test discovery, compilation, and link time are not
included. It covers the complete revision-bound material path and therefore catches performance regressions in
SQLite transactions, candidate validation, projections, Typst compilation, PDF validation, and invalidation.

## Running the gates

```console
cargo test --release -p canisend-cli --locked \
  --test performance_contract -- --ignored --nocapture

CANISEND_PERFORMANCE_GATE=1 cargo test --release -p canisend-store --locked \
  --test store_contract \
  evidence_and_match_tasks_enforce_stable_revision_bound_identities \
  -- --exact --nocapture
```

Set `CANISEND_PERFORMANCE_OUTPUT` to write the release-binary metrics as JSON. CI stages that file as
`PERFORMANCE.json` inside the Linux native evidence bundle.

## Threshold changes

A threshold may change only with all of the following:

1. a recorded before/after baseline on the same target and release profile;
2. an explanation of whether the change is product work, dependency/toolchain movement, or CI-host variance;
3. confirmation that URL, path, validation, integrity, privacy, and render controls remain enabled;
4. an updated baseline document and roadmap/note entry in the same commit.

Thresholds are intentionally looser than the initial reference measurements. They are regression tripwires, not a
reason to remove security checks or weaken artifact verification.
