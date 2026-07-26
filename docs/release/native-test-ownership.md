# Native release test ownership

CanISend runs the complete locked Rust workspace suite once in the candidate source gate. Native
release jobs do not repeat the whole workspace test graph for every target. Their responsibility is
to prove the behavior that depends on the exact target runner and packaged bytes.

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
| Windows CLI job | PowerShell parser plus stage-required self-signed Authenticode verification |
| Linux GNU job | release performance and full synthetic workflow budgets |
| Linux musl job | musl linker and execution of the extracted static-target archive |
| Apple Silicon desktop job | version-matched CLI/GUI build, bounded app archive, companion integrity, nested/outer ad-hoc signatures, packaged workflows, and GUI launch |
| Ordinary CI and scheduled workflows | cross-platform recovery, concurrency, rendering, staged quickstart, performance, dependency assurance, and fuzzing |

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

Timing evidence is diagnostic and never authorizes publication. The release manifest, checksums,
native qualification records, signatures, and GitHub attestations remain authoritative.
