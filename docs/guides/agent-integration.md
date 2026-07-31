# Agent integration

CanISend is designed to run beside Codex, Claude, or another agent host. The native binary owns validation,
revisioning, privacy scopes, workflow state, storage, and rendering. The host agent owns conversation and bounded
semantic reasoning.

## Recommended: continue in the agent host

The desktop Agent screen prepares the selected workspace, optional job, and Codex or Claude in one explicit action.
It safely installs or upgrades four CanISend-owned workflow skills in the host's project discovery directory, then
returns:

- a safely quoted one-step terminal command that opens the selected workspace and supplies the body-free start point;
- the same optimized starting message for manual use;
- the recommended `canisend-application` orchestration skill;
- exact `agent capabilities` and job-scoped `agent context` commands; and
- current blockers and next actions from the same Rust application facade used by the GUI and CLI.

Copy the one-step command into Terminal. Codex receives `$canisend-application`; Claude receives
`/canisend-application`. The orchestration skill reads the current context first, routes only the relevant focused
skill, continues through safe reads and previews, and pauses at explicit consent, approval, decision, or blocker
boundaries. The Codex or Claude session remains the conversation authority and keeps its own search, MCP, skills,
plugins, connectors, approvals, and transcript.
CanISend remains the application-state authority. Every mutation must return through the versioned CLI or Agent v2
task loop. MCP inspection stays body-free; its guarded job-intake and task tools use explicit consent, preview,
single-use confirmation tokens, and application-facade commits. The host must never edit `.canisend`, SQLite, blobs,
or managed projections directly.

The managed skill set is:

- `canisend-application` — context-first orchestration and stage routing;
- `canisend-job-intake` — link/PDF/local intake, parse, and criteria;
- `canisend-application-materials` — evidence, matching, decision, and drafting; and
- `canisend-application-review` — review, package, reconciliation, render, and export.

Codex project skills live under `.agents/skills`; Claude project skills live under `.claude/skills`. A small
`canisend-skills.json` manifest records only CanISend-managed file digests. Re-running setup upgrades an unchanged
managed skill, is a no-op when current, and refuses to overwrite a user-modified skill.

The desktop **Built-in Skills** manager shows all four skills for the selected host, their managed-file counts,
version-matched state, discovery directory, and ownership manifest. It distinguishes not installed, current,
update available, incomplete, user-modified, and unmanaged states. Install/update repairs only unchanged
manifest-owned files. Removal performs a complete preflight and deletes only files whose SHA-256 still matches the
manifest; user-modified and unmanaged files are preserved and block the operation.

The equivalent CLI operations use the same application facade:

```console
canisend --workspace ./applications agent assets status --host codex --json
canisend --workspace ./applications agent assets install --host codex --json
canisend --workspace ./applications agent assets uninstall --host codex --json
```

The handoff contains managed workflow instructions, canonical paths, public IDs, commands, and body-free status only.
It does not contain advert, profile, evidence, draft, or review bodies, and preparing it does not contact a provider.

### Return to the connected App workflow

The desktop keeps one global workspace and application context across Opportunities, Applications, Profile,
Application workspace, Agent integration, and Documents & delivery. Its stage rail recommends the first unmet
durable requirement rather than treating those screens as unrelated tabs. Agent-context next actions and committed
task results deep-link back to the exact source, criteria, evidence, match, plan, document, review, package, or render
surface.

The App remembers only the active screen/detail, canonical workspace path, selected public job ID, and the most
recent body-free action summary. This convenience state is not an application transcript and is never authoritative:
the Rust workspace, revisions, task leases, and receipts remain the source of truth. Changing workspaces hides a
recent action that belongs to a different workspace.

## Optional: use the in-App read-only bridge

For quick inspection, the desktop can discover an installed `codex` or `claude` executable and run a consent-gated,
read-only turn. This is a convenience bridge, not CanISend's primary agent surface:

- Discovery proves only the executable path and bounded `--version` output. It does not inspect or
  claim successful sign-in, provider access, MCP, skills, plugins, search, or other host
  configuration.
- Codex uses JSONL `exec` events and resumes the recorded thread ID.
- Claude uses print-mode JSON and resumes the recorded session ID.
- The selected workspace is the process working directory.
- Search, MCP servers, skills, and plugins are available only when that CLI runtime exposes them through its normal
  local configuration.
- A connector that exists only in a separate desktop host is not automatically inherited.
- The bridge can inspect and advise, but it cannot mutate CanISend state.

CanISend stores a body-free binding for each `(workspace, runtime, optional job)` scope. On macOS the registry is
`~/Library/Application Support/CanISend/agent-sessions.json`. It contains the canonical workspace path, runtime,
optional job ID, external session ID, and timestamps. It does not contain prompts, responses, transcripts, tokens, or
credentials. Codex or Claude remains the authority for its own local transcript. Switching tabs preserves the
in-memory App view; closing the App discards that rendered view, while the external session binding permits the next
turn to resume the host-owned conversation.

Starting a new conversation replaces only the binding for the selected scope after the new host session succeeds. A
workspace-level conversation and each job-level conversation therefore remain independent.

An in-App turn can be cancelled while its local CLI process is running. Cancellation is scoped to the exact
workspace, runtime, and optional job, terminates only that process, and does not parse or save a partial response. A
cancelled new conversation does not replace an existing session binding. This control is for the optional bridge;
conversation and task controls in the external host remain owned by that host.

### Privacy and execution boundary

Every turn requires explicit provider-send consent. The request is passed to the local runtime through standard input,
not a shell or command-line argument. Executable discovery is limited to fixed command names and known user/system
locations; there is no arbitrary executable field. Runtime output, diagnostics, duration, and concurrent turns are
bounded. Imported job adverts and profile sources are identified as untrusted data in the bridge prompt.

The local runtime may send necessary context to the provider configured in that runtime. CanISend does not gain access
to the provider credential and does not implement a provider HTTP client in this mode.

### Current and target interaction modes

| Mode | Session continuity | Host tools | CanISend writes |
| --- | --- | --- | --- |
| External-host handoff (recommended) | Managed entirely by Codex or Claude | Full capability of that host surface | Only through versioned CanISend operations |
| Local bridge (optional) | Stored Codex thread/Claude session ID plus host transcript | Whatever the selected CLI exposes | None; read-only |
| CanISend MCP adapter | Managed entirely by the external host | Thirteen portable, versioned tools | Four guarded writes; two require a matching preview token |
| Direct model API (not implemented) | CanISend would have to own history and truncation | No automatic access to consumer-app plugins | Would require a separate credential and consent design |

### Connect the guarded MCP adapter

The desktop Agent screen generates a host-specific registration command and configuration snippet for the selected
workspace. The adapter runs inside the same version-matched `canisend` binary:

```console
canisend --workspace /absolute/path/to/workspace mcp serve
```

Codex project configuration uses `.codex/config.toml`:

```toml
[mcp_servers.canisend]
command = "/absolute/path/to/canisend"
args = ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
enabled = true
default_tools_approval_mode = "writes"
```

Claude project configuration uses `.mcp.json`:

```json
{
  "mcpServers": {
    "canisend": {
      "type": "stdio",
      "command": "/absolute/path/to/canisend",
      "args": ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
    }
  }
}
```

The adapter negotiates MCP `2025-11-25` over stdio and exposes, in deterministic order:

- `canisend_capabilities`
- `canisend_context`
- `canisend_job_detail`
- `canisend_job_intake_commit`
- `canisend_job_intake_preview`
- `canisend_jobs_list`
- `canisend_profile_sources`
- `canisend_task_completion_commit`
- `canisend_task_completion_preview`
- `canisend_task_inputs`
- `canisend_task_latest`
- `canisend_task_prepare`
- `canisend_workflow_status`

Seven routine inspection tools are read-only and idempotent. Job-intake and task-completion preview tools are
read-only but intentionally non-idempotent because each creates an in-memory single-use token; URL intake is the only
open-world tool. The four write tools are explicitly declared non-read-only and non-destructive so the host can apply
its write-approval policy:

- `canisend_job_intake_commit` consumes an exact source preview;
- `canisend_task_prepare` freezes current revisions into a task lease;
- `canisend_task_inputs` exports only declared inputs after explicit consent; and
- `canisend_task_completion_commit` consumes an exact validated completion preview.

The desktop configuration card reports these categories separately as nine read-only/preview
tools and four approval-gated writes. The complete thirteen-tool list remains deterministic and
versioned; the display categories are generated from the same Rust application contract used by
the MCP protocol test.

Inputs and serialized outputs are bounded, application failures preserve the typed CanISend error classification,
and routine job/profile responses exclude source bodies. Preview tokens are scoped to one MCP process, capped,
single-use, and restored only when an application commit fails. The adapter calls the same Rust application facade as
CLI and GUI; it contains no duplicate business rules.

#### Give the agent a job link or PDF

Select or create the target job, then tell the host which URL or absolute local file you want to add. The host calls
`canisend_job_intake_preview` with the matching explicit consent. CanISend reads/fetches the source once and returns:

- target job and expected revision;
- source kind, requested/final locator, redirects, content type, size, page/line counts, and SHA-256;
- duplicate-content and semantic-review notices; and
- the exact intended source attachment and job-revision change.

No source body is returned in the preview. After reviewing it, approve
`canisend_job_intake_commit` in the host. Commit uses the bytes already held by the preview and fails if the target job
revision changed. The desktop Applications screen follows the same preview/confirm sequence.

A richer [Codex App Server](https://developers.openai.com/codex/app-server/) or Claude stream client remains an
optional later convenience and is not a prerequisite for the product workflow.

## Export a self-contained host pack

Install or safely update discoverable skills directly in an existing workspace:

```console
canisend --workspace ./applications agent assets install --host codex
canisend --workspace ./applications agent assets install --host claude
```

Use export when a standalone copy is needed:

```console
canisend agent assets export --host codex --destination ./canisend-codex-pack
canisend agent assets export --host claude --destination ./canisend-claude-pack
canisend agent assets export --host generic --destination ./canisend-generic-pack
```

Each pack includes host instructions, the four discoverable workflow skills, operation prompts, public schemas,
examples, and an integrity manifest. Codex packs contain 39 files; Claude and generic packs contain 35 files. The
pack is versioned for `canisend.agent/v2` and does not depend on source-repository files after export. Give the
selected pack to the host according to that platform's local instruction mechanism.

## Discover current state

An agent should never infer capabilities from prose alone:

```console
canisend agent capabilities --json
canisend --workspace ./applications application list --json
canisend --workspace ./applications application show --job JOB_ID --json
canisend --workspace ./applications content list --job JOB_ID --json
canisend --workspace ./applications content search QUERY --job JOB_ID --json
canisend --workspace ./applications agent context --job JOB_ID --json
canisend --workspace ./applications workflow status --job JOB_ID --json
```

Treat only `available` capabilities as executable. The application dossier is the shared, body-free read model used
by the CLI, desktop, and selected-job Agent context. It projects discovery origin, location, deadline, source count,
workflow progress, the relevant current blocker, and exact next actions without copying or disclosing imported
bodies. Context adds host execution guidance around that same authoritative state.

The content catalog is the related body-free artifact map used by the desktop Content Library. Metadata search can
help a host locate a source, confirmed decision, material, or delivery output without exporting a body. Private
full-text search is not an MCP tool and is never implied by Agent Context; invoke it only after the user explicitly
approves `--include-private-bodies --allow-private-read`. Its bounded index is rebuilt in memory and discarded after
the command.

## Bounded task loop

1. Prepare a task with the operation named by workflow status.
2. Inspect the returned descriptor and consent requests.
3. After user approval, export only declared inputs.
4. Ask the host to produce JSON matching the descriptor's output schema and embedded prompt.
5. Complete the task with the same task ID, lease ID, expected job revision, and input revisions.
6. Follow structured validation remediation or prepare a new task if stale.

```console
canisend --workspace ./applications task prepare \
  --job JOB_ID --operation job-parse --mode host-agent --json

canisend --workspace ./applications task inputs TASK_ID \
  --destination ./agent-work/TASK_ID \
  --allow-private-read --json

canisend --workspace ./applications task complete \
  --file ./agent-work/TASK_ID/completion.json --json
```

Candidate validation is schema-first and semantic. An invalid candidate leaves the lease prepared and returns stable
violation codes plus JSON pointers. Replaying the identical accepted candidate is idempotent. If a source/profile
revision or lease changes, completion returns `task.stale`; prepare again and do not reuse the old candidate.

## User-only decisions

An agent may propose evidence, matches, drafts, and semantic review findings. The user remains responsible for:

- confirming or correcting job criteria;
- confirming, excluding, or revising profile evidence;
- choosing apply, hold, or skip and confirming the plan;
- resolving required placeholders and review dispositions;
- consenting to private read/provider send/local export;
- checking the final package and submitting outside CanISend.

No command, capability, or host pack authorizes application submission.

## Protocol behavior

Use `--json` or capture stdout to receive exactly one versioned envelope. Diagnostics never share stdout with the
JSON object. Exit classes are stable: 0 success, 2 CLI usage, 3 validation/consent, 4 state conflict/stale, 5 external
I/O/provider failure, and 6 internal invariant failure. See [agent protocol v2](../contracts/agent-protocol-v2.md)
for the complete fields and error registry.
