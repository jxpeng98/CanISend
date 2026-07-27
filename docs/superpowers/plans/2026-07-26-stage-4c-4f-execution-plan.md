# CanISend Stage 4C–4F execution plan

**Status:** Stage 4C–4E complete; Stage 4F active

**Decision date:** 2026-07-26

**Source baseline:** `1.0.0-alpha.2` after public checkpoint `v1.0.0-alpha.2`

**Entry parity:** `22 implemented`, `13 deferred-beta`

**Stage 4C exit parity:** `26 implemented`, `9 deferred-beta`

**Stage 4D exit parity:** `29 implemented`, `6 deferred-beta`

**Stage 4E exit parity:** `33 implemented`, `2 deferred-beta`

**Stage 4F entry parity:** `33 implemented`, `2 deferred-beta`

**Parent roadmap:** [CanISend 1.0 release roadmap](2026-07-25-1.0-release-roadmap.md)

## 1. Outcome

Complete the remaining ordinary CLI-to-GUI operation coverage in dependency order, without
duplicating store rules or turning the GUI into a shell:

| Stage | Operation families | Exit parity |
|---|---|---:|
| 4C — decision foundation | `profile.*`, `criteria.*`, `match.*`, `plan.*` | `26/35` |
| 4D — discovery and agent work | `discovery.*`, `task.*`, `agent.*` | `29/35` |
| 4E — documents and delivery | `document.*`, `review.*`, `package.*`, `render.*` | `33/35` |
| 4F — inspection and Beta freeze | `schema.*`, `resource.*` | `35/35` |

Beta transition remains prohibited until all 35 families are implemented, the intended Agent v2
and persisted contracts are frozen, public Alpha feedback is triaged, and the Beta readiness gate
passes. Completing an implementation stage does not create a tag or release.

## 2. Dependency order

The application workflow is upstream-to-downstream:

```text
profile sources
  -> confirmed evidence
  -> confirmed job criteria
  -> criterion/evidence matches
  -> confirmed application plan
  -> document tasks and accepted documents
  -> review dispositions
  -> package readiness
  -> render and export
```

Discovery can create jobs, while Agent v2 tasks can produce evidence, match, document, and review
candidates. They follow the decision foundation so their GUI task panels can render the same typed
profile and workflow state instead of inventing a second representation.

## 3. Shared architectural rules

- `canisend-store` remains authoritative for revisions, source spans, confirmation, invalidation,
  workflow transitions, readiness, and export integrity.
- `canisend-app` owns typed requests, read models, receipts, and error classification shared by
  CLI and GUI adapters.
- GUI pages emit typed worker requests; only the worker invokes `Application`.
- CLI adapters preserve current JSON, human output, error code, and exit-class contracts.
- The GUI never executes the equivalent CLI command it displays.
- Private source bodies do not appear in navigation, registry data, routine diagnostics, activity
  summaries, or error strings.
- Import, private review, provider send, URL fetch, and private export retain distinct explicit
  consent boundaries.
- External JSON editing remains available for agents and advanced users, but ordinary GUI users
  receive structured fields and validation rather than a raw-JSON requirement.
- Worker-backed mutations disable duplicate submission and expose an operation status when work
  may exceed 300 milliseconds.

## 4. Stage 4C — decision foundation

### 4.1 Product surface

Add a **Profile** page with:

- body-free source metadata, type, sensitivity, revision, and import time;
- local Markdown, text, or JSON source import;
- explicit `public` or `private-local` classification before reading;
- job-scoped evidence proposal/template/current state;
- structured evidence confirmation, correction, exclusion, and sensitivity controls; and
- clear stale-state guidance after the profile changes.

Extend the selected job's **Workflow** surface with:

- proposed and confirmed criteria, source-span identity, importance, and confidence;
- current criterion-to-evidence match strength and prohibited-claim guidance;
- application decision, positioning, priorities, risks, document requirements, and blockers; and
- explicit plan confirmation or revision.

The default surface is content-first and low decoration. Forms use field-level validation, visible
keyboard focus, clear success receipts, English and Simplified Chinese labels, and no color-only
state. Destructive or invalidating revisions state their downstream effect before confirmation.

### 4.2 C0 — tracking authority

- Create this execution plan and link it from the parent roadmap.
- Freeze entry parity at `22/35`.
- Record Stage 4C exit parity as `26/35`, without marking any family implemented early.
- Keep `1.0.0-alpha.2` as the source version until an explicit release-line decision.

### 4.3 C1 — Profile application facade

Add typed application operations for:

- local profile-source import with explicit private-read consent and sensitivity;
- source list and source metadata inspection;
- evidence proposal, editable template, and confirmed catalog reads; and
- evidence confirmation/revision through the existing store validator.

Required invariants:

- source list/show receipts contain metadata and artifact references, never source bodies;
- invalid UUIDv7 input is rejected before workspace access;
- malformed or unknown-field evidence candidates fail before mutation;
- confirmation returns the committed artifact reference;
- changing the profile retains the store-owned downstream invalidation behavior; and
- application errors retain the existing Agent v2 error classification.

### 4.4 C2 — Profile CLI adapter alignment

- Route `profile source add/list/show` and `profile evidence proposed/export/confirm/show` through
  `canisend-app`.
- Keep private JSON export create-new, regular-parent, `.canisend` exclusion, and `0600` Unix mode.
- Preserve committed CLI JSON snapshots, human summaries, next actions, and exit classes.
- Prove that no adapter response contains an imported source body.

### 4.5 C3 — Profile GUI source catalog

- Add the Profile navigation entry and page module.
- Load source metadata only after a workspace is selected.
- Add a native file picker restricted to Markdown, text, and JSON.
- Require the user to choose sensitivity and confirm private reading before dispatch.
- Show import success, profile revision change, and downstream refresh guidance.
- Restore focus to the source action after success or failure.

### 4.6 C4 — evidence review and confirmation

- Select a job and load proposal, editable template, or current confirmed catalog.
- Render evidence kind, summary, source quote, sensitivity, confirmed, and excluded fields.
- Preserve source-span and identity fields as read-only.
- Validate required summaries and mutually meaningful confirmation/exclusion state before
  dispatch, while leaving canonical validation to the store.
- Preview that a revision may stale matches, plan, documents, review, package, and render outputs.
- Refresh workflow controls after a successful confirmation.

### 4.7 C5 — criteria application and GUI

- Add proposed/template/current/confirm application operations.
- Route the criteria CLI family through the facade without output drift.
- Render structured requirement, importance, kind, confidence, and source identity.
- Keep source spans read-only and confirmation explicit.
- Refresh Match readiness after a successful confirmation.

### 4.8 C6 — match read model

- Add a current-match application read model and CLI adapter route.
- Show every criterion once, match strength, cited evidence identities, explanation, and
  prohibited claims.
- If Match is not complete, show the exact workflow or task action required; do not synthesize a
  match in the GUI.
- Treat match creation as a Stage 4D task lifecycle concern while completing the existing
  read-only `match.*` family in Stage 4C.

### 4.9 C7 — plan application and GUI

- Add plan template/current/confirm application operations.
- Route plan export/confirm/show through the facade without changing its external contract.
- Render decision, strategy, priorities, risks, derived blockers, and planned documents.
- Derived blockers and match identity remain read-only.
- Require explicit confirmation for a new decision or revision and refresh downstream workflow
  state after success.

### 4.10 C8 — Stage 4C closure

- [x] Mark only the four completed families `implemented`.
- [x] Record `26 implemented` and `9 deferred-beta`.
- [x] Add a repeatable worker-to-application persistence test covering profile import, evidence and
  criteria confirmation, current-match inspection, plan confirmation, and workspace reopen.
- [x] Run formatter, affected app/GUI/CLI/store tests, relevant all-target Clippy, release check, and
  hosted Fast CI.
- [x] Run the local packaged macOS accessibility smoke alongside the decision-workflow persistence
  test. Together they cover the native shell and the complete revision-bound decision path.
- [x] Keep native five-target packages release-only and run them only for an explicitly authorized
  checkpoint.

## 5. Stage 4D — discovery and agent work

### 5.1 Product surface

Add a **Discovery** page with:

- the compiled adapter catalog and bounded limits;
- CSV, JSON, or host-agent batch preview before an explicit commit;
- user-invoked public-source refresh with a separate network-consent boundary;
- source metadata, active/history filtering, freshness, and lead detail;
- bounded possible-duplicate suggestions that never merge automatically; and
- explicit lead promotion with the resulting job and safe next action.

Extend the selected job's **Workflow** surface with a task panel that shows:

- the exact operation, execution mode, lease expiry, declared input artifacts, output kind, and
  candidate schema;
- prepared, committed, cancelled, or stale status in text, never by color alone;
- separate private-read and configured-provider-send consent;
- a bounded input-export destination and the resulting manifest identity;
- completion-file validation and committed artifact identity;
- cancel and prepare-again recovery actions; and
- explicit loading, success, validation-error, stale, and recovery feedback.

Add an **Agent integration** surface for body-free Agent v2 capability/context inspection and
Codex, Claude, or generic asset-pack export. The destination must be new or empty. The GUI may show
the exact exported files and copy bounded commands, but it cannot run a general shell.

The UI remains content-first and uses the existing semantic theme, platform typography, English and
Simplified Chinese catalog, visible focus, reduced motion, 100–200% text scaling, and AccessKit
live-region feedback. Provider credentials remain outside workspace and registry storage.

### 5.2 D0 — tracking authority

- [x] Freeze Stage 4D entry parity at `26 implemented`, `9 deferred-beta`.
- [x] Record the dependency order and atomic work packages before changing parity.
- [x] Keep `1.0.0-alpha.2` as the source version until an explicit release-line decision.
- [x] Keep five-target package qualification release-only.

### 5.3 D1 — discovery application facade

- Add typed adapter, import-preview/commit, source, lead, suggestion, and promotion operations.
- Keep a previewed normalized report in memory so GUI confirmation commits the exact reviewed batch
  rather than rereading a changed file.
- Require explicit private-read consent for local CSV/JSON and explicit network consent for public
  refresh.
- Validate lead UUIDv7 values before workspace access.
- Preserve store-owned limits, freshness/history rules, idempotent promotion, and no-auto-merge
  behavior.
- Return typed receipts and safe next actions without exposing provider credentials.

### 5.4 D2 — discovery CLI adapter alignment

- Route all eight discovery commands through `canisend-app`.
- Preserve current JSON shapes, human summaries, error codes, exit classes, and dry-run behavior.
- Keep host-agent JSON explicit and reject host-agent CSV.
- Keep JSON source identity inside the versioned batch and CSV source provenance in CLI arguments.

### 5.5 D3 — discovery GUI

- Add Discovery navigation, source/lead lists, history filter, detail, and freshness labels.
- Preview CSV/JSON/host-agent diagnostics before commit and require explicit private-read consent.
- Preview a public adapter refresh before commit and require explicit user-invoked network consent.
- Show bounded duplicate suggestions without automatic merge.
- Confirm promotion, refresh Jobs after success, and expose the safe advert-import next action.

### 5.6 D4 — task application facade

- Add typed operation/mode requests for all eight compiled task operations.
- Add descriptor/state inspection, scoped input export, completion-file validation, cancel, and
  prepare-again recovery.
- Preserve exact lease, job revision, input revision/hash, output-kind, and schema validation.
- Keep private-read and provider-send consent independent.
- Treat validation failure as non-mutating, stale state as prepare-again, and cancellation as an
  audited terminal state.

### 5.7 D5 — task CLI adapter alignment

- Route task prepare/show/inputs/complete/cancel through `canisend-app`.
- Preserve current Agent v2 JSON and human contracts.
- Keep stdin completion CLI-only; GUI uses bounded regular JSON files.
- Preserve committed artifact and idempotent-replay reporting.

### 5.8 D6 — task GUI

- Add the selected-job task panel and operation/mode selector constrained by current workflow state.
- Show declared inputs and required consents before preparation or export.
- Export to a selected new/empty directory without revealing undeclared workspace bodies.
- Load one bounded completion JSON file, show field-level validation, and commit explicitly.
- Show cancel, stale, and prepare-again recovery actions with deterministic focus restoration.

### 5.9 D7 — Agent v2 application facade

- Add typed capability and body-free context read models shared by CLI and GUI.
- Add Codex, Claude, and generic asset-pack export through the embedded resource verifier.
- Preserve new/empty destination, safe path, manifest, and digest rules.

### 5.10 D8 — Agent v2 CLI adapter alignment

- Route capabilities, context, and asset-pack export through `canisend-app`.
- Preserve committed JSON snapshots and human output.
- Prove the context remains public metadata only.

### 5.11 D9 — Agent integration GUI

- Add capability/context inspection with a selected optional job.
- Show blockers and next actions as text with copy controls.
- Export one selected host pack to a new/empty directory after previewing the destination.
- Show the manifest and resource count after success; never execute the exported host.

### 5.12 D10 — Stage 4D closure

- [x] Mark only `discovery.*`, `task.*`, and `agent.*` implemented.
- [x] Record `29 implemented` and `6 deferred-beta`.
- [x] Add a repeatable discovery-to-task-to-agent persistence regression.
- [x] Run formatter, affected tests, relevant all-target Clippy, release check, hosted Fast CI, and a
  local packaged macOS bilingual/accessibility smoke.
- [x] Do not create a tag, release, package-manager update, or five-target native matrix.

Exit parity is `29/35`.

## 6. Stage 4E — documents and delivery

### 6.1 Product surface

Add selected-job **Workflow**, **Documents**, and **Review & export** views without adding a second
workspace or artifact representation.

The Documents view provides:

- an explicit private-read boundary before loading draft bodies;
- the accepted current document-set identity, plan binding, and exact member revisions;
- document title, kind, generation mode/task, sections, claims, citations, and placeholders;
- unresolved-placeholder and stale/missing guidance in text; and
- the Agent-task action that creates or replaces structured drafts rather than an untracked editor.

The Review & export view provides:

- deterministic and human-review findings with authority, severity, target, status, and resolution;
- read-only deterministic findings plus an explicit disposition and rationale for each selected
  human finding;
- package readiness, body-free reason codes, exact input revisions, and the permanent
  `submission_performed: false` boundary;
- a private projection export with exact destination preview and explicit consent;
- managed-projection reconciliation with separate inspect, replace, and preserve-copy-then-replace
  actions;
- trusted embedded rendering from authoritative structured artifacts rather than editable
  projections;
- PDF page, byte, warning, timing, artifact, and render-manifest inspection; and
- a separate private PDF export with exact file paths and explicit consent.

Every export destination is a safe relative path below `jobs/JOB_ID/`. Existing files and
directories are never overwritten. Edited projections are never replaced automatically, and a
preserved user copy must use a distinct unmanaged path. No control launches a shell, starts a
general host, or submits material to an application portal.

The UI retains the existing semantic theme and platform typography. It uses English and Simplified
Chinese labels, visible focus, 44-point minimum actions, reduced motion, 100–200% text scaling,
plain-text status, field-level validation, and AccessKit headings/live feedback. Private document
bodies do not enter navigation, registry data, routine diagnostics, or error strings.

### 6.2 E0 — tracking authority

- [x] Freeze Stage 4E entry parity at `29 implemented`, `6 deferred-beta`.
- [x] Record the dependency order and atomic work packages before changing parity.
- [x] Keep `1.0.0-alpha.2` as the source version until an explicit release-line decision.
- [x] Keep five-target package qualification release-only.

### 6.3 E1 — document application facade

- [x] Add typed current-list, current-kind, and accepted-set operations.
- [x] Require explicit private-read consent before returning document bodies.
- [x] Validate job UUIDv7 input before workspace access.
- [x] Preserve exact plan, planned-document, generation-task, revision, citation, and placeholder
  data.
- [x] Return the store-owned accepted set only when every exact current document head matches it.

### 6.4 E2 — document CLI adapter alignment

- [x] Route document list/show/set through `canisend-app`.
- [x] Preserve committed JSON shapes, human summaries, artifacts, error codes, and exit classes.
- [x] Keep document mutation in the existing Agent-task lifecycle; do not add an untracked CLI
  editor.

### 6.5 E3 — Documents GUI

- [x] Add selected-job sub-navigation and a Documents view.
- [x] Require a user-invoked private-read confirmation before loading bodies.
- [x] Show the accepted set and every exact member revision once.
- [x] Render sections, claims/citations, placeholders, and generation metadata with body-free
  collapsed
  summaries by default.
- [x] Show ready/stale/missing recovery through the existing task and workflow controls.

### 6.6 E4 — review application facade

- [x] Add typed current-review, disposition-template, and confirm-disposition operations.
- [x] Require explicit private-read consent before exposing finding messages and rationales.
- [x] Keep deterministic findings read-only.
- [x] Validate the exact review artifact, finding identity/revision, selected disposition, and
  rationale
  before mutation.
- [x] Return the revised artifact and preserve store-owned package/render invalidation.

### 6.7 E5 — review CLI adapter alignment

- [x] Route review export/confirm/show through `canisend-app`.
- [x] Preserve create-new private JSON export, regular-file input, committed JSON/human output, and
  exit
  classes.
- [x] Keep explicit user review as the only authority for accepted-risk or dismissed dispositions.

### 6.8 E6 — Review GUI

- [x] Load current findings and an editable human-disposition template after explicit private-read
  consent.
- [x] Show deterministic authority and blockers without an enabled disposition control.
- [x] Require a non-empty rationale for every selected human disposition and at least one
  selection.
- [x] Preview package/render invalidation and confirm the revised review explicitly.
- [x] Restore focus to the first invalid field or the review action after completion.

### 6.9 E7 — package application and CLI

- [x] Add typed readiness check/current, projection export/current, reconcile, replace, and
  copy-as-new operations.
- [x] Add a distinct explicit private-export consent boundary.
- [x] Validate job UUIDv7 and safe relative destinations before workspace access.
- [x] Preserve readiness reasons, revision binding, new-file behavior, edit detection, user-copy
  preservation, and authoritative-artifact immutability.
- [x] Route all package CLI commands through `canisend-app` without external contract drift.

### 6.10 E8 — package GUI

- [x] Show deterministic package readiness and exact body-free reasons.
- [x] Preview a job-scoped destination before an explicit private export.
- [x] Show the resulting export receipt and every managed projection path/status.
- [x] Reconcile only on user request.
- [x] Require a separate confirmation to discard an edit, or a distinct safe destination to
  preserve
  the edit before restoring the generated projection.

### 6.11 E9 — render application and CLI

- [x] Add typed build/current/export operations through the embedded renderer.
- [x] Add distinct private PDF-export consent and safe relative destination validation.
- [x] Preserve package revision binding, bounded PDF validation, manifest identity,
  page/byte/warning
  counts, no-overwrite behavior, and `submission_performed: false`.
- [x] Route render build/show/export through `canisend-app` without external contract drift.

### 6.12 E10 — render GUI

- [x] Build only when the authoritative workflow exposes Render as ready.
- [x] State that editable Typst projections are not renderer inputs.
- [x] Show every rendered document's kind, artifact revisions, page/byte/warning counts, and
  elapsed
  time.
- [x] Preview and explicitly confirm a private PDF export, then show every resulting path.
- [x] Never open an application portal or imply submission.

### 6.13 E11 — Stage 4E closure

- [x] Mark only `document.*`, `review.*`, `package.*`, and `render.*` implemented.
- [x] Record `33 implemented` and `2 deferred-beta`.
- [x] Add a repeatable document-to-review-to-package-to-render persistence and export-recovery
  regression through the worker/application boundary.
- [x] Run formatter, affected tests, relevant all-target Clippy, release check, hosted Fast CI, and
  a
  local packaged macOS bilingual/accessibility smoke.
- [x] Do not create a tag, release, package-manager update, or five-target native matrix.

Exit parity is `33/35`.

## 7. Stage 4F — inspection and Beta freeze

### 7.1 Product surface

Extend **Diagnostics** with a public, workspace-independent **Schemas & resources** section:

- verify embedded-resource integrity before presenting catalog data;
- show every public schema ID, canonical URI, version, resource ID, size, and SHA-256 digest;
- show every embedded resource ID, kind, version, path, size, and SHA-256 digest;
- provide separate schema/resource filtering without hiding the total or verification state;
- keep schema/resource metadata selectable and copyable without rendering unbounded raw bodies;
- export the complete verified catalog only after a user selects a new or empty destination;
- write a versioned manifest plus exact public embedded files with create-new semantics; and
- show the final manifest path, file count, sizes, and digests without launching an Agent host.

Catalog inspection and export do not require a workspace or private-data consent because the
inputs are compiled public resources. They remain explicit user actions. The destination rejects
symlinks, non-directories, non-empty directories, `.canisend` components, duplicate resource IDs,
unsafe embedded paths, and any existing output file. Partial output is removed on a failed export
when it was created by the current operation.

The page retains the existing semantic theme, platform typography, English and Simplified Chinese
catalog, visible keyboard focus, 44-point controls, reduced motion, 100–200% text scaling,
plain-text verification state, AccessKit headings/live feedback, and background worker. It does
not expose job adverts, profile evidence, documents, provider payloads, credentials, arbitrary
workspace files, a shell, or automatic export.

### 7.2 F0 — tracking authority

- [x] Freeze Stage 4F entry parity at `33 implemented`, `2 deferred-beta`.
- [x] Record the inspection/export boundary and atomic work packages before changing parity.
- [x] Keep `1.0.0-alpha.2` as the source version until an explicitly authorized Beta transition.
- [x] Keep native five-target packages, signing, and publication outside the ordinary edit loop.

### 7.3 F1 — bounded embedded-resource export

- Add a typed versioned export manifest in `canisend-resources`.
- Export the generated resource paths with create-new files under one new or empty root.
- Verify the compiled manifest and every selected digest before writing.
- Reject empty or duplicate selections, internal/unsafe paths, symlink components, and existing
  files.
- Roll back files and directories created by a failed export without deleting a pre-existing
  destination.
- Keep public catalog export distinct from host-specific Agent-pack export.

### 7.4 F2 — schema/resource application facade

- Add typed schema list/show, resource list/show, and complete-catalog export actions.
- Reuse committed `SchemaCatalogData`, `SchemaCatalogEntry`, `ResourceCatalogData`, and
  `ResourceCatalogEntry` contracts where their external shape is already authoritative.
- Add resource path/detail and export read models only in `canisend-app`.
- Verify embedded-resource integrity before returning catalog or export receipts.
- Preserve `schema.not_found`, `resource.not_found`, integrity, and input-path error
  classifications.
- Keep all catalog receipts public, deterministic, body-free with respect to user workspaces, and
  independent of workspace discovery.

### 7.5 F3 — CLI adapter alignment

- Route `schema list/show` and `resource list` through `canisend-app`.
- Preserve committed JSON shapes, human output, not-found codes, exit classes, and ordering.
- Remove direct schema/resource catalog construction from the CLI adapter.
- Do not add an implicit export, workspace requirement, or resource-body output to existing
  commands.

### 7.6 F4 — Diagnostics worker and state

- Add one typed worker request that loads both verified catalogs through the application facade.
- Add one typed worker request for explicit complete-catalog export.
- Keep loading, success, error, destination preview, and export receipt state separate from doctor
  state.
- Invalidate a reviewed destination when it changes and prevent duplicate dispatch while busy.
- Accept only a new or empty regular directory outside `.canisend`.

### 7.7 F5 — Diagnostics GUI

- Add schema/resource segmented navigation, text filter, totals, and integrity status.
- Render metadata in a content-first list with copyable IDs, URIs/paths, versions, sizes, and
  digests.
- Load catalogs explicitly in the background and expose progress through the existing activity
  live region.
- Preview a chosen destination before enabling export.
- Explain that exported resources are public, do not contain workspace bodies, and are not
  executed.
- Show the manifest and every created file after success, with English and Simplified Chinese
  labels and no color-only state.

### 7.8 F6 — inspection/export qualification

- Add application tests for deterministic catalogs, ID/slug lookup, not-found classification,
  integrity verification, safe destination policy, create-new output, digest equality, repeated
  export refusal, and cleanup after bounded failure.
- Add CLI binary-contract assertions proving facade routing preserves existing responses.
- Add GUI state/localization tests for filters, destination invalidation, bilingual labels,
  loading/error/success states, and disabled duplicate actions.
- Add a repeatable worker/application regression that loads catalogs without a workspace, exports
  all resources, verifies the on-disk manifest and digests, and refuses a second export.

### 7.9 F7 — Stage 4F closure and Beta freeze audit

- Mark only `schema.*` and `resource.*` implemented after F1–F6 evidence exists.
- Record `35 implemented` and `0 deferred-beta`.
- Run formatter, complete workspace tests, all-target Clippy, release check, hosted Fast CI, and
  the local packaged macOS bilingual/accessibility smoke.
- Triage available public Alpha reports without telemetry or private issue bodies and record any
  unresolved release blocker explicitly.
- Refresh the Beta readiness and contract-freeze records without pre-authorizing source changes.
- Freeze Agent v2, schema/resource formats, GUI action contracts, workspace registry, and bundle
  layout only when the checked-in freeze authority accepts the exact source commit.
- Do not transition, tag, publish, update package managers, or run the five-target native release
  matrix without separate authorization.

**Implementation exit parity:** `35/35`.

Reaching `35/35` completes GUI operation parity but does not itself authorize Beta. If public Alpha
evidence, blocker resolution, freeze activation, or transition approval is absent, the source stays
on `1.0.0-alpha.2` and the exact remaining Beta gate is reported rather than bypassed.

## 8. Verification ownership

| Change | Focused owner | Broader owner |
|---|---|---|
| application request/read model | `canisend-app` tests | Fast CI complete workspace suite |
| store confirmation/invalidation | named store regression | scheduled property/fuzz assurance |
| GUI form/reducer/accessibility | `canisend-gui` tests | local packaged macOS checkpoint |
| CLI adapter | binary contract snapshots | release-only five-target archive smoke |
| private input/export | bounded local fixtures | release privacy/lifecycle qualification |
| parity/docs | JSON parse and release check | Beta contract-freeze gate |

The ordinary edit loop does not build Windows/Linux packages, run extended fuzzing, sign, notarize,
or create a release.

## 9. Atomic sequence

1. `docs(stage4): define decision-to-beta execution stages`
2. `feat(app): add profile source and evidence actions`
3. `refactor(cli): route profile operations through application facade`
4. `feat(gui): add profile source catalog`
5. `feat(gui): add evidence confirmation workflow`
6. `feat(app): add criteria and match actions`
7. `refactor(cli): route criteria and match through application facade`
8. `feat(gui): add criteria and match workflow views`
9. `feat(app): add application plan actions`
10. `refactor(cli): route plan operations through application facade`
11. `feat(gui): add application plan confirmation`
12. `test(stage4): qualify decision workflow slice`
13. `docs(stage4): close decision workflow parity`
14. `docs(stage4): define discovery and agent execution slice`
15. `feat(app): add discovery actions`
16. `refactor(cli): route discovery through application facade`
17. `feat(gui): add discovery workflow`
18. `feat(app): add agent task actions`
19. `refactor(cli): route tasks through application facade`
20. `feat(gui): add agent task workflow`
21. `feat(app): add agent integration actions`
22. `refactor(cli): route agent integration through application facade`
23. `feat(gui): add agent integration workflow`
24. `test(stage4): qualify discovery and agent workflow`
25. `docs(stage4): close discovery and agent parity`
26. `docs(stage4): define documents and delivery slice`
27. `feat(app): add document actions`
28. `refactor(cli): route document operations through application facade`
29. `feat(gui): add document workspace`
30. `feat(app): add review actions`
31. `refactor(cli): route review operations through application facade`
32. `feat(gui): add review workflow`
33. `feat(app): add package actions`
34. `refactor(cli): route package operations through application facade`
35. `feat(gui): add package delivery workflow`
36. `feat(app): add render actions`
37. `refactor(cli): route render operations through application facade`
38. `feat(gui): add render delivery workflow`
39. `test(stage4): qualify documents and delivery workflow`
40. `docs(stage4): close documents and delivery parity`
41. `docs(stage4): define inspection and beta-freeze slice`
42. `feat(resources): add bounded catalog export`
43. `feat(app): add schema and resource actions`
44. `refactor(cli): route schema and resource operations through application facade`
45. `feat(gui): add schema and resource diagnostics`
46. `test(stage4): qualify inspection and catalog export`
47. `docs(stage4): close inspection parity and record beta gate`

Each commit must pass its focused gate. Parity changes land only after the corresponding application,
CLI, GUI, localization, accessibility, and persistence evidence exists.

## 10. Stage 4C exit decision

Stage 4C is complete when a macOS GUI user can import profile sources, explicitly confirm
evidence and criteria, inspect current matches, explicitly confirm an application plan, close and
reopen the workspace, and observe the same revision-bound state through the CLI and Agent v2
contracts.

The structured GUI surfaces and repeatable worker/application persistence qualification now cover
that path.

## 11. Stage 4D exit decision

Stage 4D is complete when a macOS GUI user can preview and commit discovery leads, promote a lead
to a job, prepare and recover revision-bound Agent tasks, inspect public Agent v2 context, export
one verified host resource pack, close and reopen the workspace, and observe the same state through
the CLI and Agent v2 contracts.

The shared application facade, body-free read models, `discovery -> task -> agent` reopen
regression, hosted Fast CI, and packaged bilingual/accessibility smoke now cover that path.

## 12. Stage 4E exit decision

Stage 4E is complete when a macOS GUI user can inspect the accepted structured document set,
explicitly disposition human-review findings, prove package readiness, export and reconcile
editable projections, recover user edits without changing authoritative artifacts, build validated
PDFs, export them with separate consent, reopen the workspace, and observe the same persisted
state.

The shared application facade, stable CLI adapters, selected-job Documents and Review & export
views, worker/application delivery regression, hosted Fast CI, and packaged bilingual/accessibility
smoke now cover that path. Stage 4F is next at `33 implemented` and `2 deferred-beta`. No Alpha.3,
Beta, RC, Stable, package-manager, five-target native matrix, or public update action is implied.
