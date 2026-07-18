# R10 scheduled fuzzing follow-up

**Date:** 2026-07-18

**Roadmap item:** Definition of Done / release evidence

**Status:** Harness compiled and smoke-fuzzed locally; first full GitHub run pending

## Boundary

Normal Rust quality gates remain fast and stable-only. A separate weekly/manual workflow uses pinned nightly
libFuzzer tooling for three production boundaries: public structured JSON, intake/discovery/HTML/PDF validation, and
PDF text extraction. Inputs are capped before parsing, each target has a five-minute campaign, and the job has a
hard time/memory boundary.

The workflow uploads a generated reproducer only on failure. It never seeds from a real workspace or private job
application. Reproducible crashes remain release blockers and must become ordinary regression tests before the note
can record resolution.

## Remaining evidence

The pinned nightly toolchain and cargo-fuzz 0.13.2 compiled all three harnesses locally. Each target completed a
100-run AddressSanitizer/libFuzzer smoke campaign without a crash, panic, timeout, or reproducer. Generated corpus,
artifact, and build directories are ignored and contain no project/user data.

After release-gate implementation completed, all three targets also ran concurrent 60-second campaigns with the
workflow's pinned `nightly-2026-07-01`, 15-second per-input timeout, and 4096 MiB RSS limit. No target produced a
crash, timeout, or artifact. This is stronger local prequalification, but it is not the required scheduled run.

Dispatch the full five-minute-per-target workflow and require all three jobs to finish successfully. The Definition
of Done fuzz checkbox remains open until that exact GitHub Actions run is recorded. GitHub does not allow a newly
introduced workflow to be manually dispatched until the workflow path exists on the default branch; the first full
run therefore belongs to the reviewed Rust-native main/RC cutover, not an unevidenced branch-only claim.
An attempted branch dispatch returned GitHub HTTP 404 for this exact default-branch registration rule.
