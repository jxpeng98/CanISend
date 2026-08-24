# Agent integration

CanISend owns validation, Pack identity, revisions, consent, storage, review, rendering, recovery,
and audit state. Codex, Claude Code, Claude Desktop, or another Agent host owns conversation and
bounded semantic reasoning. A host must never edit `.canisend`, SQLite, immutable Blobs, or managed
projections.

## Clean v4 boundary

Alpha.7 and later host resources require `canisend.workspace/v4` and `canisend.agent/v4`. One
Workspace can contain academic, generic, or other Pack-bound Applications together. Select one
exact Application and preserve its Pack ID, Pack version, Pack digest, revision, and snapshot
digest; never select a Workspace mode.

Earlier Skills, protocol requests, command aliases, and host-resource layouts are unsupported.
CanISend detects those resources before installation and returns clean-install guidance without
modifying either the old files or authoritative Workspace state.

## Install the v4 host resources

The desktop initialization and Agent setup journey installs four version-matched Skills from one
embedded, integrity-checked source:

| Skill | Canonical Agent v4 tasks |
| --- | --- |
| `canisend-workspace` | orientation, Profile/Evidence, Application creation, recovery |
| `canisend-intake` | Source intake and Requirements |
| `canisend-materials` | fit/Plan and Deliverable drafting |
| `canisend-review-export` | review and local export |

Codex receives the Skills under `.agents/skills` with generated `agents/openai.yaml` metadata.
Claude Code receives the same `SKILL.md` bytes under `.claude/skills`. The host-specific manifest
is `.agents/canisend-agent-v4.json` or `.claude/canisend-agent-v4.json`; it binds the product,
protocol, Workspace format, task-model digest, every file path, and every file digest.

Install and update are idempotent within v4. CanISend replaces only bytes recorded by the current
manifest, refuses user-modified or unmanaged paths, and performs a complete digest preflight before
uninstalling. Host setup never writes inside `.canisend`.

When a user selects Codex or Claude in the App's **Create workspace** dialog, CanISend can safely
create the complete project-local setup because the destination is required to be new or empty.
It writes `.codex/config.toml` or `.mcp.json` with the exact version-matched desktop executable and
installs the corresponding Skills in the same rollback boundary as Workspace registration. It
uses create-new semantics and never overwrites or merges a host file. Selecting no host creates an
App-only Workspace; host resources can be installed later.

The standalone CLI provides the same setup path without opening the App. `setup` installs the
managed Skills and returns the exact MCP registration command and configuration snippet. It does
not merge host configuration automatically:

```console
canisend --workspace /absolute/path/to/workspace host setup --host codex --json
canisend --workspace /absolute/path/to/workspace host setup --host claude --json
canisend --workspace /absolute/path/to/workspace host status --host codex --json
```

These commands default to `--scope project`, which installs under the Workspace. Use the same
explicit `--scope global` on setup, status, and removal to manage the current user's installation:

```console
canisend --workspace /absolute/path/to/workspace host setup --host codex --scope global --json
canisend --workspace /absolute/path/to/workspace host status --host codex --scope global --json
canisend --workspace /absolute/path/to/workspace host remove --host codex --scope global --json
```

By default the MCP guidance uses the currently running CanISend executable. A packaged or renamed
binary can be selected explicitly with `--executable /absolute/path/to/canisend`. To remove only
unchanged, manifest-owned Skills while preserving the host's MCP entry:

```console
canisend --workspace /absolute/path/to/workspace host remove --host codex --json
```

## Connect the MCP adapter

The native binary serves MCP `2025-11-25` over stdio and does not require the App to be open:

```console
canisend --workspace /absolute/path/to/workspace mcp serve
```

Apply the `registration_command` returned by `host setup` in a user-reviewed terminal, or merge its
`configuration_snippet` into the reported `configuration_target`. Then run the returned
`verification_command`. This explicit boundary prevents CanISend from overwriting unrelated host
servers or user policy. This manual merge rule applies to standalone CLI setup and existing
Workspaces; only the App's atomic new-or-empty Workspace bootstrap creates a project file directly.

Codex project configuration:

```toml
[mcp_servers.canisend]
command = "/absolute/path/to/canisend"
args = ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
enabled = true
default_tools_approval_mode = "writes"
```

Claude Code project configuration:

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

Claude Desktop chat uses the same `mcpServers.canisend` JSON entry, but reads it from the
user-level `claude_desktop_config.json` rather than the project's `.mcp.json`. Merge only that
entry through **Settings > Developer > Edit Config**, then choose **Developer > Reload MCP
Configuration**. CanISend does not rewrite this user-global file or unrelated Desktop servers.
Claude Desktop is an MCP client; the project-local `.claude/skills` resources remain the Claude
Code workflow surface.

List tools at the start of every session. The current clean-v4 MCP adapter exposes these body-free
read operations:

- `canisend_workspace_status`;
- `canisend_workspace_check`;
- `canisend_application_list`;
- `canisend_application_show`;
- `canisend_requirement_list`;
- `canisend_requirement_show`;
- `canisend_plan_show`;
- `canisend_deliverable_list`;
- `canisend_deliverable_show`;
- `canisend_export_list`;
- `canisend_export_show`;
- `canisend_profile_source_list`;
- `canisend_profile_association_list`; and
- `canisend_evidence_association_list`.

Requirement, Plan, and Deliverable responses include the exact Application, Pack, revision, and
snapshot digest. Deliverable list/show return metadata and content references only; reading a
Deliverable body remains a separate consented audit or review operation.

Private Deliverable bodies are available only through consented `canisend_deliverable_audit` or
`canisend_review_inspect` calls.

It exposes ten guarded mutation pairs:

- `canisend_profile_association_preview` → `canisend_profile_association_commit`; and
- `canisend_evidence_association_preview` → `canisend_evidence_association_commit`;
- `canisend_requirement_extract_preview` → `canisend_requirement_extract_commit`;
- `canisend_requirement_confirm_preview` → `canisend_requirement_confirm_commit`;
- `canisend_plan_propose_preview` → `canisend_plan_propose_commit`;
- `canisend_plan_confirm_preview` → `canisend_plan_confirm_commit`;
- `canisend_deliverable_draft_preview` → `canisend_deliverable_draft_commit`; and
- `canisend_deliverable_revise_preview` → `canisend_deliverable_revise_commit`;
- `canisend_review_disposition_preview` → `canisend_review_disposition_commit`; and
- `canisend_export_prepare_preview` → `canisend_export_prepare_commit`.

Requirement extraction accepts only Pack-qualified candidates whose statements equal exact UTF-8
spans in one current Source revision already associated with the selected Application. It appends
new proposals, rejects duplicate spans, and never deletes persisted Requirements. Local-file and
text-PDF Sources require explicit private-read consent for both preview and commit.

Profile Source bodies remain in local Workspace authority. A user can import a reviewed source
without the App through `canisend profile-source import`; `private-local` input requires the
explicit `--confirm-private-read` flag. Both CLI listing and the MCP tool return IDs, revisions,
digests, kinds, and privacy metadata without returning original or normalized body text.
The two association-list tools require one exact Application ID and distinguish Workspace
candidates from explicit links. They do not imply consent or create an association. A preview
validates the exact current resource revision and returns a CSPRNG `apv1_` token, preview digest,
expiry, and private-read requirement without mutating Workspace authority. Commit requires that
same Application, Pack, revision, digest, token, explicit `approved: true`, and any required private
read consent. Denial, wrong context, malformed binding, expiry, and successful commit consume the
token; replay fails without mutation. Only explicitly classified transient I/O or database failures
restore the same still-valid token.

Direct CLI preview/commit for these guarded operations is intentionally absent: independent CLI
processes cannot share the in-memory single-use Broker safely. Headless Agent writes use one
running `mcp serve` session; CLI retains the equivalent body-free list/read operations until a
separately designed durable approval authority exists.

Do not infer that an operation from the canonical registry is callable when it is absent from the
runtime tool list. Use a native CLI operation only when it exposes the same operation ID, or stop
with the missing capability as a blocker.

## Canonical task sequence

Orientation is read-only and follows `orient -> verify`. Every mutation follows:

```text
orient -> propose -> preview -> approve -> commit -> verify
```

The proposal and preview bind the exact Workspace, Application, Pack, revision, snapshot, schema,
operation, candidate digest, required consents, and expiry. Approval belongs to the user and binds
the exact preview digest. Commit consumes one opaque, process-bounded token. Expiry, replay, stale
revision, wrong Pack, denied consent, or host restart requires a fresh orientation and preview.

The canonical task model, operation registry, seven schemas, two examples, host guide, and Skills
ship in the Agent v4 export pack. The Codex pack has 20 files; Claude and generic-host packs have
16 because OpenAI UI metadata is Codex-specific.

## User-only boundaries

An Agent may propose Requirements, Evidence relationships, Plans, Deliverables, and review
findings. The user confirms sources and Evidence, chooses whether to proceed, grants private,
provider, network, and export consent, approves exact previews, reviews final artifacts, and
submits outside CanISend. No tool, Skill, receipt, export, or readiness state authorizes login,
upload, portal automation, or submission. Every export receipt confirms that
`submission_performed` is `false`.

See [Agent v4](../contracts/agent-v4.md) and
[Privacy and consent](privacy-and-consent.md).
