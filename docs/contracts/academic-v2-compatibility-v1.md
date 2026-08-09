# Academic Agent v2 and `job` CLI compatibility v1

**Status:** bounded compatibility contract

**Deprecated surfaces:** `canisend.agent/v2`, MCP v2 tools, and `canisend job ...`

**Eligible Pack:** exact built-in `org.canisend.academic-job@1.0.0` snapshot

## Purpose

The compatibility adapter preserves deterministic academic workflows while the canonical neutral
Agent v3 and Application/Opportunity commands are completed. It is not a second generic API and
does not make `Job`, fixed academic stages, or fixed document kinds part of the framework kernel.

Admission compares Pack ID, semantic version, and content digest with the verified embedded Pack.
Matching only `org.canisend.academic-job` is insufficient. The adapter never chooses a Pack, maps
by label, selects a latest version, or interprets an unlinked Application.

## Authority modes

| Workspace state | Read compatibility | Legacy write compatibility |
|---|---|---|
| Workspace v2 | Allowed as `workspace-v2-implicit-academic` | Allowed; existing v2 invariants remain authoritative |
| Workspace v3 with exact migrated academic binding | Allowed as `workspace-v3-academic-read-only` | Rejected before mutation |
| Workspace v3 with another Pack, digest mismatch, or missing legacy mapping | Rejected before mutation | Rejected before mutation |

Workspace v3 legacy writes are deliberately disabled. Current v2 services update legacy tables but
cannot advance the neutral v3 Application snapshot atomically; allowing them would create mixed
authority. The failure names the canonical v3 operation that must eventually own the write.
External task-input export is treated as a scoped read because it does not modify Workspace
authority, while preparation and completion previews are write-intent operations.

## Machine-readable response

Successful compatibility receipts and CLI Agent responses contain `compatibility`:

```json
{
  "surface": "job-cli",
  "deprecated": true,
  "legacy_operation": "job.show",
  "canonical_v3_operation": "application.show",
  "authority": "workspace-v3-academic-read-only",
  "pack": {
    "id": "org.canisend.academic-job",
    "version": "1.0.0",
    "content_digest": "3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07"
  }
}
```

Existing `data` remains unchanged and deterministic. CLI `job` responses use `job-cli`; Agent v2
and MCP receipts use `agent-v2`.

Failure uses stable code `compatibility.unavailable`, exit class `conflict`, a body-free `details`
object, and `remediation.action` equal to the canonical v3 operation. Details identify the reason,
detected Pack bindings, and `workspace_mutated: false`.

## Frozen operation map

| Legacy operation | Canonical v3 operation |
|---|---|
| `agent.capabilities` | `agent-v3.capabilities` |
| `agent.context` | `agent-v3.context` |
| `job.list` | `application.list` |
| `job.show` | `application.show` |
| `job.create` | `application.create` |
| `job.archive` | `application.archive` |
| `job.import`, `job.intake.commit` | `opportunity.intake.commit` |
| `job.intake.preview` | `opportunity.intake.preview` |
| `profile.source.list` | `profile-source.list` |
| `task.show` | `agent-v3.task.show` |
| `task.latest` | `agent-v3.task.latest` |
| `task.prepare` | `agent-v3.task.prepare` |
| `task.inputs` | `agent-v3.task.inputs.export` |
| `task.complete.preview` | `agent-v3.task.completion.preview` |
| `task.complete` | `agent-v3.task.completion.commit` |
| `task.cancel` | `agent-v3.task.cancel` |
| `task.prepare-again` | `agent-v3.task.prepare-again` |
| `workflow.status` | `application.workflow.status` |

These target identifiers are stable routing names, not a claim that every Agent v3 leaf operation
is already exposed. An operation absent from this table has no inferred compatibility mapping.

## Verification boundary

Tests prove registry completeness and unique legacy keys, exact binding comparison, generic-Pack
failure with canonical remediation, migrated-academic reads, pre-mutation refusal of migrated
legacy writes, unchanged Job authority, CLI golden response compatibility, and MCP receipt parity.
