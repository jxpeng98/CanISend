# M1-DEP-001 — Dependency assurance and exception governance

Date: 2026-08-03

## Outcome

Dependency changes now trigger a dedicated read-only GitHub workflow that runs both the repository
exception validator and pinned `cargo deny` advisory, ban, license, and source checks. The same
exception validator is part of `release check`, and the candidate source job still independently
runs `cargo deny`.

The machine authority
[`release/dependency-advisory-exceptions.json`](../../../release/dependency-advisory-exceptions.json)
binds all 23 `deny.toml` exceptions to 751 current third-party lock entries. Each entry records its
classification, owner, exact product reachability, review and expiry dates, removal condition, and
an upstream tracking reference. Workspace-only version transitions are excluded from the
fingerprint; any third-party package, version, source, checksum, or dependency change invalidates
the review.

## Corrected vulnerability classification

The earlier Alpha.4 review covered only informational unmaintained advisories. The current list now
also contains two vulnerabilities:

- [`RUSTSEC-2026-0194`](https://rustsec.org/advisories/RUSTSEC-2026-0194.html), a quadratic-time
  `quick-xml` attribute check; and
- [`RUSTSEC-2026-0195`](https://rustsec.org/advisories/RUSTSEC-2026-0195.html), an unbounded
  namespace-declaration allocation in `NsReader`.

Both affect `quick-xml 0.38.4`; RustSec identifies `0.41.0` as patched. The older version remains
only through `citationberg → hayagriva → typst-library`. CanISend accepts no bibliography, CSL, or
XML render input, and its fixed projector does not invoke the embedded CV template's
declaration-only `publication` helper. The source gate now rejects either an IO-projector call or a
second template call site, so that reachability claim cannot silently change.

This is a bounded exception, not remediation. The removal condition is to upgrade the Typst
citation chain to `quick-xml 0.41` or later. Any bibliography/CSL/XML exposure, source-gate drift,
missed review, or expiry blocks the Alpha candidate.

## Review window and workflow

Every current exception was reviewed on 2026-08-03, must be reviewed again by 2026-08-10, and
expires on 2026-08-17. The validator permits at most a 14-day review interval and a 30-day hard
exception lifetime. IDs must be unique and sorted; the policy and `deny.toml` sets and reachability
text must match exactly.

`.github/workflows/dependency-assurance.yml` runs on changes to Cargo manifests, `Cargo.lock`,
`deny.toml`, the exception authority, the validator, the guarded renderer/template boundary, or the
workflow itself. It has only `contents: read`, uses Rust 1.97.0 and immutable action pins, and cannot
build a release profile, publish, or replace packaged-binary qualification.

## Verification

- `cargo run -p xtask --locked -- dependencies fingerprint` — 751 packages,
  `d2807a35172dc853ad98f7e128f1cbc4737b61aac8cb31f4ddf56c18b05ed903`
- `cargo run -p xtask --locked -- dependencies check` — 23 exceptions, 2 vulnerabilities
- `cargo deny check advisories bans licenses sources` — all four classes passed
- focused stale-review and workspace-version-only fingerprint regressions
- full `cargo test -p xtask --locked`
- `cargo run -p xtask --locked -- release check`

## Remaining boundary

M1-DEP-001 is implemented and locally verified in source. The M1A checkbox remains open until the
committed dependency workflow and fast CI pass remotely and the evidence is linked and reviewed.
No Alpha.6 write, tag, candidate, push, or publication occurred in this work.
