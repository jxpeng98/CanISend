# Alpha.10 exact public release and qualification gap

Date: 2026-08-29

## Exact release binding

- Tag: `v1.0.0-alpha.10`
- Annotated tag object: `6a43aa0889445ae5531736ac8e6d71cc363f6869`
- Source commit: `cd40180f2ff8ac957276f1948ba88da428511a82`
- Candidate workflow run: `32678848156`
- Candidate artifact: `canisend-v1.0.0-alpha.10-release-assets` (`9503978913`)
- Candidate artifact digest:
  `sha256:3594845c857afa1777368813505597323daef1f0433943cd4383693478d24be6`
- Promotion workflow run: `33267148891`
- Public release: <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.10>
- Public release ID: `379063211`
- Public manifest SHA-256: `6669fb73e728d64bea10cb99d4f403ed7ffcb06e15401bfe04f93230a35e7bb5`
- Public `SHA256SUMS` SHA-256:
  `5679fece0bc5af68fc1b135a8cd46e2d4c09375201cf942db2b39cbd8a9e44f5`
- Apple Silicon CLI archive SHA-256:
  `c5339ca17bd7bcf48a374d34bfdbfe140cd1bd0fc5e3f8b6bcb5744e6c93833a`
- Extracted Apple Silicon CLI SHA-256:
  `69d97bbe5c9c16f6737764bb308c9e55e7269c01f805cd7821588128b499170b`

The annotated tag peels to the exact source commit. Promotion reused the successful unexpired
candidate, reported `recompiled_during_promotion: false`, and skipped the source, signing, Windows,
macOS, and five-target build jobs. All five draft CLI smokes and the Apple Silicon App ZIP/DMG
smoke passed before publication.

An independent fresh download verified all 15 manifest-managed files, all 16 public attestations,
and byte-for-byte identity between the candidate artifact and public release. The public CLI
reported version `1.0.0-alpha.10`, source `cd40180f2ff8`, target `aarch64-apple-darwin`, and the
active v4 contracts.

Promotion evidence artifact `9719079855` has digest
`sha256:8375bb234bd48a0d8cb515d47c1f5c5eee8ca7193b4d74ee552acf4b91fa0adf`.
Public verification artifact `9719114194` has digest
`sha256:4a657f7cc85c443a8a9b7c8b2a96b1d161a326bd1e2357eb7bff1b570ea8eab7`.

## Contract binding

- Agent protocol: `canisend.agent/v4`
- Public schema: `4.0.0`
- Workspace format: `canisend.workspace/v4`
- Agent-host resource format: `canisend.agent-host-resources/v4`
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
- MCP inventory: 36 tools, including 26 read/preview tools and 10 guarded commit tools

## Exact-host outcomes

Candidate/public byte identity makes the candidate-host results applicable to the public release.
Host versions observed during final reconciliation were Codex CLI `0.147.0`, Claude Desktop
`1.34493.1`, and Claude Code `2.1.237`.

| Host | Scenario | Result |
|---|---|---|
| Codex CLI | Generic Requirement confirm preview/cancel | Passed; revision `1` and state `proposed` remained unchanged |
| Claude Desktop | Generic Requirement confirm preview/cancel | Passed in an incognito session; revision `1` and state `proposed` remained unchanged |
| Bounded MCP client | Inventory, unknown-field refusal, and Generic preview/cancel | Passed; protocol `2025-11-25`, unknown input failed closed, and no commit tool ran |
| Claude Code | Academic Requirement confirm preview/cancel | `skipped-by-maintainer`; the attempted run stopped before provider access because the OAuth refresh token had expired |

No completed scenario performed a mutation, upload, or submission. Final Workspace integrity was
healthy. Claude Desktop configuration was restored byte-for-byte with SHA-256
`8281e2dfda423041cc5fd1eb93a6a2dd1fdf9b5dd82a8c0aa305ede83fb32cd4`.

## Qualification disposition

Alpha.10 is an exact, same-byte, publicly verified prerelease, but it is not a complete
provider-qualified checkpoint under the accepted Alpha.10 PRD because Claude Code was explicitly
skipped. Issue #194 and affected-scenario Issue #68 therefore remain open. Issue #70 remains bound
to exact fully provider-qualified Alpha.9, and `release/provider-dogfood.json` is not rewritten or
misrepresented as Alpha.10 evidence.

This note retains no application body, transcript, local Workspace path or object identifier,
credential, provider token, approval token, or private user content. Synthetic host results do not
count as invited-user evidence and do not authorize Beta.
