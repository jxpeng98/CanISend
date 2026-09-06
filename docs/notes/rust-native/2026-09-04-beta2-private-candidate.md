# Beta.2 private native candidate verification

Date: 2026-09-04

## Exact candidate binding

- Intended tag: `v1.0.0-beta.2` (not created)
- Source commit: `2ae2b507b953eef3101aa9689bd60f91a0046605`
- Entry PR: <https://github.com/jxpeng98/CanISend/pull/222>
- Protected Fast CI: <https://github.com/jxpeng98/CanISend/actions/runs/33824048853>
- Protected dependency assurance:
  <https://github.com/jxpeng98/CanISend/actions/runs/33824048854>
- Native candidate run: <https://github.com/jxpeng98/CanISend/actions/runs/33824463477>
- Candidate artifact: `canisend-v1.0.0-beta.2-release-assets` (`9920609356`)
- Artifact digest:
  `sha256:38b3666b9e9de07cc55b77e8798f1200e4b76f611418b14598c39524b68d7c03`
- Artifact size: `172023607` bytes
- Artifact retention: created `2026-09-04T02:00:55Z`, expires `2026-10-04T02:00:48Z`
- Release-manifest SHA-256:
  `2a36947740a198d00499adfc5655d9e9a8f62599ce2bacdee836fbec5b5e3206`
- `SHA256SUMS` SHA-256:
  `bb6b7a42227dd6f22939ee618fb25df74458a986f5a4a3b13b248d8fe7d0a124`
- Downloaded files: `20`; manifest-managed files: `19`; verified attestations: `20`
- SBOM: CycloneDX `1.6`, CanISend `1.0.0-beta.2`, `739` components

The candidate workflow completed every source, signing-readiness, native-build, Windows-release,
macOS App, assembly, and attestation job against the source commit above. Candidate lookup,
promotion, draft upload, draft verification, publication, and published-release verification were
all skipped.

Independent verification in a fresh temporary directory passed the complete `SHA256SUMS`,
`release verify-candidate`, `release verify`, and one GitHub provenance check for each of the 20
files. Every attestation identifies the repository release workflow and exact source commit above.

## Product and package evidence

The manifest contains the five supported CLI targets: Apple Silicon macOS, Intel macOS, glibc
Linux x86-64, musl Linux x86-64, and Windows x86-64 MSVC. It also contains the Apple Silicon
macOS portable App ZIP and installer DMG. Both App qualification records passed packaged CLI
doctor, dual-Pack quickstart, Agent v4 host resources, persistent MCP lifecycle, GUI launch,
archive or disk-image integrity, and no-publication checks. Intel macOS desktop evidence remains
compile-only and makes no native-runtime support claim.

Protected Fast CI passed desktop formatting, Svelte/TypeScript checks, UI tests, accessibility,
the complete Rust suites, and Linux, Windows, and macOS CLI/MCP coverage. Its existing Agent v4
smokes exercised Workspace v4 initialization and checking, Codex and Claude project/global Skill
lifecycle, the four version-bound Skills, 36 MCP tools, both built-in Packs, Application creation,
guarded writes, backup/restore, and unsupported-legacy refusal without live data or global host
mutation.

## Signing boundary

The Apple Silicon and Intel CLI archives have valid Apple ad-hoc signatures with hardened runtime
and without `get-task-allow`. They are not Developer ID signed or notarized and do not establish
Gatekeeper publisher trust or secure timestamps. The Windows archive has an intact self-signed
Authenticode signature for `CN=CanISend Community Build`; it is not chain-trusted or publicly
timestamped. These documented community-signing limits are integrity evidence, not public trust.

## Disposition

This is a private, nonpublishing candidate only. No `v1.0.0-beta.2` tag, GitHub Release, package
candidate, external package-index submission, qualification-ledger write, or RC action was
created. Public, qualified `v1.0.0-beta.1` remains unchanged. The consented body-free invited
cohort remains the next pre-RC evidence gate.

This note retains no application body, transcript, prompt, credential, private user content, or
host path.
