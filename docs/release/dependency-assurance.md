# Dependency assurance

CanISend checks dependency advisories, licenses, duplicate/banned packages, and sources whenever a
Cargo manifest, lockfile, `deny.toml`, its exception authority, the validator, or the guarded
renderer/template boundary changes. The dedicated `dependency-assurance` workflow is read-only
development evidence. Candidate source qualification reruns `cargo deny`; neither result replaces
exact packaged-binary qualification.

## Local checks

Run both checks after a dependency or policy change:

```console
cargo run -p xtask --locked -- dependencies check
cargo deny check advisories bans licenses sources
```

The first command verifies that
[`release/dependency-advisory-exceptions.json`](../../release/dependency-advisory-exceptions.json)
matches every `deny.toml` exception and the current third-party portion of `Cargo.lock`. Workspace
package version-only transitions do not change that fingerprint. Adding, removing, or changing a
third-party package does, which forces a fresh review.

Every exception records:

- its exact RustSec ID and classification;
- a named owner and product-specific reachability statement identical to `deny.toml`;
- `reviewed_on`, `review_by`, and `expires_on` dates;
- a concrete removal condition; and
- an HTTPS upstream issue, commit, or advisory-tracking reference.

Reviews are valid for at most 14 days and exceptions for at most 30 days. The current lock-bound
set was re-reviewed on 2026-08-24 against the unchanged 751-package fingerprint and a fresh exact
`cargo-deny 0.19.5` advisory, ban, license, and source check. Its next review and hard expiry are
both 2026-09-07, so there is no grace period after a missed review. A missing, new, reordered,
stale, expired, or lock-mismatched exception fails before `cargo deny` can treat it as accepted.

## Vulnerability boundary

`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` concern `quick-xml 0.38.4`, which remains in Typst's
bibliography dependency chain even though another root dependency already uses patched
`quick-xml 0.41.0`. CanISend's renderer accepts structured, bounded document data and uses fixed
Typst projections; it accepts no bibliography, CSL, or XML render input and never invokes the
embedded template's declaration-only bibliography helper.

`dependencies check` enforces that boundary in source. Adding a bibliography/helper invocation,
changing the helper from declaration-only, or exposing the affected parser fails the source gate.
The permanent removal condition is to move the Typst citation chain to `quick-xml 0.41` or later.
Until then, either exception expiring or its input becoming reachable blocks the Alpha candidate.

Unmaintained GTK3 crates remain compile-graph-only for Tauri's nonpublished Linux GUI backend;
Linux public artifacts are CLI-only. The rust-unic path receives only checked-in Tauri patterns,
and the font/parser paths receive only embedded release-verified assets. Expanding those inputs or
public platforms invalidates the relevant exception immediately.
