# R10 scheduled fuzzing follow-up

**Date:** 2026-07-18

**Roadmap item:** Definition of Done / release evidence

**Status:** Complete — first full default-branch GitHub campaign passed

## Boundary

Normal Rust quality gates remain fast and stable-only. A separate weekly/manual workflow uses pinned nightly
libFuzzer tooling for three production boundaries: public structured JSON, intake/discovery/HTML/PDF validation, and
PDF text extraction. Inputs are capped before parsing, each target has a five-minute campaign, and the job has a
hard time/memory boundary.

The workflow uploads a generated reproducer only on failure. It never seeds from a real workspace or private job
application. Reproducible crashes remain release blockers and must become ordinary regression tests before the note
can record resolution.

## Completed evidence

The pinned nightly toolchain and cargo-fuzz 0.13.2 compiled all three harnesses locally. Each target completed a
100-run AddressSanitizer/libFuzzer smoke campaign without a crash, panic, timeout, or reproducer. Generated corpus,
artifact, and build directories are ignored and contain no project/user data.

After release-gate implementation completed, all three targets also ran concurrent 60-second campaigns with the
workflow's pinned `nightly-2026-07-01`, 15-second per-input timeout, and 4096 MiB RSS limit. No target produced a
crash, timeout, or artifact. This is stronger local prequalification, but it is not the required scheduled run.

After the reviewed Rust-native cutover registered the workflow on `main`, GitHub Actions run `29684660492` executed
the full five-minute campaign for `structured_inputs`, `intake_parsers`, and `pdf_extract` at exact source commit
`520caee847215d864094aba7378d842f1b5a3990`. All three jobs completed successfully, and the failure-only reproducer
step did not upload an artifact. The cold AddressSanitizer builds plus bounded campaigns completed within the
separate 20-minute extended-assurance limit without making the ordinary edit/test loop slower.

The earlier branch dispatch returned GitHub HTTP 404 because GitHub requires a manually dispatched workflow to
exist on the default branch. That historical failure and the later successful default-branch run distinguish
workflow registration from fuzz-target behavior; only the successful run closes the Definition of Done item.
