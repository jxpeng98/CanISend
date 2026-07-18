# CanISend Third-Party Notices

CanISend is distributed under the MIT License in `LICENSE`. The standalone native binary includes third-party Rust
code and embedded rendering assets. This file identifies the components whose notices are especially relevant to the
embedded renderer and storage engine.

## Embedded Typst renderer

- `typst-as-lib` 0.16.0: MIT.
- Typst compiler crates 0.15.1, including `typst`, `typst-kit`, and `typst-pdf`: Apache-2.0.
- `typst-assets` 0.15.1: Apache-2.0, with additional asset notices supplied by the upstream crate.

The native release bundle includes exact copies of `typst-assets`' upstream `LICENSE` and `NOTICE`. The notice covers
the embedded font families and other compiler assets, including:

- Libertinus Serif: SIL Open Font License 1.1;
- New Computer Modern: GUST Font License / LPPL terms identified by upstream;
- DejaVu Sans Mono: Bitstream Vera and DejaVu attribution terms identified by upstream.

CanISend does not modify or rename these fonts. PDFs created with the fonts are not themselves placed under the font
licenses.

## SQLite storage

- SQLite amalgamation: public domain dedication published by the SQLite project.
- `rusqlite` and `libsqlite3-sys`: MIT.

The binary uses bundled SQLite so end users do not need to install a database library.

## Complete dependency evidence

`Cargo.lock` freezes the complete Rust dependency graph and `deny.toml` defines the accepted license policy. The R10
release pipeline will add a machine-readable SBOM and dependency-wide license report. This notice does not replace
the exact upstream licenses packaged in native release bundles.
