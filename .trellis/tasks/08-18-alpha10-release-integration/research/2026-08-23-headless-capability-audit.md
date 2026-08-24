# Alpha.10 headless capability audit

## Decision recovered from prior sessions

The user first requested 0.x CLI and Skills capability in the 1.0 line, then explicitly replaced
byte/command compatibility with a clean Workspace v4 and Agent v4 design. The current requirement
continues that decision: preserve user outcomes, not old Python commands, old Skills, Agent v2/v3,
`job` aliases, host layouts, or Workspace v2/v3 migration.

## Current protected source

- `main` and `origin/main`: `975e1b18518b5a26c73dd7bd79d3718a257a50e3`
- Current public release: `v1.0.0-alpha.9`
- Exact Alpha.9 source: `4876c5669b7ae48ca053b5e06e0005419d2051f6`
- Alpha.9-to-main changes are evidence, guidance, Roadmap, and Trellis records only.

## Existing reusable implementation

| Capability | Current implementation | Evidence |
|---|---|---|
| Neutral Workspace initialization | `Application::initialize_workspace_v4*` | `crates/canisend-app/src/workspace.rs` |
| Starter guidance, Profile example, examples, Typst templates | `WORKSPACE_STARTER_RESOURCES` | `crates/canisend-app/src/workspace.rs` |
| Standalone CLI | 31 compiled Clap leaves | `crates/canisend-cli/src/lib.rs` |
| Workspace lifecycle | init/status/check/backup/restore/repair | `crates/canisend-cli/src/lib.rs` |
| Project-local host setup | setup/status/remove for Codex, Claude, generic | `crates/canisend-cli/src/lib.rs` |
| Project/global Skill roots | `AgentSkillsInstallScope` | `crates/canisend-app/src/agent.rs` |
| Canonical host resources | four embedded Agent v4 Skills | `crates/canisend-resources/resources/skills/` |
| Persistent guarded mutation transport | 36 MCP tools over stdio | `crates/canisend-mcp/src/lib.rs` |
| Cross-surface identity | typed operation registry | `crates/canisend-contracts/operation-registry-v1.json` |
| App-closed exact-host proof | Codex CLI, Claude Code, Claude Desktop | `docs/notes/rust-native/2026-08-09-m3-agent-v4-real-host-dogfood.md` |

## Confirmed gaps

1. CLI host setup always requests `AgentSkillsInstallScope::Project`; the application facade and
   App already support explicit project/global choice.
2. CLI directly exposes initialization, lifecycle, selected mutations, and reads, but not the
   guarded association, Requirement, Plan, Deliverable, review-disposition, and export-preparation
   mutations. Those mutations already exist in the persistent MCP process and application facade.
3. The public operation registry describes the split but the ordinary headless quick start does
   not present one concise empty-directory-to-restored-Workspace journey as the primary product
   path.
4. The Roadmap marks Alpha.7/8/9 structural and exact-host qualification complete while its M3
   exit and definition-of-done still leave final App/CLI initialization and Agent/Skills/MCP
   integration evidence open.
5. The current source does not ship the old Codex plugin manifest. The user accepted Skills plus
   bundled MCP as the supported headless definition, so a new plugin packaging layer is not
   required unless an existing host cannot load the canonical Skills directly.

## Agreed target boundary

- One standalone `canisend` binary is sufficient while the App is closed.
- Direct CLI commands own initialization, management, inspection, recovery, and other operations
  that do not need a persistent approval token.
- The same binary's `mcp serve` process owns guarded mutations and consumes single-use approvals.
- Skills guide Codex and Claude hosts across those two surfaces; they never write authority.
- No second business-logic implementation and no durable cross-process token store are added.

## Minimal delivery split

1. Headless/bootstrap closure: expose project/global host scope in CLI, align next actions and
   docs, and qualify clean Workspace starter behavior.
2. Agent/MCP journey closure: prove the complete dual-Pack App-closed workflow and correct only
   real registry/resource/guide gaps found by that smoke.
3. Alpha.10 qualification: reconcile Roadmap/GitHub truth, perform the sequential version
   transition, qualify exact build-once bytes, promote, publicly reverify, and record host evidence.

The first two outcomes can be independently reviewed and rolled back. The release child depends
on both and must not begin from unmerged or locally qualified source.
