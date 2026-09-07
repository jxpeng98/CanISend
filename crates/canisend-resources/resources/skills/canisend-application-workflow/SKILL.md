---
name: canisend-application-workflow
description: Prepare or resume a complete CanISend application, from understanding an opportunity and assembling evidence to planning, drafting, reviewing and delivering local materials. Use for end-to-end application help, progress assessment, or coordinating multiple materials; route isolated edits to the relevant stage Skill.
---

# CanISend Application Workflow

Own continuity across the application journey; the stage Skills own their operations. This is
an orchestration entrypoint over the existing Agent v4 tasks, not a new task kind or state store.
CanISend prepares evidence-bound applications; it does not submit to employers, funders or portals.

## Start from the user's outcome

Identify the opportunity, selected Workspace/Application, desired materials, language and any
source-backed deadline or formatting constraints. Reuse information already supplied. Ask only
for missing facts or choices that affect the result; do not ask the user to choose every next tool.
For an isolated stage request, perform that stage rather than forcing a full restart.

Use `canisend-workspace` to initialize/connect if needed, require `canisend.workspace/v4` and
`canisend.agent/v4`, select the exact Pack, and read canonical Application state. Inspect the
complete verified Pack catalog and dependencies. Pack stages guide business ordering; current
CLI/MCP schemas determine available operations. A declared capability does not prove an adapter
is callable. Never use an old prompt's job IDs or v2 candidate shape in a v4 request.

## Carry the journey through

| Stage and owner | Needed input | Useful result and transition |
|---|---|---|
| Opportunity brief — `canisend-intake` | User-selected advert/call and supporting documents | Source-backed purpose, audience, eligibility, criteria, deliverables, deadlines and ambiguities; create/select the Application through `canisend-workspace` when needed |
| Profile and evidence — `canisend-workspace` | Relevant CV/profile/records supplied with required read consent | Confirmed, source-traceable Evidence; explicit associations to this Application; a missing-fact list |
| Requirements — `canisend-intake` | Imported, associated Sources | Confirmed Pack-qualified Requirements; separate mandatory, preferred and unclear requirements |
| Fit and plan — `canisend-materials` | Confirmed Requirements, associated Evidence and Pack catalog | Requirement-to-Evidence assessment, gaps and prohibited claims, user's proceed/hold decision, confirmed material plan |
| Draft and revise — `canisend-materials` | Confirmed Plan, supported facts and user feedback | Purposeful, audience-specific structured Deliverables; factual claims trace to exact Evidence; gaps remain explicit |
| Review — `canisend-review-export` | Current drafts, source constraints and audit findings | Corrected materials, cross-document consistency and current review dispositions; route substantive corrections back to their owning stage |
| Local delivery — `canisend-review-export` | Current readiness and required export consent | Verified local files, format/path/digest inventory and unresolved limitations; no upload or submission claim |

If a stage is already valid for the current inputs, reuse it. Independent profile preparation may
proceed while opportunity details are clarified. A hold decision produces a concrete gap-resolution
plan; it is not permission to draft around mandatory deficiencies. Do not invent support merely to
move the workflow forward. A new advert, fact or document revision may invalidate downstream work;
refresh affected state rather than rerunning every unrelated stage.

## Continue across sessions and collaborators

Read Application, Plan, Deliverable, review and export state to locate the first unmet dependency.
If local candidate tasks exist, inspect their metadata before preparing another one. Use only the
supported local-task handoff for bounded draft work; workers exchange candidate references, not
approval tokens. The final reviewer refreshes the Application and exact candidate digest.

After each durable change, retain returned IDs/revisions and re-read changed context. Summarize
completed stages, current missing input or decision, and the next concrete action; do not create
another workflow database or invent completion flags. On restart, upgrade, stale context or lost
preview metadata, rediscover schemas as needed and obtain a fresh preview. Prior conversation
text cannot reconstruct a token or establish the current revision.

Use `request_confirmation: true` only to request the actual form for guarded writes; the user
answers it. Follow `canisend-workspace` for private-data and confirmation boundaries. After a
denial stop that operation; independent authorized work may continue. An export receipt proves
local preparation, not submission, application quality, eligibility or likelihood of acceptance.
