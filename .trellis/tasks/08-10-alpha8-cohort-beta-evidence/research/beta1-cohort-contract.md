# Research: Beta.1 cohort contract

## Repository findings

- Issue #70 is `state:ready` and requires exact public Beta.1, Codex-primary validation, at least
  8 cumulative users, at least 20 supported flows, both Packs, mixed-Application coverage,
  body-free metrics, exclusions, blocker dispositions, and consent before participant contact.
- The authoritative Roadmap places `M3-EVID-005` after public Beta.1 and before RC.1. Its thresholds
  are release evidence rather than telemetry.
- Public Beta.1 source is `6e1397b79031cad54e794ccdc9edca2153f23b3e`; candidate run is
  `33281162734`; public-verification run is `33283530240`.
- Feature freeze is active at protected baseline
  `acf25dc483643ca9be0210320775708da116b715` with zero initial exceptions.
- `release/beta-readiness.json` and its xtask validator intentionally require `cohort_evidence` to
  remain `not-started` with zero users and zero flows. Reusing or rewriting that record would erase
  the Beta-entry boundary.
- No active post-Beta cohort JSON or validator exists. The stale Trellis task still named Alpha.9
  and incorrectly proposed refreshing Beta readiness.

## Minimal decision

Retain the stable Trellis task directory to avoid link churn. Plan one separate aggregate cohort
record plus one reviewed note only when consented execution begins. Do not add participant
management, a new tracker, compatibility code, or a second test matrix.

## Consent boundary

This research authorizes no invitation, provider send, temporary host configuration, participant
roster, or private-data access. The validation owner must supply a bounded schedule and confirm
consent before Phase B begins.
