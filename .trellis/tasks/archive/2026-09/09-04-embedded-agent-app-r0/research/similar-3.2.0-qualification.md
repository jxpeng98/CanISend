# `similar` 3.2.0 candidate qualification

Checked: 2026-09-04

Status: qualified as the R2 candidate; deliberately not added to `Cargo.toml` or `Cargo.lock` in R0.

## Exact source identity

- crates.io version: `3.2.0`, published 2026-08-17, not yanked.
- crates.io package SHA-256:
  `4f66ca1f7aca2474dc10c942eb22feffc897735f54cd1db90138c2fddb490987`.
- A fresh download from the crates.io version endpoint produced the same SHA-256.
- The crate's `.cargo_vcs_info.json` binds Git source
  `e3804055b844f5d0c3cb111f5f6276f60e07459a` in
  [`mitsuhiko/similar`](https://github.com/mitsuhiko/similar/tree/3.2.0).
- crates.io records trusted publishing from that GitHub repository, workflow run `32016763700`,
  at the same source commit.

## Compatibility and dependency surface

- The manifest declares Rust `1.85` and edition `2024`; CanISend pins Rust `1.97` and edition
  `2024`.
- License is Apache-2.0, already allowed by `deny.toml`.
- Default features are only `std` and `text`. They activate no normal transitive dependency.
- Optional `bytes`, `unicode`, `serde`, `web-time`, and `hashbrown` dependencies are unnecessary for
  the selected UTF-8 line-diff path and must remain disabled unless R2 proves otherwise.
- The documented `TextDiffConfig` supports Myers selection, deadline/timeout, grouped operations,
  exact trailing-newline reporting, and line-level iteration required by the bounded design.

## Advisory and source disposition

An exact-package search of the RustSec advisory database on 2026-09-04 returned no advisory for
`similar`. This is candidate evidence, not a substitute for a lockfile audit. When R2 adds the
dependency it must rerun the repository's normal `cargo deny` advisory, license, ban, and source
checks against the actual lockfile and reject any new result.

The source archive contains Rust implementation, tests/bench material, README, changelog, Apache
license, and Cargo metadata only; it adds no build script or native library. R2 must enable only the
features used by `canisend-app`, exercise the accepted maximum-input/deadline fixture, and keep the
crate out of Store, Tauri, and frontend dependencies.

## Decision

Retain `similar` 3.2.0 as the single R2 candidate. Do not add it in R0. Reject or requalify it if
the R2 lockfile changes the dependency surface, the advisory/source checks fail, or the bounded
pathological fixture exceeds the accepted UI budget.

## Sources

- [crates.io `similar` 3.2.0 metadata](https://crates.io/crates/similar/3.2.0)
- [`similar` 3.2.0 source and license](https://github.com/mitsuhiko/similar/tree/3.2.0)
- [`TextDiffConfig` documentation](https://docs.rs/similar/3.2.0/similar/struct.TextDiffConfig.html)
- [RustSec advisory database](https://github.com/RustSec/advisory-db)
