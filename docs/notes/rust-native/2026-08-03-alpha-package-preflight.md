# Alpha release-profile and Apple Silicon package preflight

Date: 2026-08-03

## Scope and identity boundary

This nonpublishing preflight used source
`602a9ea4abe1dcb92fde67d1739ca6a4dfb0b739` on an Apple Silicon Mac. It exercised the exact Alpha
Cargo profile and local macOS packaging/smoke scripts without changing the source version, creating
a tag, pushing, dispatching a candidate, or publishing any asset.

The source still reported `1.0.0-alpha.5` but was 37 commits beyond the public Alpha.5 tag. The
files below are therefore diagnostic only: their Alpha.5 names do not make them Alpha.5 release
candidates, and their hashes cannot authorize Alpha.6. The source already carries the post-Alpha.5
GPL transition, so the staged notice bundle contains GPLv3; a genuine historical Alpha.5 package
must still be reproduced from its immutable tag with its historical MIT facts.

## CLI package path

The exact native command built `canisend-cli` for `aarch64-apple-darwin` with the `release-alpha`
profile. `package_native_release.sh` then staged the binary and complete notices into a compressed
archive, and `smoke_release_archive.sh` verified the extracted bytes.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| optimized CLI binary | 55,312,704 | `6440f879c0fb360fb8be1ab55876749435c8b480d85749475e0d77bd67e7120e` |
| CLI `tar.gz` | 25,274,972 | `ee82789ace39ae73f84f030722e004d62dd3d640890b678cc4786aa509fd00bf` |

The extracted archive matched the built binary byte-for-byte and passed version, doctor, embedded
Typst, complete notice bundle, dual-Pack documented quick-start, Host Agent, isolated installation,
uninstallation, and Workspace-retention checks.

## Unified desktop package path

The exact native command built `canisend-gui` with `release-alpha`, target
`aarch64-apple-darwin`, and `canisend-gui/custom-protocol`. `package_macos_gui_release.sh` created one
unified GUI/CLI/MCP application, ad-hoc signed the nested host and outer bundle, froze a companion
integrity manifest, and produced the ZIP and compressed read-only DMG.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| optimized unified host | 66,428,416 | `dbb96d3604f04a01d0dac9ccb5c7ae276ae01e17e76ec4800a3cef9dad89e3dc` |
| desktop ZIP | 27,571,201 | `0b25e8f4022d629fd9f2e837e8a12cc6ae45c09adf84018ca7f7e5888e7a62be` |
| desktop DMG | 30,514,503 | `d32b797cab0d838b1527ea00741613867ca6c254c6c919c62437e211a95faf19` |

The ZIP smoke verified exact top-level shape, bounded size, no archive symlinks, companion integrity,
nested and outer ad-hoc signatures, version/doctor, dual-Pack quick-start, Host Agent workflow, and
an actual packaged GUI launch. The DMG checksum and read-only mount passed; its top level contained
only `CanISend.app`, its manifest, and the exact `/Applications` link, and the mounted application
passed signature and integrity verification.

Native GUI launch and disk-image mounting require host OS permission and therefore ran outside the
filesystem sandbox. No network, external repository, installed application, version authority, or
release channel was changed.

## Roadmap effect

This reduces Apple Silicon packaging uncertainty before M2-CANDIDATE-001. It does not close that
task: the authorized Alpha.6 transition must first produce one clean candidate commit, and the full
five-target CLI plus Apple Silicon desktop matrix must rebuild once on exact native runners. The
candidate's archives, manifests, SBOM, signatures, provenance, lifecycle evidence, Agent dogfood,
promotion, and public re-verification must bind the new Alpha.6 hashes rather than any value in this
preflight.
