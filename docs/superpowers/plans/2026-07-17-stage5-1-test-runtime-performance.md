# Stage 5.1 Test And Runtime Performance Implementation Plan

**Status:** In progress — Tasks 1–6 implemented; final release acceptance remains

**Date:** 2026-07-17

**Branch:** `perf/test-runtime-architecture`

**Baseline:** Stage 5 completion commit `8ede9725cdf7c02f53dde26298ffbd04092ce19f`

## Goal

Reduce ordinary developer feedback from tens of minutes to seconds without weakening release evidence, narrow the
declared Python floor to 3.12, and remove the repeated workflow-graph and schema-validation work responsible for the
Stage 5 runtime regression.

## Baseline Evidence

- Stage 5 CI run `29557401479` passed 1,322 tests in 4,183.06 seconds on Python 3.11, 5,297.41 seconds on Python
  3.12, and 2,688.83 seconds on Python 3.13. The dependent build and installed-wheel smoke added 12 minutes.
- Stage 4 had 1,248 tests and completed on Python 3.13 in 832.67 seconds. Test count grew by about 6%, while the
  Stage 5 Python 3.13 duration grew by more than three times.
- The selected high-signal contract gate passed 170 tests in 17.03 seconds on the development interpreter.
- Profiling one Package integration test found 4,796 `inspect_stage_status` calls, 3,134 JSON Schema
  `check_schema` calls, 652 Draft candidate validations, and about 722 million Python calls. Disabling `fsync` did
  not improve its 110-second unprofiled runtime.
- The final Python 3.12 fast gate (including Stage Runtime and prebuilt-fixture contracts) passes 223 tests in 15.18
  seconds on a warm run. The profiled Package scenario fell from about 110 seconds to
  59.65 seconds after operation-scoped status reuse, then to 39.42
  seconds after content-addressed schema compilation reuse on the development interpreter; the final Python 3.12
  measurement is 51.49 seconds.
- GitHub Actions run `29581321336` passed the cold Python 3.12 fast job in 39 seconds. The full and package jobs were
  skipped on the non-main performance branch as designed.
- The repeated Draft/Review and dual-document Package prefixes now use byte-verified process-local templates. Every
  test receives an independent copy, and template SHA-256 manifests are checked before and after every clone.
- A representative 24-test slow slice fell from 168.02 seconds sequentially to 96.05 seconds with two workers. A
  165-test concurrency, CAS, recovery, migration, and mutation audit passed with two workers in 133.48 seconds.
- The complete Python 3.12 suite passed 1,336 tests with two workers in 588.91 seconds (9 minutes 48 seconds). The
  slowest remaining individual end-to-end test took 89.12 seconds.
- The updated source and wheel distributions passed Twine and packaged-resource checks. A clean Python 3.12.12
  wheel install passed the 13-stage Decision Spine smoke with 20 mutation receipts and the complete offline Stage 4
  discovery smoke.

## Fixed Decisions

- `requires-python` becomes `>=3.12`; metadata, documentation, and lock state must agree.
- Python 3.12 is the single mandatory lower-bound interpreter. Ordinary CI will not run a 3.11–3.13 matrix.
- Pull requests and non-main pushes run a maintained `fast` pytest gate on Ubuntu and Python 3.12.
- Full tests plus source package checks run on main, manual dispatch, and release tags only.
- macOS/Ubuntu/Windows workflow smokes and installed-wheel smokes are release gates, not edit-loop gates.
- Release tags still run the complete suite before publishing to TestPyPI or PyPI.
- Runtime caches are scoped to one read-only status inspection. They never survive a filesystem mutation, stage
  prepare/apply/cancel boundary, user mutation, repair, migration, or separate CLI request.
- A Rust rewrite is outside this plan. Performance work must first make the existing dependency traversal linear in
  the number of stage/document nodes.

## Delivery Sequence

### Task 0: Contract And Measurement Freeze

- [x] Record the Stage 5 interpreter, smoke, and package timings.
- [x] Benchmark one high-signal 170-test fast gate.
- [x] Profile a representative slow Package integration test and identify repeated status/schema validation.
- [x] Freeze the new Python support and CI lane decisions.

### Task 1: Python 3.12 Floor

- [x] Raise project and lock metadata from Python 3.11 to Python 3.12.
- [x] Remove the Python 3.11 classifier and update the README badge and installation contract.
- [x] Add a repository contract test so metadata and documentation cannot drift.

### Task 2: Maintained Fast Gate

- [x] Register strict `fast`, `integration`, `slow`, and `release` pytest markers.
- [x] Assign the accepted high-signal modules to `fast` from one explicit manifest.
- [x] Add tests that reject an empty, missing, duplicated, or unknown fast-gate module.
- [x] Document `uv run python -m pytest -q -m fast` as the default edit-loop command.

### Task 3: CI And Release Lane Split

- [x] Run only the Python 3.12 fast gate for pull requests and non-main pushes.
- [x] Run the complete Python 3.12 suite and source package checks on main or manual dispatch.
- [x] Move cross-platform Stage 5/Stage 4 smokes from ordinary CI into the tag-driven release gate.
- [x] Preserve full-suite, build, Twine, resource, clean-wheel, TestPyPI, and publication ordering for releases.

### Task 4: Linear Stage Inspection

- [x] Add one operation-scoped inspection context keyed by workspace, job, stage, and document.
- [x] Reuse a completed status inspection inside the same read-only dependency traversal.
- [x] Detect accidental recursive cycles and never cache an error or partial inspection.
- [x] Prove separate calls and calls after mutation observe current filesystem state.

### Task 5: Compiled Schema Reuse

- [x] Cache parsed, checked Draft 2020-12 validators by exact schema content.
- [x] Replace repeated `check_schema` calls in stage candidate validation without weakening candidate checks.
- [x] Prove local schema overrides invalidate by content and invalid schemas still fail closed.

### Task 6: Integration Fixture And Parallelism Audit

- [x] Create immutable prebuilt workflow fixtures for the repeated Draft/Review and dual-document/package prefixes.
- [x] Clone fixtures per test so no mutable or private test state is shared.
- [x] Trial two pytest workers and retain parallel execution only if adversarial/concurrency tests remain stable.

### Task 7: Exit Acceptance

- [x] Keep the fast gate below 30 seconds locally (15.18 seconds for 223 tests on Python 3.12).
- [x] Confirm the fast gate stays below two minutes in a cold GitHub Actions run (39 seconds, run `29581321336`).
- [x] Reduce the full single-interpreter suite to a measured target below 15 minutes (9 minutes 48 seconds).
- [x] Pass focused cache, runtime, mutation, projection, migration, and release-contract suites.
- [x] Pass one complete Python 3.12 suite plus local build, Twine, resources, and clean-wheel smokes.
- [ ] Confirm the Ubuntu/macOS/Windows release smokes on a release-candidate tag.
- [ ] Record immutable evidence before marking Stage 5.1 complete.

## Safety And Compatibility Constraints

- Cached status is an in-memory read optimization only; authoritative state and receipt files remain unchanged.
- No cache can turn blocked, stale, failed, drifted, cancelled, or repair-required work into current work.
- User-owned YAML, Typst primaries, private candidates, and exact compare-and-swap behavior remain unchanged.
- Direct URL/PDF/text advert intake, Stage 4 discovery imports, host-agent execution, and legacy projections remain
  covered by full/release gates even when they are not in the ordinary fast lane.
- Historical plans remain historical evidence and are not rewritten to claim the new Python floor retroactively.
