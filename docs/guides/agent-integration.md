# Agent integration

CanISend owns validation, Pack identity, revisions, consent, storage, review, rendering, and audit
state. Codex, Claude, or another Agent host owns conversation and bounded semantic reasoning. The
host must never edit `.canisend`, SQLite, immutable blobs, or managed projections directly.

## Select the protocol from the Workspace Pack

Inspect the Workspace before exposing tools:

```console
canisend --workspace ./applications workspace status
```

| Exact Pack | Authority | Agent surface |
| --- | --- | --- |
| `org.canisend.generic-application` | Workspace v3 | canonical Agent v3, nine MCP tools |
| `org.canisend.academic-job` | Workspace v2 compatibility, or migrated v3 with preserved academic authority | bounded Agent v2, thirteen MCP tools |

Generic Agent v3 operations fail closed on the Academic Pack. Agent v2 academic aliases fail
closed on the Generic Pack. Do not route by filenames, labels, or guessed content; use the exact
Pack ID shown by Workspace status and the exact binding returned by Agent capabilities.

## Recommended external-host handoff

The desktop Agent screen or CLI installs four version-matched, manifest-owned workflow skills in
the selected project:

```console
canisend --workspace ./applications agent assets status --host codex --json
canisend --workspace ./applications agent assets install --host codex --json
canisend --workspace ./applications agent assets install --host claude --json
```

The managed skills cover orchestration, intake, materials, and review. Installation updates only
unchanged CanISend-owned files and refuses to overwrite local edits. Removal performs a complete
digest preflight:

```console
canisend --workspace ./applications agent assets uninstall --host codex --json
```

The handoff contains public IDs, canonical paths, body-free state, commands, and blockers. It does
not contain source, Profile, Evidence, draft, review, token, or credential bodies. Codex or Claude
retains its own transcript, tools, search, plugins, connectors, provider policy, and approvals.

## Connect the MCP adapter

The version-matched binary serves MCP `2025-11-25` over stdio:

```console
canisend --workspace /absolute/path/to/workspace mcp serve
```

Codex project configuration:

```toml
[mcp_servers.canisend]
command = "/absolute/path/to/canisend"
args = ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
enabled = true
default_tools_approval_mode = "writes"
```

Claude project configuration:

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

Start every session by listing tools and reading capabilities/context. Never infer a mutation from
prose when the current context reports a blocker or a different `next_action`.

## Canonical Generic Agent v3 tools

The Generic Pack exposes nine deterministic operations:

- `canisend_agent_v3_capabilities`
- `canisend_agent_v3_context`
- `canisend_applications_list`
- `canisend_application_create`
- `canisend_application_plan`
- `canisend_application_compose`
- `canisend_application_review`
- `canisend_application_approve`
- `canisend_application_export`

Capabilities, context, list, and review are read-only. Review requires explicit private-read
consent. Create, Plan, compose, approve, and export are host-approval-gated writes; export also
requires private-export consent. Every mutation is exact-Pack and expected-revision bound.

Approval uses the shared App-owned broker. A successful review creates a cryptographically random,
single-use token bound to the canonical Workspace, exact Pack digest, Application, operation,
reviewed snapshot, and expected revision. It has a ten-minute monotonic lifetime and exists only in
the current process. Expiry, replay, wrong context, changed revision, or capacity exhaustion causes
no mutation. Only a classified transient commit failure may restore the same reservation for an
explicit retry; restarting MCP requires a new review.

The generic lifecycle is create → confirm Requirements and Plan → compose Deliverables → privately
review → approve → privately export. Export renders locally and returns
`submission_performed: false`.

## Academic Agent v2 compatibility tools

The Academic Pack exposes thirteen deterministic operations:

- `canisend_capabilities`
- `canisend_context`
- `canisend_job_detail`
- `canisend_job_intake_preview`
- `canisend_job_intake_commit`
- `canisend_jobs_list`
- `canisend_profile_sources`
- `canisend_task_completion_preview`
- `canisend_task_completion_commit`
- `canisend_task_inputs`
- `canisend_task_latest`
- `canisend_task_prepare`
- `canisend_workflow_status`

Routine inspection is body-free. Intake preview holds the reviewed local/URL/PDF bytes in the MCP
process and returns metadata, digest, duplicate notice, target revision, and intended mutation—not
the source body. Intake commit consumes that exact single-use preview. Task preparation freezes
revisions into a lease; `task inputs` exports only declared private inputs after consent; completion
preview validates candidate JSON; completion commit consumes the matching preview.

These names remain compatibility aliases for the academic journey. They do not define the Generic
Pack ontology and must not become fallback writes when Agent v3 rejects a Pack mismatch.

## CLI task loop for the Academic Pack

```console
canisend --workspace ./academic-applications task prepare \
  --job JOB_ID --operation job-parse --mode host-agent --json
canisend --workspace ./academic-applications task inputs TASK_ID \
  --destination ./agent-work/TASK_ID --allow-private-read --json
canisend --workspace ./academic-applications task complete \
  --file ./agent-work/TASK_ID/completion.json --json
```

Candidate validation is schema-first and semantic. Invalid output leaves the lease prepared and
returns stable violation codes and JSON pointers. An identical accepted replay is idempotent. If a
source, Profile, input, or lease revision changes, prepare again and do not reuse the old bundle.

## Optional in-App bridge

The desktop may discover an installed `codex` or `claude` executable and run a consent-gated
read-only turn. It uses standard input, bounded output, and a fixed executable-discovery policy.
CanISend stores only a body-free binding for `(Workspace, runtime, optional Application)`; it does
not store prompts, responses, transcripts, tokens, or credentials. The selected host remains the
transcript and provider authority. This bridge cannot mutate CanISend state and does not inherit
tools that exist only in another desktop product.

## User-only boundaries

An Agent may propose Requirements, Evidence relationships, Plans, Deliverables, and review
findings. The user remains responsible for confirming sources and Evidence, making the proceed/hold
decision, approving private/provider/export scopes, resolving unsupported claims, reviewing final
artifacts, and submitting outside CanISend. No tool, host pack, approval token, or capability
authorizes login, upload, portal automation, or submission.

JSON commands return one versioned envelope on stdout. Preserve the stable operation, status,
error code, retryable flag, expected revision, and remediation when diagnosing failures. See
[Privacy and consent](privacy-and-consent.md) and the
[Agent v3/MCP contract](../contracts/agent-v3-mcp.md) or
[Agent protocol v2 compatibility contract](../contracts/agent-protocol-v2.md), as selected by the
Workspace Pack.
