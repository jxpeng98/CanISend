# R9.1 embedded Typst compiler note

## Outcome

The production Rust workspace now contains the in-process Typst compiler that was proven during R0. This is no
longer an excluded spike dependency: `canisend-io` privately owns `typst-as-lib = 0.16.0` and `typst-pdf = 0.15.1`,
and no Typst type appears in a CanISend public contract.

`canisend doctor --json` compiles the embedded Cover Letter template with embedded default fonts and exports a PDF
in memory. This keeps the renderer reachable through the optimized executable, rather than allowing LTO to remove an
adapter that only unit tests call.

## Restricted world

The adapter constructs a new Typst engine for one bounded in-memory source. It intentionally adds no filesystem
resolver and compiles `typst-as-lib` without the `packages`, `ureq`, or Typst-side `reqwest` features. The font search
option is configured with system fonts disabled and embedded fonts enabled. An import of an online Typst package and
a read of an absolute local path both fail closed in tests.

The embedded Typst assets provide Libertinus Serif, New Computer Modern, New Computer Modern Math, and DejaVu Sans
Mono. The assets carry their upstream license and notice metadata in the pinned dependency; R9.4 will copy the full
notices into each release archive.

Compiler and PDF diagnostics are reduced to an error category and diagnostic count. The adapter never formats the
Typst source diagnostic, attempted path, package identifier, or application body into its error string.

## Bounds and remaining isolation risk

The current bounds are:

- Typst source: 1 MiB;
- rendered PDF: 16 MiB;
- observed compile plus PDF export time: 10 seconds.

The source cap limits input memory, and the output cap rejects oversized successful PDFs. Typst 0.15.1 does not
expose a safe compilation cancellation hook, so the elapsed limit is checked after compilation and after PDF export;
it detects but cannot preempt a CPU or memory overrun inside the library. R10 must decide whether release hardening
requires a bounded worker process for adversarial or user-authored Typst source. R9.2 will only generate source from
validated structured documents and an embedded template, which substantially narrows the default threat surface.

## Measured cost

On the local macOS arm64 development host:

- the release binary is 48,774,160 bytes (approximately 47 MiB);
- the packaged `doctor` render self-check completed in 0.74 seconds;
- the first clean optimized compiler integration build took 2 minutes;
- the two renderer unit tests completed in 0.24 seconds once built.

Before a CLI path referenced the renderer, LTO produced a roughly 13.4 MB binary. That smaller result was rejected as
evidence because it did not prove the distributed executable contained a usable compiler.

## Verification

Local acceptance passed:

- feature-tree inspection showing only Typst embedded-font features and no package resolver;
- formatting and full-workspace Clippy with warnings denied;
- 70 Rust tests, including restricted file/package access, body-free errors, embedded template/font PDF generation,
  and the binary-level `doctor` contract;
- 38 generated-schema and 48 embedded-resource checks;
- locked release compilation and packaged host-agent smoke.

The preceding R8.5 checkpoint independently passed GitHub Actions run `29626595493` in 2 minutes 21 seconds.
