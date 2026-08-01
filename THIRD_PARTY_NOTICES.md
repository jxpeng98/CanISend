# CanISend Third-Party Notices

CanISend is distributed under the MIT License in `LICENSE`. The standalone native binary includes third-party Rust
code and embedded rendering assets. This file identifies the components whose notices are especially relevant to the
embedded renderer and storage engine.

## Embedded Typst renderer

- `typst-as-lib` 0.16.0: MIT.
- Typst compiler crates 0.15.1, including `typst`, `typst-kit`, and `typst-pdf`: Apache-2.0.
- Model Context Protocol Rust SDK (`rmcp` and `rmcp-macros`) 3.0.1: Apache-2.0.
- `typst-assets` 0.15.1: Apache-2.0, with additional asset notices supplied by the upstream crate.

The native release bundle includes exact copies of `typst-assets`' upstream `LICENSE` and `NOTICE`. The notice covers
the embedded font families and other compiler assets, including:

- Libertinus Serif: SIL Open Font License 1.1;
- New Computer Modern: GUST Font License / LPPL terms identified by upstream;
- DejaVu Sans Mono: Bitstream Vera and DejaVu attribution terms identified by upstream.

CanISend does not modify or rename these fonts. PDFs created with the fonts are not themselves placed under the font
licenses.

## Embedded ModernPro templates

- `modernpro-cv` 2.0.0 by Academic Template Collective: MIT. The embedded source is based on the versioned Typst
  Universe archive with SHA-256
  `1d108f538571e804f96b59dc1f3c0b0e0dc275b3eb35c6368fd7cc89775851f0`.
- `modernpro-coverletter` 1.0.0 by Academic Template Collective: MIT. The embedded source is based on the versioned
  Typst Universe archive with SHA-256
  `d3c5e8031e8a74ab4ae6e3163b0f37d6ecebc972dd7a4b3b41fc99ff07585130`.

The CanISend copies add an offline structured-data adapter and a bounded configuration-precedence
fix so the selected embedded font wins over an unavailable upstream fallback. They use the already
embedded Libertinus font assets. Optional contact icons remain data-driven, so no additional icon
package or font is bundled.

Copyright (c) 2023 Academic Template Collective

Copyright (c) 2024 Academic Template Collective

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## SQLite storage

- SQLite amalgamation: public domain dedication published by the SQLite project.
- `rusqlite` and `libsqlite3-sys`: MIT.

The binary uses bundled SQLite so end users do not need to install a database library.

## Complete dependency evidence

`Cargo.lock` freezes the complete Rust dependency graph and `deny.toml` defines the accepted license policy. The R10
release pipeline will add a machine-readable SBOM and dependency-wide license report. This notice does not replace
the exact upstream licenses packaged in native release bundles.
