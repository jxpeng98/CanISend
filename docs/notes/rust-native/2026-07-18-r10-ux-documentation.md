# R10.4 UX and documentation closeout

**Date:** 2026-07-18

**Roadmap item:** R10.4

## User-facing contract

Human-mode success output now surfaces warnings and next actions that were previously visible only in JSON. Human
errors name the stable machine code, remediation, and retryability without printing private bodies. Missing
workspaces and scanned/image-only PDFs provide direct recovery guidance, while JSON envelopes remain unchanged for
agent hosts.

The required installation, quick-start, agent integration, privacy/consent, backup/recovery, and troubleshooting
guides are linked from the README and checked for missing files or broken local links by `xtask`. The guides are
explicit that image-only PDFs require external OCR plus user review, provider sends require scoped consent, and
CanISend never submits an application. R11 added a seventh release-verification guide without weakening the original
six-guide R10 contract.

## Staged-binary evidence

`scripts/smoke_documented_quickstart.sh` follows the documented path from a staged binary: version and offline
renderer diagnosis, help/remediation wording, host-pack export, workspace creation, local advert import, workflow
start, integrity check, verified backup, restore, projection repair, and final integrity check. It does not activate
Python or invoke an external Typst runtime.

GitHub Actions run `29630593937` passed every source, dependency, recovery, render, and performance gate but correctly
failed the Windows staged smoke before invoking the binary. `RUNNER_TEMP` arrived as `D:\\a\\_temp`; the script treated
it as a relative POSIX path and prefixed the checkout directory. A shared `canisend_absolute_path` helper now converts
Windows drive paths with `cygpath` while preserving normal POSIX and repository-relative paths.

The clean-checkout rerun `29631149914` passed the exact staged documentation workflow on macOS arm64, Windows x86_64,
and Linux x86_64, together with the complete Rust quality, dependency, recovery, native render, and performance
matrix. This closes R10 without suppressing or bypassing the platform-specific failure.

## Transition

R0–R10 are complete. R11.1 is active and owns five-target release archives, exact packaged-binary smokes, checksums,
CycloneDX SBOM, release manifest, GitHub provenance, known limitations, real-job dogfood, and explicit issue intake
without default telemetry.
