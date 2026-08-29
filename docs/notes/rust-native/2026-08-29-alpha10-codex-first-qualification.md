# Alpha.10 Codex-first qualification

Date: 2026-08-29

## Policy boundary

Codex CLI is the required external Agent host for entry to `v1.0.0-beta.1`. Claude Code and
Claude Desktop remain supported compatibility surfaces generated from the same Agent v4
resources, but their real-host sessions are non-blocking observations for this transition. An
unrun or unauthenticated host session is never reported as passed.

This policy changes release evidence, not product bytes, Agent architecture, host resources, or
the guarded MCP contract.

## Exact public identity

- Tag: `v1.0.0-alpha.10`
- Source commit: `cd40180f2ff8ac957276f1948ba88da428511a82`
- Candidate workflow run: `32678848156`
- Candidate artifact: `canisend-v1.0.0-alpha.10-release-assets` (`9503978913`)
- Candidate artifact digest:
  `sha256:3594845c857afa1777368813505597323daef1f0433943cd4383693478d24be6`
- Promotion workflow run: `33267148891`
- Public release ID: `379063211`
- Public manifest SHA-256:
  `6669fb73e728d64bea10cb99d4f403ed7ffcb06e15401bfe04f93230a35e7bb5`
- Apple Silicon CLI archive SHA-256:
  `c5339ca17bd7bcf48a374d34bfdbfe140cd1bd0fc5e3f8b6bcb5744e6c93833a`
- Extracted Apple Silicon CLI SHA-256:
  `69d97bbe5c9c16f6737764bb308c9e55e7269c01f805cd7821588128b499170b`

The existing candidate, same-byte promotion, public download, attestation, and byte-equality
evidence remains unchanged. The newly downloaded archive and executable matched the public
digests above, and the executable reported `1.0.0-alpha.10`, source `cd40180f2ff8`, target
`aarch64-apple-darwin`, Agent v4, Workspace v4, and host-resource v4.

## Contract binding

- Agent protocol: `canisend.agent/v4`
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

## Required Codex outcomes

Codex CLI `0.147.0` completed both canonical scenarios against exact Alpha.10 bytes:

| Scenario | Pack | Revision | Requirement state | Result |
|---|---|---:|---|---|
| `codex-cli-academic-requirement-preview-cancel` | `org.canisend.academic-job` | `1` → `1` | `proposed` → `proposed` | Passed |
| `codex-cli-generic-requirement-preview-cancel` | `org.canisend.generic-application` | `1` → `1` | `proposed` → `proposed` | Passed |

The Generic outcome is the existing exact Alpha.10 candidate-host result whose bytes were later
verified identical to the public release. The Academic outcome used a fresh public CLI download,
a clean synthetic Workspace, project-local generated Skills, and the exact stdio MCP server. Only
read, Requirement-preview, and integrity-check tools were exposed; no commit tool was available.
The App remained closed. A post-session CLI check confirmed revision `1`, state `proposed`, and a
healthy Workspace.

Both previews returned `previewed`. Neither scenario committed a mutation, exported or uploaded
content, or performed a submission.

## Non-blocking host observations

- Claude Desktop `1.34493.1` previously passed the Generic preview/cancel observation against
  exact Alpha.10 bytes.
- Claude Code `2.1.237` remains `skipped-by-maintainer`; its attempted run stopped before provider
  access after OAuth expiry.
- The bounded MCP client previously passed inventory, unknown-field refusal, and Generic
  preview/cancel checks.

These observations are retained accurately but are not required passed scenarios in
`canisend.provider-dogfood/v2`.

## Consent and disposition

The maintainer explicitly authorized synthetic-data provider dogfood. Provider send was limited
to synthetic metadata. This note retains no Application body, transcript, Workspace path, object
identifier, credential, provider token, approval token, or secret material.

Alpha.10 is the current Codex-qualified checkpoint and the exact entry candidate for a separately
authorized Beta.1 transition. This evidence represents zero invited users and does not authorize
Beta, feature freeze, RC, Stable, upload, or submission. Invited-user evidence is collected after
Beta.1 and remains required before RC planning.
