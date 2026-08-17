# Alpha.9 exact public host dogfood

Date: 2026-08-16

## Exact release binding

- Tag: `v1.0.0-alpha.9`
- Source commit: `4876c5669b7ae48ca053b5e06e0005419d2051f6`
- Candidate workflow run: `31609344160`
- Candidate artifact: `canisend-v1.0.0-alpha.9-release-assets` (`9147597003`)
- Candidate artifact digest:
  `sha256:da3c6a5c0aab4cc7f41c2fb1a33fc3c2769232ed74d0333e73f0a33cd5d489d9`
- Promotion workflow run: `31618836210`
- Public release: <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.9>
- Public manifest SHA-256: `6d3e5e64dcb6663b5122c70420dc3e16d8c8e3aed8c3bcec35b4ba101537ba5b`
- Public `SHA256SUMS` SHA-256: `13e9de63e54fa0a011d58146fd83ae4ee0cdd05cc25b6dba0d2d2ba59202573f`
- Apple Silicon CLI archive SHA-256: `5ef236bcf4feef7232efaa4b2501d3f1b3cc26e4d96759b10860e6cd1a806e2d`
- Extracted CLI SHA-256: `ffcdabbddaa9407db742adc5a39da9849879d5922ae248cf8bbbea222021f0b5`

The downloaded archive and manifest matched the public checksum file. The extracted standalone CLI
reported Alpha.9, source `4876c5669b7a`, target `aarch64-apple-darwin`, and the active v4
contracts. It passed the repository's full guarded dual-Pack MCP lifecycle smoke without using a
development binary.

## Candidate qualification links

The same candidate run passed the existing gates that own the supporting assertions:

- [source gates](https://github.com/jxpeng98/CanISend/actions/runs/31609344160/job/94157420891)
- [Windows recovery and render tests](https://github.com/jxpeng98/CanISend/actions/runs/31609344160/job/94157502506)
- [Apple Silicon archive build and exact-archive smoke](https://github.com/jxpeng98/CanISend/actions/runs/31609344160/job/94157502775)
- [release assembly and attestation](https://github.com/jxpeng98/CanISend/actions/runs/31609344160/job/94164337073)

## Contract binding

- Agent protocol: `canisend.agent/v4`
- Workspace format: `canisend.workspace/v4`
- Agent-host resource format: `canisend.agent-host-resources/v4`
- Task-resource model SHA-256:
  `3d0ed27c1f244810dffc14eeb5bd678565934d244d5c433c2ca32d432b06f648`
- Academic Pack digest:
  `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`
- Generic Pack digest:
  `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`
- `canisend-workspace` Skill SHA-256:
  `012d2cf2f4e53c5cdf489554c2588bec03b4f1cdad4c20b4059ec6b2f132adf1`
- `canisend-intake` Skill SHA-256:
  `4c26eed5f18cc183b78733203d025cd0003a66a2d96b59356f46614cc952b0ec`
- `canisend-materials` Skill SHA-256:
  `8106c5ec0052e4430240b38a88109cba1255a03da7f4e3b9a2b322f4eed94580`
- `canisend-review-export` Skill SHA-256:
  `868f39b71bbd496e6731fffb2d13cb39cf3beca0da1b98d5326e02c2816b1efd`

## Exact-host outcomes

A fresh synthetic Workspace contained both built-in Packs. Each host completed the canonical
Requirement confirm preview/cancel flow and then independently reported the Application revision
and Requirement state unchanged.

| Host | Version | Scenario | Result |
|---|---|---|---|
| Codex CLI | `0.147.0` | Generic Requirement confirm preview/cancel | Passed; an extra input field failed closed before the corrected preview |
| Claude Code | `2.1.231` | Academic Requirement confirm preview/cancel | Passed under a narrow tool allowlist |
| Claude Desktop | `1.26832.0` | Generic Requirement confirm preview/cancel | Passed in a new incognito chat with one-session approvals |

No scenario invoked a commit tool, performed a mutation, uploaded content, or submitted an
Application. The final integrity check reported both Applications unchanged and no stale,
missing, unreferenced, or repairable data.

Claude Desktop's pre-existing configuration was backed up, temporarily pointed at the exact public
Alpha.9 CLI, and restored byte-for-byte after the App was closed. The original, backup, and
restored configuration SHA-256 was
`8281e2dfda423041cc5fd1eb93a6a2dd1fdf9b5dd82a8c0aa305ede83fb32cd4`.

The standard-chat stale-memory attempt remains rejected as provider evidence under Issue #67. Only
the new incognito-chat result is admissible for this qualification.

## Privacy boundary

The maintainer explicitly authorized synthetic-data provider dogfood and temporary one-session host
configuration. Only synthetic metadata and body-free state were sent. This note retains no
application body, transcript, local Workspace path or object identifier, credential, provider
token, approval token, or private user content. It is exact public-host evidence, not invited-user
evidence and not Beta authorization.
