# R11.2 Beta readiness baseline

**Date:** 2026-07-18

**Roadmap item:** R11.2

## Alpha blocker audit

The first post-publication audit queried all GitHub issues after `v0.7.0-alpha.1` was public and independently
verified. At `2026-07-18T06:43:48Z`, the repository contained zero issues in total and therefore zero open reports in
the four release-blocker classes: data loss/corruption, security/privacy boundary, protocol/workspace compatibility,
and rendering failure/corruption.

An empty issue list is not treated as proof by itself. `release/beta-readiness.json` binds the audit to the exact
Alpha tag, source commit, release run, and public URL, and records concrete CI/dogfood evidence for every blocker
class. `xtask release check` rejects an unknown/missing class, an open issue number, a non-clear status, an empty
evidence set, a changed Alpha identity, enabled default telemetry, or any unresolved release blocker.

## Known limitations

The published Alpha limitations remain accepted scope boundaries rather than silent defects: no OCR for image-only
PDFs, no GUI or application submission, bounded public-source adapters, unsigned Alpha archives, and no Python-era
workspace migration. None contradicts the documented Alpha contract. Signing/notarization remains an explicit Beta
delivery gate and is not classified as an already resolved Alpha defect.

## Re-audit rule

The ledger is an auditable snapshot, not live telemetry. It must be refreshed immediately before the Beta tag. Any
new report in one of the four blocker classes changes the corresponding status and open-issue list, which makes the
release check fail until the issue is resolved and linked to verification evidence. Feedback continues to arrive
only through explicit, sanitized GitHub issues.

## Transition

The Alpha blocker-resolution item is currently clear and machine-gated. R11.2 proceeds to the agent protocol v2 and
workspace v2 migration freeze, followed by package-manager candidates and credential-backed signing/notarization.
