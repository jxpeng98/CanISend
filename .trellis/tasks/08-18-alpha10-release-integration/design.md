# Alpha.10 headless integration design

## Architecture boundary

CanISend remains one Rust product with three adapters over the application facade:

```text
direct CLI (bootstrap, management, reads)
                         \
                          -> canisend-app -> Core/Store/IO authority
                         /
persistent MCP (guarded preview/approve/commit)

desktop App (optional observer/operator of the same authority)
```

The CLI and MCP ship in the same `canisend` binary. Skills contain workflow guidance only. No
adapter may duplicate business rules, write `.canisend` directly, or create a second approval
store.

## Delivery units

### 1. `M3-HEADLESS-001` — close the App-absent product journey

This child owns the only expected product diff:

- expose the already-supported `AgentSkillsInstallScope` as an explicit CLI host option;
- keep project scope as the backward-safe default;
- reuse existing host install/status/remove application-facade calls;
- align Workspace initialization receipts, Skills, host guidance, and the documented quick start;
- extend the existing App-closed smoke to cover empty-directory initialization, starter
  resources, project/global host lifecycle, both Packs, guarded MCP work, export, backup, restore,
  and App-compatible reopen;
- fix only gaps that the smoke proves in the existing v4 path.

No separate Codex plugin layer, direct CLI aliases for MCP mutations, provider integration, or old
command compatibility is added.

### 2. `M3-ALPHA10-001` — qualify and publish exact Alpha.10

This child begins only after the headless child is merged to protected `main`. It owns:

- sequential Alpha.10 controlled metadata transition;
- exact source gate and protected PR;
- build-once native candidate qualification;
- affected-scenario rerun under existing `M3-EVID-003` / Issue #68;
- exact Codex CLI, Claude Code, Claude Desktop, and bounded MCP-client host evidence;
- same-byte tag promotion, independent public download verification, and truth reconciliation;
- moving Issue #70's cohort baseline from Alpha.9 to Alpha.10 without rewriting Alpha.9 evidence.

## CLI contract

`host setup`, `host status`, and `host remove` gain:

```text
--scope project|global
```

- `project` remains the default and writes only manifest-owned files under the Workspace host
  root.
- `global` delegates root resolution to `canisend-app`; missing user-home discovery fails before
  mutation.
- MCP configuration is still generated, not globally overwritten.
- status and removal use the same scope as installation and report that scope in structured data.
- removal deletes only unchanged manifest-owned files.

The command leaf inventory does not change, so the operation ID remains the existing host
adapter-only identity. The typed operation registry still guards the compiled command tree.

## Workspace bootstrap contract

The existing Workspace v4 initializer remains authoritative. Both CLI and App call it and receive:

- neutral Workspace v4 authority with zero Applications;
- verified README and starter resources;
- Typst Profile example;
- generic Application examples;
- embedded Typst templates;
- both built-in Packs available from the executable.

Initialization never installs Skills implicitly in the CLI. The returned next action directs the
user to an explicit `host setup --host ... --scope ...`, preserving a reviewable filesystem
boundary.

## Headless workflow data flow

1. `workspace init` creates authority and starter files.
2. `profile-source import` admits reviewed Profile content.
3. `host setup` installs canonical Skills and emits host-specific MCP registration guidance.
4. The host starts `canisend --workspace PATH mcp serve`.
5. Direct CLI creates/inspects Applications; MCP performs guarded association, Requirement, Plan,
   Deliverable, review-disposition, and export-preparation mutations.
6. Every mutation remains `orient -> propose -> preview -> approve -> commit -> verify` within one
   MCP process.
7. CLI checks/backups/restores the same authority; App reopen reads the resulting revisions.

## Security and privacy invariants

- No automatic global host-config overwrite.
- No direct Agent database/blob/projection writes.
- No durable or cross-process approval token.
- Denial, replay, stale context, wrong Pack/Application/Workspace, malformed input, and host
  restart leave authority unchanged.
- Private bodies, local paths, credentials, provider tokens, and transcripts are excluded from
  checked-in evidence.
- `submission_performed` remains false.

## Compatibility

This is additive within clean v4. Project-scope CLI behavior remains unchanged when `--scope` is
omitted. Old Skills, host layouts, Python commands, Agent v2/v3, `job` aliases, and Workspace v2/v3
continue to fail closed before mutation.

## Rollout and rollback

- Merge the headless capability PR before any version transition.
- A failing smoke rolls back only the bounded implementation branch; Alpha.9 remains public.
- A failed Alpha.10 candidate is not tagged or published. Fixes create a new exact candidate from
  a new protected source; candidate evidence is never rewritten.
- Promotion locates the qualified candidate and never rebuilds.
- If public verification fails, stop publication claims and retain Alpha.9 as the last qualified
  public checkpoint until corrected bytes complete the full path.
