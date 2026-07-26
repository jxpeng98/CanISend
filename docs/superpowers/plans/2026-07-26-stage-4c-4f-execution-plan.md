# CanISend Stage 4C–4F execution plan

**Status:** Stage 4C complete; Stage 4D next; Stage 4E–4F planned

**Decision date:** 2026-07-26

**Source baseline:** `1.0.0-alpha.2` after public checkpoint `v1.0.0-alpha.2`

**Entry parity:** `22 implemented`, `13 deferred-beta`

**Stage 4C exit parity:** `26 implemented`, `9 deferred-beta`

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

Implement:

- adapter/source inspection, bounded CSV/JSON/host-agent import, user-invoked refresh, lead
  history, suggestions, and promotion;
- prepared task inspection, declared inputs, consent, lease/start, result validation, completion,
  failure, stale handling, and recovery; and
- Agent v2 capability/context inspection plus Codex, Claude, and generic asset-pack export.

Provider credentials remain outside workspace and registry storage. The GUI may reveal a prepared
task directory or copy an exact command but cannot start a general shell. Exit parity is `29/35`.

## 6. Stage 4E — documents and delivery

Implement:

- current document set, member revisions, acceptance, and stale-state handling;
- deterministic and human-review findings with explicit dispositions;
- package readiness, reconciliation, edited-projection recovery, and private export consent; and
- trusted embedded render, PDF inspection, render manifest, and private export.

No control submits to an application portal. Existing destinations are never overwritten, edited
managed projections require an explicit recovery choice, and exports remain revision-bound. Exit
parity is `33/35`.

## 7. Stage 4F — inspection and Beta freeze

Implement read-only schema/resource catalog inspection and bounded export in Diagnostics. Then:

- reach `35/35` parity;
- triage public Alpha reports without telemetry or private issue bodies;
- resolve data-loss, privacy, integrity, accessibility, and upgrade blockers;
- freeze Agent v2, schema/resource formats, GUI action contracts, workspace registry, and bundle
  layout;
- refresh Beta readiness and contract-freeze records; and
- transition to `1.0.0-beta.1` only through the checked-in stage-transition authority.

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

Each commit must pass its focused gate. Parity changes land only after the corresponding application,
CLI, GUI, localization, accessibility, and persistence evidence exists.

## 10. Stage 4C exit decision

Stage 4C is complete when a macOS GUI user can import profile sources, explicitly confirm
evidence and criteria, inspect current matches, explicitly confirm an application plan, close and
reopen the workspace, and observe the same revision-bound state through the CLI and Agent v2
contracts.

The structured GUI surfaces and repeatable worker/application persistence qualification now cover
that path. Stage 4D is next. No Alpha.3, Beta, RC, Stable, package-manager, or public update action
is implied.
