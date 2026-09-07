---
name: canisend-workspace
description: Operate the domain-neutral CanISend Workspace v4 boundary. Use when initializing or orienting a Workspace, managing basic Profile and confirmed Evidence, creating an Application with an exact Pack, checking health, upgrading Host Skills, or recovering from stale or damaged state.
---

# CanISend Workspace

Use CanISend as the only authority for durable state. This skill covers Agent v4 tasks
`orientation`, `profile-evidence`, `application-create`, and `recovery`.

## Initialize and reconnect

For a new Workspace, use `canisend --workspace PATH workspace init --host codex --json`
(or `--host claude`). Omit `--host` to initialize without installing Skills. Project Skills live
under `.agents/skills` for Codex or `.claude/skills` for Claude Code; `--scope global` selects
the user home instead. Open the Workspace in the Host to discover project Skills.

Setup returns an MCP registration command; it does not register or verify the connection.
Follow the user's selected Host and scope. Routine initialization, resource installation and
body-free checks use their CLI contracts and need no invented business confirmation form.
Reuse choices and authorization already supplied; ask only for missing outcome-changing decisions.

After replacing the executable, run `version --json`, `doctor --json`, and
`canisend --workspace PATH host status --host HOST --scope SCOPE --json` with the new binary.
For `update-available` or `incomplete`, run `host setup` with the same Workspace, Host and scope.
For `user-modified` or `unmanaged`, preserve the files and report the conflict; never force-update,
edit the ownership manifest, or silently remove customizations. Use the installed path and scope,
not a second copy in another discovery directory. Do not treat an old CLI as a Skills rollback tool.
After updating, reconnect the Host, rediscover tools, discard old previews and refresh Application
context. If the executable path changed, use the newly returned registration command. Resource
status `ready` alone does not establish a working MCP connection.

## Establish context

1. Require `canisend.workspace/v4` and `canisend.agent/v4`. Stop before mutation when either
   identifier differs.
2. Prefer the CanISend MCP adapter. Use the native `canisend` CLI only when it exposes the same
   operation ID; the desktop App does not need to be open.
3. Inspect Workspace status and health, then list Applications. Routine orientation must remain
   body-free.
4. If an Application is selected, preserve its UUID, Pack ID, Pack version, Pack digest, revision,
   and snapshot digest exactly. A Workspace can contain different Packs at the same time; never
   infer a Workspace mode. Read `canisend_application_pack_show` for the complete verified Pack
   catalog, including every Deliverable kind and its minimum/maximum count.

## Perform the bounded task

- For basic Profile or reusable Evidence, show current metadata, propose only source-grounded
  changes, and let the user correct and confirm them. Do not make private data available to an
  Application without an explicit typed association. Use guarded `canisend_evidence_confirm_preview`
  and `canisend_evidence_confirm_commit` to confirm Evidence grounded in exact Workspace Profile Sources
  before proposing its separate Application association.
- To create an Application, present the available Packs and require one exact selection. Preview
  the title, Pack identity, and initial associations before asking for approval.
- For recovery, inspect first. Explain stale revisions, denied consent, missing runtime, malformed
  output, restart, or integrity findings. Use only a CanISend backup, restore, or repair operation;
  never edit internal files.

Guarded business mutations follow orient, propose, preview, request native confirmation, user
approval, commit, and verify; use each operation's actual schema for other CLI operations. Keep token, digest, and expiry from the same actual successful preview;
use its opaque token once. On expiry, replay, restart, Pack mismatch, or stale revision, discard
it and orient again. After denial, stop that operation; do not retry through another tool or CLI.
Continue independently authorized inspection or diagnosis. A fresh preview is not renewed user
approval, and a task authorization is not a substitute for a required native form.

MCP `request_confirmation: true` requests a native form, not an assertion of prior approval.
Likewise, `request_private_read` and `request_private_export` request the respective consent form.
Only the user's accepted form with `confirm: true` grants authorization; never answer it for them.
`false` cancels or declines to request the form. Legacy `approved`, `confirmed_private_read`, and
`confirmed_private_export` fields are rejected. After an upgrade, restart/reconnect the MCP Host,
discover tool schemas again, and discard old previews.

## Finish

Verify the returned revision, snapshot digest, and available audit/artifact fields. Report absent
receipt fields as absent; never invent an ID or stop solely because an optional field is missing.
Continue the authorized workflow until complete or blocked by a specific unmet requirement. Never inspect or edit `.canisend`, SQLite, immutable Blobs, or
managed projections directly. Never interpret readiness or export as permission to upload or
submit anything.
