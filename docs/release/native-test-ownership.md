# Native release test ownership

CanISend runs the complete locked Rust workspace suite once in the candidate source gate. Native
release jobs do not repeat the whole workspace test graph for every target. Their responsibility is
to prove the behavior that depends on the exact target runner and packaged bytes.

Development uses three Apple Silicon jobs in `.github/workflows/fast-ci.yml`. The first owns locked
Svelte dependencies, type checks, unit tests, and one production static-asset build. It uploads
those exact assets; the formatting/Clippy/release-contract job and the complete
workspace/property/debug-build job consume the same artifact before compiling the Tauri binary.
The two Rust jobs then run in parallel. No job builds a release profile or uses a Windows/Linux
runner.

The initial and exact-cache warm measurements are recorded in
[`fast-ci-stage6.json`](../performance/fast-ci-stage6.json). Their critical paths were 287 and 99
seconds respectively, so both stay below the five-minute development budget.

The machine-readable authority is
[`release/native-test-ownership.json`](../../release/native-test-ownership.json). `xtask release
check` rejects policy drift, a missing source suite, a repeated target workspace suite, or removal
of the named native package gates.

## Gate ownership

| Owner | Required evidence |
|---|---|
| Candidate source gate | format, full workspace Clippy, complete locked workspace tests, generated property contracts, release contracts, dependency policy |
| Every CLI native job | locked target release build; exact archived-binary comparison; extracted version, doctor, quickstart, host-agent, isolated install/uninstall, and workspace-retention smoke |
| macOS CLI jobs | native architecture plus stage-required ad-hoc signing |
| Intel macOS candidate job | standalone CLI for every stage; exact-commit GUI compilation evidence for Beta and later |
| Scheduled Intel GUI workflow | weekly/manual Alpha development compile regression; never release evidence or a support claim |
| Scheduled Windows/Linux desktop workflow | latest Typst template audit; four-candidate desktop profile matrix; native one-host NSIS/MSI, DEB/RPM/AppImage builds; GUI/CLI/MCP smoke; and size records; nonpublishing until promoted into the candidate matrix |
| Windows CLI job | PowerShell parser plus stage-required self-signed Authenticode verification |
| Linux GNU job | release performance and full synthetic workflow budgets |
| Linux musl job | musl linker and execution of the extracted static-target archive |
| Apple Silicon desktop job | version-matched CLI/GUI build, bounded ZIP, compressed read-only DMG with `/Applications` link, companion integrity, nested/outer ad-hoc signatures, packaged workflows, and GUI launch |
| Svelte fast CI | locked dependencies, Svelte/TypeScript checks, focused unit tests, and production static-asset build |
| macOS Rust fast CI | development formatting, Clippy, complete workspace tests, generated properties, debug CLI/GUI build, recovery/render coverage, and CLI/host-agent smoke |
| Windows release tests | PowerShell parsers plus bounded recovery, concurrency, embedded-font, complex-layout, and revision-bound render contracts |
| Native release source and package gates | Linux full suite, dependency policy, GNU performance/synthetic budgets, Linux/Windows exact package smoke, and signing checks |
| Scheduled workflows | Intel GUI compilation, Windows/Linux desktop package qualification, and bounded malformed-input fuzzing outside the edit loop |

Windows and Linux tests are release-only. The candidate Windows gate and Linux source gate begin
alongside the native package jobs; assembly waits for every owner, so parallelization changes
feedback time without allowing a failed source or platform test to authorize an artifact.

The scheduled desktop owners compare `release`, `size-s-thin`, `size-z-thin`, and `size-z-fat`
against the exact upgraded Typst template contract before packaging. The smallest material result
is still only a candidate. The workflow currently packages the leading `size-z-fat` candidate so
that the exact host passes that target's GUI, renamed CLI, MCP, rendering, package lifecycle, and
integrity gates before the production profile changes. A native failure retains that target's
existing production profile.

Alpha candidates use the explicit `release-alpha` profile. Beta, RC, Stable, and the scheduled
Intel GUI compile keep the canonical `release` profile. The stage selector is emitted only after
the tag has passed validation, and each release artifact records the selected profile.

The extracted archive is compared byte-for-byte with the target binary before it is executed.
`version`, `doctor`, the documented workflow, host-agent workflow, and isolated
installation/uninstallation therefore run against the actual bytes intended for release rather
than a development substitute.

## Timing evidence

Each successful native CLI and desktop package job uploads a body-free
`canisend.native-release-timing/v1` record. It separates:

- locked release compilation;
- target-specific validation such as signing, performance, or Intel compile-only work;
- packaging; and
- exact extracted-archive smoke.

The timing record includes the validated build profile. An Alpha record with `release`, or a
Beta/RC/Stable record with `release-alpha`, is rejected before evidence is written.

Timing evidence is diagnostic and never authorizes publication. The release manifest, checksums,
native qualification records, signatures, and GitHub attestations remain authoritative.

## Compiler cache

Candidate source, native CLI, and Apple Silicon desktop builds use `sccache` with the GitHub
Actions cache backend. The scheduled Intel GUI compile uses the same bounded mechanism. The
installation action is pinned to an immutable commit, installs `sccache` `v0.16.0`, and verifies
the official release SHA-256 sidecar before extraction.

The existing Rust cache stores the Cargo registry only; it does not restore `target/`. Each
`sccache` namespace includes the target, Rust version, profile, and feature/package set. If
installation or server startup fails, the workflow does not set `RUSTC_WRAPPER` and continues with
the unchanged Cargo command. Server I/O errors also fall back to the real compiler.

Each owner records a body-free `canisend.sccache-stats/v1` document containing compile requests,
hits, misses, cache errors, hit rate, compiler/cache durations, and the measured compile window.
Time saved remains `null` until cold and warm candidates are compared on the same runner class.
These statistics are diagnostic job artifacts: they are excluded from the release manifest and
cannot authorize publication or replace source, archive, signature, checksum, or provenance
verification.

The completed Alpha comparison is preserved in
[`release-pipeline-stage4.json`](../performance/release-pipeline-stage4.json). Its three
non-publishing candidates use the same tag, source commit, and runner classes; the warm run reuses
`stage4-v1`, while the intentionally invalidated run uses `stage4-v2`.

The Alpha-fast profile comparison is preserved in
[`release-pipeline-stage5.json`](../performance/release-pipeline-stage5.json). It compares cold
canonical and `release-alpha` candidates across every package owner, records archive and runtime
effects, and keeps the observed GNU and Windows compile regressions visible for targeted follow-up.
