# Alpha.8 exact public host dogfood

Date: 2026-08-11

## Exact release binding

- Tag: `v1.0.0-alpha.8`
- Source commit: `35e7c822ea2f469ab726a31b5d08e622f6810c55`
- Candidate workflow run: `31424861083`
- Candidate artifact: `canisend-v1.0.0-alpha.8-release-assets` (`9077805013`)
- Promotion workflow run: `31441308337`
- Public release: <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.8>
- Public manifest SHA-256: `de50eb733d16772d798d264b9d655b16216b098bf28bb7f3239d243cd336d480`
- Public `SHA256SUMS` SHA-256: `ad300023e41ff7de67d5eb400cfeb7970016d2dd71da8e9fcc49254002ca098c`
- Apple Silicon CLI archive SHA-256: `115e39e634a6e597918f55822c8ce7070bdc2b2dcc25a529637b8b15e64b27fe`
- Extracted CLI SHA-256: `0e431779a59d1ac93eec695a1ad80bb7e86a1494141739bf19fdeb7c857c9f71`

The tag promotion located the reviewed candidate and published it without recompilation. All 15
entries in the downloaded checksum manifest matched, all 16 public assets passed GitHub artifact
attestation verification, and the downloaded candidate passed the repository release verifier.

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

The standalone public CLI passed the bounded full guarded dual-Pack MCP lifecycle smoke. A fresh
synthetic Workspace then contained one Generic and one Academic Application, each at revision
`1`, with its Requirement still `proposed` after every host scenario.

| Host | Version | Scenario | Result |
|---|---|---|---|
| Codex CLI | `0.146.0` | Generic Requirement confirm preview/cancel | Passed; an invalid extra revision field failed closed before the corrected preview |
| Claude Code | `2.1.222` | Academic Requirement confirm preview/cancel | Passed |
| Claude Desktop | `1.26832.0` | Generic Requirement confirm preview/cancel | Passed after a full restart in a new incognito chat |

No scenario invoked a commit tool, performed a mutation, uploaded content, or submitted an
Application. The final Workspace integrity check reported both Applications unchanged and no
stale, missing, unreferenced, or repairable data.

Claude Desktop received only one-session approvals. Its pre-existing configuration was backed up,
temporarily pointed at the exact public Alpha.8 CLI, and restored byte-for-byte after the test; the
restored configuration SHA-256 was
`8281e2dfda423041cc5fd1eb93a6a2dd1fdf9b5dd82a8c0aa305ede83fb32cd4`.

## Privacy boundary

The maintainer explicitly authorized synthetic-data provider dogfood. Only synthetic metadata and
body-free state were sent. This note retains no application body, transcript, local Workspace
path, credential, provider token, approval token, or private user content. It is exact public-host
evidence, not invited-user evidence and not Beta authorization.
