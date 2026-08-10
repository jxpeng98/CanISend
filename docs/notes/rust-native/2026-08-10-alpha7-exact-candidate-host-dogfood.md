# Alpha.7 exact-candidate host dogfood

**Date:** 2026-08-10

**Roadmap:** `M3-AGENT-001`, `M3-AGENT-002`, `M3-EVID-001`, `M3-ALPHA7-001`

**Data:** synthetic, body-free, isolated temporary Workspace

## Candidate binding

- release workflow run: [31339441501](https://github.com/jxpeng98/CanISend/actions/runs/31339441501);
- release artifact: `9045783528` (`canisend-v1.0.0-alpha.7-release-assets`);
- source commit: `9986a6a63b596b7760b4721a7e97c36aedce6d51`;
- product/target: `1.0.0-alpha.7` / `aarch64-apple-darwin`;
- CLI archive SHA-256: `0610935c8e5ebbddae33b93dd6fb413d0c064b64908bdf8916ec7632d17e7ed1`;
- extracted CLI SHA-256: `ef082fdd73810c0668c74fbacb5ad63de819b76c028f69cb60999d7259cde6a9`;
- Workspace/protocol/schema: `canisend.workspace/v4`, `canisend.agent/v4`, schema `20`;
- task-resource-model SHA-256:
  `3d0ed27c1f244810dffc14eeb5bd678565934d244d5c433c2ca32d432b06f648`;
- Codex/Claude host manifest SHA-256:
  `afeebed6dc269d05703986fbc7237ee23a204de97187f79e5b6ce0d78ce08414` /
  `72705440f255c2ccea7e7564a0458700fb32b085ed5a17297fbd0819adf18bbd`;
- academic Pack `1.0.0` digest:
  `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`;
- generic Pack `1.0.0` digest:
  `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`.

The four installed Skill hashes matched the candidate manifests: Workspace
`012d2cf2f4e53c5cdf489554c2588bec03b4f1cdad4c20b4059ec6b2f132adf1`, intake
`4c26eed5f18cc183b78733203d025cd0003a66a2d96b59356f46614cc952b0ec`, materials
`8106c5ec0052e4430240b38a88109cba1255a03da7f4e3b9a2b322f4eed94580`, and
review/export `868f39b71bbd496e6731fffb2d13cb39cf3beca0da1b98d5326e02c2816b1efd`.

## Exact host matrix

| Host | Scenario | Outcome |
|---|---|---|
| Codex CLI `0.146.0` | Load v4 Skills and exact MCP; list both Packs; preview and cancel the generic Requirement confirmation | Passed. An invalid `confirmed` enum failed closed; the corrected `confirm` preview succeeded. No commit ran and revision remained `1`. |
| Claude Code `2.1.222` | Load project v4 Skills and exact MCP; list both Packs; preview and cancel the academic Requirement confirmation | Passed with no permission denial. No commit ran and revision remained `1`. |
| Claude Desktop `1.26832.0` | Full restart with exact user-level stdio entry; incognito chat; list both Packs; preview and cancel the generic Requirement confirmation | Passed. The exact generic ID and revision were re-read after preview; revision remained `1` and the Requirement remained proposed. |

The exact candidate MCP exposed 36 tools and both host resource sets remained `up-to-date`.
The final Workspace integrity check passed with two referenced Blobs and no issue, stale artifact,
unreferenced Blob, or projection repair.

## Desktop client boundary

`Reload MCP Configuration` did not attach the changed local server after a corrected configuration;
a full Claude Desktop restart did. The first ordinary chat then reused prior dogfood memory and
reported stale Application IDs, so it was rejected as evidence. A native incognito chat re-ran every
read and preview against the exact candidate and produced the passing Desktop result above. The
original Desktop configuration was restored byte for byte and its prior CanISend server reconnected.

Candidate qualification should use a new incognito Desktop chat after a full restart whenever the
stdio executable or Workspace binding changes. Stale chat summaries are host output, not CanISend
authority, and must never be accepted without current MCP receipts.

## Privacy and release boundary

No source body, Deliverable body, transcript, credential, provider token, approval token, or private
identifier is retained in this note. No commit tool, direct internal write, upload, or submission ran.
The temporary Workspace ended with both Applications at revision `1` and their original snapshot
digests. This evidence binds the nonpublishing exact Alpha.7 candidate; it is not invited-user or
public-download evidence and does not qualify different bytes.
