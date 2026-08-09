# M3 Agent v4 real-host dogfood

**Date:** 2026-08-09

**Roadmap items:** `M3-AGENT-001`, `M3-AGENT-002`, `M3-AGENT-003`, `M3-EVID-001`

**Data:** synthetic, body-free, local temporary directories only

## Exact draft candidate

- source commit: `0b9ee8dceca12cf7c543775c993485215454dc6c`;
- source tree: `2e1e8dfdce31d33e41796958fb19cdbdc4931096`;
- standalone CLI SHA-256:
  `210264eaf1976e3c5af050cca2496123c8d2eff8f226379e181c60a06314e597`;
- standalone CLI size: `48612912` bytes;
- product version: `1.0.0-alpha.7`;
- Workspace format/schema: `canisend.workspace/v4` / `20`;
- Agent/resource formats: `canisend.agent/v4` / `canisend.agent-host-resources/v4`;
- task-resource-model SHA-256:
  `3d0ed27c1f244810dffc14eeb5bd678565934d244d5c433c2ca32d432b06f648`;
- Codex resource manifest SHA-256:
  `afeebed6dc269d05703986fbc7237ee23a204de97187f79e5b6ce0d78ce08414`;
- Claude resource manifest SHA-256:
  `72705440f255c2ccea7e7564a0458700fb32b085ed5a17297fbd0819adf18bbd`;
- academic Pack `1.0.0` digest:
  `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`;
- generic Pack `1.0.0` digest:
  `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`.

The four Claude Code `SKILL.md` files matched the canonical Codex copies byte for byte:

- Workspace: `012d2cf2f4e53c5cdf489554c2588bec03b4f1cdad4c20b4059ec6b2f132adf1`;
- intake: `4c26eed5f18cc183b78733203d025cd0003a66a2d96b59356f46614cc952b0ec`;
- materials: `8106c5ec0052e4430240b38a88109cba1255a03da7f4e3b9a2b322f4eed94580`;
- review/export: `868f39b71bbd496e6731fffb2d13cb39cf3beca0da1b98d5326e02c2816b1efd`.

These local bytes are not public Alpha.7 qualification evidence.

## Host matrix

| Scenario | Host | Outcome |
|---|---|---|
| `codex-new-cancel` | Codex CLI `0.146.0` | Loaded Workspace/intake Skills, found both Packs, created a generic Requirement preview, and cancelled without mutation. |
| `codex-resume-commit` | Codex CLI `0.146.0` | Rejected the prior process token, reoriented, committed one fresh generic Requirement confirmation, and verified revision 2 while academic remained accessible. |
| `codex-stale-recovery` | Codex CLI `0.146.0` | Proposed and confirmed the generic Plan through fresh guarded previews, rejected an explicit revision-3 preview after authority reached revision 4 with `workspace.conflict`, reoriented, and verified that academic remained unchanged. |
| `claude-desktop-new-cancel` | Claude Desktop `1.24012.9` | Loaded the exact local MCP, found both Packs, created an academic Requirement preview, and cancelled without mutation. |
| `claude-desktop-resume-commit` | Claude Desktop `1.24012.9` | Reloaded MCP, rejected the prior token, committed one fresh academic Requirement confirmation, and verified revision 2 while generic remained unchanged. |
| `claude-code-new-cancel` | Claude Code `2.1.222` | Loaded Workspace/materials Skills, found both Packs, created an academic Plan preview, and cancelled without mutation. |
| `claude-code-resume-stale` | Claude Code `2.1.222` | Rejected the prior process token, committed a fresh Plan, committed the first of two same-revision confirmation previews, rejected the second as stale, reoriented, and verified the confirmed Plan at revision 4. |

The final generic snapshot was revision 4 with SHA-256
`7ba6c7aeb3bba97ec58992f97571e34ef904f802f86dcefed1f1a8f06df40024`. The final academic
snapshot was revision 4 with SHA-256
`408aff8824702c53c43bce400f231c2fd2e12760f4e0304b6b7940aec42207fb`.

## Fail-closed host findings

- A stale public Alpha.6 executable discovered through `PATH` was rejected for its pre-v4 identity
  before mutation; the exact candidate executable then passed.
- A malformed Requirement decision enum was rejected by MCP deserialization before mutation.
- Non-interactive Codex cancelled a guarded write despite explicit host configuration; the same
  session completed through Codex's interactive approval path without bypassing safeguards.
- Duplicate same-revision Plan proposal and confirmation tokens were invalidated at the approval
  Broker after the first commit with `approval.binding-mismatch`. Codex did not relabel that result
  as stale: it issued one explicit revision-3 preview after authority reached revision 4, received
  `workspace.conflict` (`expected 3, found 4`), reoriented, and performed no further mutation.
- Claude Code with project-only setting sources excluded its authenticated user layer and failed
  before MCP use. Removing that unnecessary host flag retained strict MCP isolation and restored
  the authenticated Code flow.
- Claude Desktop and Claude Code use separate MCP configuration locations. The same generated
  stdio entry worked for both; the Desktop user-global merge preserved its unrelated server entry.

## Consolidated failure matrix

The exact standalone CLI also passed `scripts/smoke_agent_v4_mcp.sh` as a separate, conforming
JSON-RPC client against a fresh synthetic dual-Pack Workspace. The matrix used the App-absent stdio
surface and ended with generic revision 6, academic revision 6, and a passing Workspace integrity
check.

| Boundary | Exact-candidate outcome |
|---|---|
| Missing runtime | Claude Code loaded an isolated MCP entry whose CanISend executable did not exist and rejected it with `ENOENT` in 6 ms; the dogfood Workspace remained at generic/academic revision 4. |
| Missing authentication | An isolated Claude Code configuration reported `loggedIn: false`; its exact CanISend MCP connected, but the host stopped before any tool call with `Not logged in`, zero model tokens, and zero cost. |
| Malformed output | MCP deserialization rejected an invalid Requirement decision, and the packaged smoke rejected a cross-Pack Deliverable kind before preview or mutation. |
| Invented ID | `canisend_deliverable_show` rejected a synthetic Deliverable ID with JSON-RPC `-32602`, `input.invalid`, and no other-Application data. |
| Wrong context | A generic Plan preview token could not commit against the academic Application; a fresh correctly bound preview was required. |
| Stale revision | Codex received `workspace.conflict` for expected revision 3 after authority reached revision 4, reoriented, and did not mutate. |
| Denied consent | Both Pack audits failed with `confirmed_private_read: false`; the responses contained no private marker. Fresh approved audits succeeded. |
| Host restart | Codex, Claude Code, and Claude Desktop rejected tokens minted by the prior MCP process; each successful continuation used a fresh preview. |

The packaged smoke additionally rejected preview replay, duplicate commit, and Pack-invalid output.
Its final snapshots retained the correct Pack identity and Deliverable counts, and its isolation
assertions proved that reads did not cross the generic/academic Application boundary.

No failure required the CanISend App to run. No source body, Deliverable body, transcript,
credential, provider token, or approval token is retained here. Every successful mutation used an
explicit fresh preview approval through the MCP process. No direct internal write, provider send,
upload, or submission occurred.

## Remaining boundary

This record proves the frozen local draft candidate only. The evidence-only commits containing this
note change repository identity and do not retroactively qualify new bytes. Five-target native
qualification, promotion of the same build-once artifacts, public download, and public-byte
reverification remain mandatory for `M3-ALPHA7-001`. Invited-user evidence remains post-release.
