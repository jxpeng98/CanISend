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
set was re-reviewed on 2026-09-19 after a fresh advisory scan found `RUSTSEC-2026-0285` in
`rustls 0.23.42`. The compatible lock update moves rustls to 0.23.45, its crypto and WebPKI
dependencies to patched versions, and the yanked `chacha20 0.10.1` to 0.10.2. The graph still has
751 third-party packages, and no renderer, font, bibliography, GTK or pattern input changed. That
`cargo-deny 0.19.7` advisory, ban, license and source check passed.

The same 23 exceptions were re-reviewed on 2026-10-04 after CI rejected the overdue 2026-10-03
review. A fresh `cargo-deny 0.19.7` scan against RustSec database commit
`ef6173cbc5c50ec8166f9a5b28f07834144373ee` passed advisories, bans, licenses and sources;
existing duplicate and unmatched-license warnings remain. The lock fingerprint remains
`90de0659277c7aa1f940e059931b95403502e44800218c6e72038c2243135ea9` for 751 third-party
packages. The structured projection and updated templates still pass text as literals, use
embedded fonts and leave the bibliography helper declaration-only. The reverse dependency
review also checked `lopdf 0.42.0`: its `ttf-parser` call is in `FontData::new`, an unused PDF
creation API, not CanISend's PDF load/text-extraction path. GTK and pattern input boundaries
remain unchanged. No exception was added and no input or platform scope was expanded.

The next review is due 2026-10-18; the existing hard expiry remains 2026-10-19. A missing, new,
reordered, stale, expired, or lock-mismatched exception fails before `cargo deny` can treat it as
accepted. This development review does not renew native or real-Host release qualification.

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
