# Workflow

Use this reference for the normal installed-package workflow. From a development checkout, prefix commands with `uv run`.

## 1. Initialize Or Inspect Workspace

For a new user workspace:

```bash
canisend init-workspace --workspace <private-workspace>
canisend agent context --workspace <private-workspace> --format json
```

For an existing workspace, start with `agent context`. Use `doctor` when a human-readable environment diagnostic is
also useful, and resolve missing profile, prompt, skill, provider, or Typst items before generating
application-facing material.

After package upgrades:

```bash
canisend update-workspace --workspace <private-workspace>
canisend agent context --workspace <private-workspace> --format json
```

Use `--overwrite` only when the user intentionally wants packaged defaults to replace local prompt, template, or skill edits.

## 2. Prepare Profile Evidence

The user should keep real CV and statement sources in ignored `profile/`. In Typst-first workflows, these are already-written `modernpro-cv` and `modernpro-coverletter` sources.

Regenerate normalized evidence whenever profile sources change:

```bash
canisend extract-profile-evidence --workspace <private-workspace>
```

Typst-backed generated evidence now includes a source-hash receipt. If Evidence reports
`evidence.source_receipt_missing` for older generated output, or `evidence.source_receipt_stale` after its raw source
changes, rerun this command. Do not repair the receipt manually.

Agents should read generated evidence from `profile/generated/`, not directly rely on prose claims in the private CV. New claims should cite item-level citations such as `profile/generated/cv.evidence.md#Teaching/cv-001`.

If generated evidence is incomplete, first report the gap. Read raw profile sources only with user approval, because in agent-assisted mode the content read by the agent may be processed by the agent model provider. `extract-profile-evidence --llm-augment` must also be explicit opt-in; it rejects augmented items that do not cite a local source chunk.

## 3. Fetch And Select Leads

Fetch jobs.ac.uk RSS leads locally:

```bash
canisend fetch-jobs-ac-uk \
  --workspace <private-workspace> \
  --feed-url "<jobs-ac-uk-rss-url>" \
  --include "<keyword>" \
  --exclude "<keyword>"
```

Use the generic command for another stable RSS or Atom source:

```bash
canisend fetch-job-feed \
  --workspace <private-workspace> \
  --source-name "<source label>" \
  --feed-url "<rss-or-atom-url>"
```

Feed leads are discovery records, not full adverts. Ask the user to choose a lead index unless they already provided
one. Generic source files are written under `job_leads/<source-name>.json` and should be passed with `--leads-file`.

## 4. Create One Job Workspace

From a feed lead:

```bash
canisend new-job-from-lead \
  --workspace <private-workspace> \
  --lead-index <index> \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD"
```

Manual job creation:

```bash
canisend new-job \
  --workspace <private-workspace> \
  --title "<job title>" \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD" \
  --source-url "<source-url>"
```

Paste or import the full selected advert into `jobs/<job-slug>/job_advert.md` before relying on parser output. Direct
intake may use `new-job --advert-file <advert.pdf|advert.md|advert.txt>`.
`new-job --source-url` records metadata only. Use `--fetch-url` only when the user explicitly asks to import that one
supplied HTML or PDF advert; it does not authorize site crawling or search-result scraping.

After creating or selecting a job, any fresh host session can resume with:

```bash
canisend agent context \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

The workspace is authoritative; no previous prompt or chat transcript is required.

## 5. Run Or Resume Evidence And Parse

Evidence and Parse are independent after intake and can run in either order. Run Evidence deterministically:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage evidence \
  --mode deterministic \
  --format json
```

Evidence prepares one immutable private
`workflow/runs/<run-id>/inputs/evidence-snapshot.json`; its TaskSpec names only that job-local input. It promotes a
strict `evidence_catalog.json` with stable content-derived IDs and `available`, `empty`, or `unavailable` state. The
snapshot, candidate, and catalog may contain normalized profile bodies and duplicate them until the user removes the
private run or job directory. Workflow state, receipts, manifests, errors, ordinary command output, and AgentResponse
extensions do not contain those bodies.

This resumable slice accepts only profiles inside the workspace. It rejects a workspace-external profile root,
unsafe manifest paths, symlinks, hard-link aliases, non-regular files, changing inputs, and versioned size-limit
violations. These restrictions do not redefine compatibility behavior of unrelated legacy commands.

Inspect Parse without writing state:

```bash
canisend stage status \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

Use the deterministic executor when local parsing is appropriate:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage parse \
  --mode deterministic \
  --format json
```

For current-host parsing, first obtain explicit approval to read the full advert, then run `stage prepare --mode
host-agent`. Read its immutable TaskSpec, create candidate JSON in a fresh scratch file, and pass that file through
`stage submit --candidate-file`. The guarded submit service writes the declared candidate and TaskResult; after
review, pass the returned paths to `stage apply`. TaskSpec and receipts are read-only through their AgentResponse
references; never write or modify a declared run path or `parsed_job.json` directly.
Submit and Apply reject stale inputs, output drift, undeclared paths, path aliases, wrong hashes, invalid schemas, and
source receipts that do not resolve to the advert.

An unchanged deterministic rerun is a cache hit. Advert or relevant job metadata changes make Parse stale; profile
evidence, writing preferences, package status, and downstream artifacts do not.

Host-agent execution applies to Parse and structured Cover Letter/Research Statement Draft. Configured-provider
execution currently applies only to Cover Letter Draft. Each prepared task has a separate immutable preparation receipt. If its inputs, upstream
dependencies, or protected output change before promotion, do not prepare a parallel task: cancel the active one
first, preserving its audit trail and candidate:

```bash
canisend stage cancel \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage <stage> \
  --format json
```

Cancellation releases the active-task slot but does not authorize overwriting a drifted authoritative output.

## 6. Project And Confirm Stable Criteria

After Parse is current, project its criteria into the Stage 2 semantic contract:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage confirm \
  --mode deterministic \
  --format json
```

Confirm writes `criteria.json` through the same candidate-validation and atomic-promotion boundary as Parse. Each
criterion has a stable ID, essential/desirable importance, categorical confidence, source-known state, and separate
user-confirmation state. Missing receipts remain unknown; ambiguous receipts expose candidate spans without silently
selecting one. The operation returns `review_required` while criteria remain unconfirmed or source-unknown, a
correction needs reconciliation, or an empty extraction remains unconfirmed. A technically successful run is not
evidence of user confirmation.

`confirmed_corrections.yaml` is an optional user-owned typed overlay. Confirm reads it; neither Parse nor Confirm
creates, rewrites, or deletes it. Users may edit it directly against
`schemas/confirmed-corrections.schema.json`; status and stage reruns validate the bytes present without normalizing
them. An explicitly consented scoped update writes a canonical next revision and may not preserve YAML comments. An
agent must use the mutation service instead of writing the YAML itself:

```bash
canisend corrections status --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend corrections init --workspace <private-workspace> --job jobs/<job-slug> \
  --confirm-user-owned-write --format json
canisend corrections status --workspace <private-workspace> --job jobs/<job-slug> --format json
```

Copy the current artifact hash and `canisend.user_artifact_revision` from the last status response. Create one bounded
strict YAML or JSON patch in safe scratch space:

```yaml
operation: correct_criterion
criterion_id: criterion_<32-lowercase-hex>
corrected_text: <user-approved-private-text>
```

Then update through compare-and-swap:

```bash
canisend corrections update \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --patch-file <safe-scratch-patch.yaml> \
  --expected-revision <current-revision> \
  --expected-sha256 <current-sha256> \
  --confirm-user-owned-write \
  --format json
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> \
  --stage confirm --mode deterministic --format json
```

The other correction operations are `confirm_criterion`, `withdraw_criterion`, and `confirm_empty`; an ambiguous
criterion confirmation may include its current `source_occurrence`. Unknown is not confirmed empty: initialization
with no corrections does not confirm an empty extraction and is fingerprint-neutral. Each semantic patch requires
current Parse and Confirm. Rerun Confirm after every update before status and the next patch; never apply several
patches against one stale Criteria catalog. If an advert source or parser interpretation changes, history remains
present and Confirm returns a privacy-safe reconciliation action instead of silently reassigning it.

## 7. Propose Durable Criterion Matches

After Confirm and Evidence are current, run Match deterministically:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage match \
  --mode deterministic \
  --format json
```

Match reads only `criteria.json` and `evidence_catalog.json`. It writes one canonical `strong`, `partial`, `weak`,
`missing`, or `unknown` classification per criterion, with matcher provenance, explicit gaps, and opaque
`evidence_catalog.json#items/<evidence-id>` references. It never copies evidence text, private headings, legacy item
labels, or evidence kinds into `criterion_matches.json`.

An empty Evidence catalog and unavailable Evidence produce different `unknown` gaps; an available catalog with no
relevant support produces `missing`. Unknown Criteria extraction blocks Match and routes back to Criteria review.
Every classification has `review_state=proposed`. Review the proposals; they are neither a user-owned application
Decision nor evidence that a package is ready. Editing either catalog directly is forbidden—rerun its owning stage.

## 8. Record Or Reconfirm The User-Owned Decision

Decision status is read-only. Initialization creates an explicitly `undecided` record only when absent:

```bash
canisend decision status --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend decision init --workspace <private-workspace> --job jobs/<job-slug> \
  --confirm-user-owned-write --format json
canisend decision status --workspace <private-workspace> --job jobs/<job-slug> --format json
```

Put one `set_decision` or `reset_decision` operation in a strict patch file. Rationale, if intentionally included,
belongs only in that private patch and user YAML:

```yaml
operation: set_decision
decision: apply
rationale_mode: keep
```

Run `decision update` with `--patch-file`, the latest `--expected-revision` and `--expected-sha256`, explicit
`--confirm-user-owned-write`, and `--format json`. Apply, hold, and skip are effective only through this user-owned
operation; undecided and Match proposals are not implicit decisions.

If current Criteria or Match receipts change, `application_decision.yaml` keeps its accepted bytes and value while
`decision status` derives `canisend.decision_basis_status=review_required`. Review the new basis, request a fresh
status baseline, and send a new `set_decision` patch—even with the same value—to reconfirm it. Never write a stale
flag into the YAML. If a response reports recovery required or receipt pending, complete only the already accepted
claim:

```bash
canisend user-mutation recover \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --mutation-id mutation_<32-lowercase-hex> \
  --confirm-user-owned-write \
  --format json
```

The CAS protocol coordinates cooperative CanISend writers while the job directory remains in place. It does not
linearize a normal editor save during the final replace window or a hostile same-user rename. Run status immediately
before mutation and avoid concurrent manual saves. Fresh status can report recovery pending if a process stopped
between publishing a complete target link and removing CanISend's verified private temporary link; only explicit
recovery cleans that exact two-link marker, while ordinary hard links remain unsafe. The private YAML/candidate and
corrected Criteria are Tier 2; the immutable body-free receipt is Tier 1 and AgentResponse never carries correction
text or rationale.

Reset/clear/withdraw operations do not erase prior private text from immutable history. Corrections history retains
old bodies, and accepted private-mode Tier 2 mutation candidates (0600 on POSIX) remain for audit/recovery. Keep the ignored job folder
private and back it up only intentionally. Removing selected private mutation events (or the whole job) is a separate
retention action that can disable recovery; CanISend does not currently provide automatic secure erasure.

## 9. Confirm The User-Owned Brief And Build The Document Plan

Brief work requires a current confirmed `decision=apply`. Status is read-only and body-free. Initialization creates
the Tier 2 user-owned file only when absent and bootstraps concrete legacy language/style values once:

```bash
canisend brief status --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend brief init --workspace <private-workspace> --job jobs/<job-slug> \
  --confirm-user-owned-write --format json
canisend brief status --workspace <private-workspace> --job jobs/<job-slug> --format json
```

For each change, create one bounded strict patch in safe private scratch space. For example:

```yaml
operation: set_brief_text
field: motivation
value: <user-approved-private-motivation>
```

Apply it with the latest Brief revision/hash and explicit consent:

```bash
canisend brief update \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --patch-file <safe-scratch-patch.yaml> \
  --expected-revision <current-revision> \
  --expected-sha256 <current-sha256> \
  --confirm-user-owned-write \
  --format json
```

Other scoped operations set/reset language, writing style, motivation, emphasis, exclusions, the document requirement
set, and individual document choices. Never replace or normalize `application_brief.yaml` directly. Status reports
only field/count state and safe receipts; reading its private body in agent-assisted mode remains ask-first Tier 2.

Run deterministic Brief planning to create or refresh the core-owned Tier 2 plan:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage brief \
  --mode deterministic \
  --format json
```

The first run may deliberately produce review blockers. Take
`canisend.document_requirements_basis_sha256` from Brief-stage status before sending a
`confirm_document_requirements` patch. A non-empty set may be `confirmed`; an empty Parsed Job list stays
`unconfirmed` unless the user explicitly selects `confirmed_empty` against that exact basis. Do not infer confirmed
none from absence, ambiguity, or extraction failure. A non-empty confirmation also requires every parsed document to
reconcile to a complete positive advert member. Conditional, alternative, qualified, truncated, multiline, or
otherwise unreconciled source context remains unknown and must not be confirmed.

`required_document_plan.json` records one task per normalized requirement. Its body may contain advert source text
and prepare/omit strategy, so an agent asks before reading it. AgentResponse contains only safe paths, hashes, opaque
IDs, states, blocker codes, and counts. An unconfirmed requirement set or Brief field, unresolved document choice,
`required + omit`, required item without preparation, or orphaned old choice blocks later Draft/Verify work. Rerun
Brief after every accepted relevant patch; never edit the plan directly.

Inspect the derived fan-out before preparing any document:

```bash
canisend documents status \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

This read-only command returns counts and generic actions only. `ready_to_prepare` can mean the guarded Cover Letter
or Research Statement executor can run. A confirmed teaching, supporting, diversity, publication, CV, or other
planned document remains `executor_unavailable`; it is not silently omitted and prevents complete fan-out. The
projection is re-derived from the current plan hash and is not a mutable artifact or a package-readiness decision.

Draft and Review runs are keyed by `(stage, document_id)`. One current dispatchable guarded target is resolved
automatically. If Cover Letter and Research Statement are both ready, ask before reading the Tier 2 plan, select the
exact stable ID mapped to the intended normalized kind, and pass `--document-id <document_...>` to `stage status`,
`prepare`, `run`, and `cancel`. Reuse that exact ID for Review. Do not infer an ID from a label, list position, output
filename, or provider text; ambiguous omitted, mismatched, or unsupported selection must fail before task creation.

If the Decision basis changes, Brief bytes remain present and status requires a new `reconfirm_brief` scoped patch.
Stage 2 is locally accepted. A current plan completes the resumable Stage 2 decision spine, but it does not make
Draft outputs or the application package ready, reviewed, final, or submission-ready.

### Manual Ownership Of The Three User YAML Files

`confirmed_corrections.yaml`, `application_decision.yaml`, and `application_brief.yaml` remain user-owned. The user
may edit any of them manually against its schema; read-only status and stage reruns validate the bytes without
normalizing or replacing them. Their bodies are Tier 2 ask-first in agent-assisted work.

An agent never writes a whole YAML file. It runs the matching `corrections`, `decision`, or `brief` status operation,
creates one bounded strict patch in safe private scratch space, and passes the latest raw-byte revision and SHA-256
through compare-and-swap with explicit `--confirm-user-owned-write`. Obtain a new status baseline for every patch and
serialize writers. CAS coordinates cooperative CanISend writes; it does not make a concurrent manual editor save
safe. Corrections additionally require current Parse and Confirm plus a Confirm rerun after every semantic patch;
Decision requires current Match; Brief requires a current confirmed `decision=apply`.

## 10. Generate And Review Structured Document Drafts

After Brief succeeds with one blocker-free confirmed `prepare` Cover Letter or Research Statement, ask before
reading its Tier 2 plan/inputs. Include the selected ID when more than one target is ready:

```bash
canisend stage prepare \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage draft \
  --mode host-agent \
  --document-id <document_...> \
  --format json
```

The prepared task returns the `read-private-draft-inputs` consent ID. Read only the returned TaskSpec paths after
that separate Tier 2 consent. The document-scoped 1.1 TaskSpec, result, submission, validation, manifest, promotion,
and state records all echo the same stable ID; non-document 1.0 task shapes remain unchanged. Produce JSON matching
the adapter-selected schema in fresh private scratch: `schemas/cover-letter-draft.schema.json` for Cover Letter or
`schemas/research-statement-draft.schema.json` for Research Statement. Every applicant-facing block must be an explicit
Claim; strong/partial facts use current Evidence IDs, while unsupported facts remain explicit blockers. Pass the
scratch file to `stage submit`, then pass the returned immutable TaskResult to `stage apply`. Never write the
declared candidate/result paths or either authoritative Draft directly. Research Statement requires
`research_overview`, `research_contributions`, and `future_agenda` sections for blocker-free completeness Review.

For Cover Letter only, if the user instead explicitly approves Tier 3 transmission to the configured provider or
command, run:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage draft \
  --mode configured-provider \
  --allow-provider-backed \
  --format json
```

This sends exactly the seven Draft TaskSpec inputs to the provider. The provider returns only section and Claim
semantics; core derives the trusted envelope, hashes, stable IDs, mode, and review state, then uses the same candidate
validation and atomic promotion path. Raw output is bounded and not retained. No consent means no provider
construction, call, or workspace write; a cache hit also makes no call. Provider failure, invalid output, or input
drift promotes nothing. If a valid candidate was already submitted before interruption, rerunning resumes without a
second provider call. This path is distinct from legacy `canisend run --llm-drafts`.

Run independent deterministic Review:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage review \
  --mode deterministic \
  --format json
```

If Draft used explicit `--document-id`, pass the same option to Review. Review may consume only the Draft instance
with that composite identity.

Use body-free counts/codes first; ask before reading Claim or finding bodies. Blockers require a new validated Draft.
Open semantic-support and non-factual Claim-kind findings require human inspection. Start with
`canisend review-dispositions status ... --document-id <same-id> --format json` for either Cover Letter or Research
Statement, initialize with explicit write consent, then submit one strict patch per finding through
`review-dispositions update` with the latest revision/hash and same ID. The ID may be omitted only when one supported
prepared target exists. Use `set_finding_disposition` with `accepted` or `revision_required`; a blocker cannot be accepted.
If Draft/Review changed, use one explicit `reset_for_current_review` patch before inspecting the new
finding set. Complete current acceptances derive per-document readiness while Draft and Review remain `proposed`;
this is not package readiness. A reviewed Research Statement may produce a standalone compatibility view, but it
does not enter application-package content or package gates.

### Aggregate Cross-Document Review

After the selected document Reviews and dispositions are current, run the independent aggregate stage without a
document ID:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage package_review \
  --mode deterministic \
  --format json
```

The stage may also run while required documents are missing or unavailable; those states become durable findings
rather than making aggregate Review disappear. `package_review_findings.json` binds exact plan, Draft, Review,
disposition, and derived-readiness receipts. Use the body-free AgentResponse first and ask before reading Tier 2
finding or correction bodies.

A required omitted, unavailable, missing, stale, blocked, unreviewed, or revision-required document is a blocker.
The same normalized factual assertion with different support classifications or Evidence receipt sets is also an
exact blocker. Repeated wording with the same receipts, shared Evidence with different wording, tone,
proportionality, emphasis, and narrative alignment require human review. Do not infer typed factual contradictions
from prose. A correction proposal never edits a Draft: target the named document/Claim set with a new guarded Draft
candidate, then rerun document Review, dispositions, and aggregate Review.

Aggregate Review remains `proposed`. Inspect derived aggregate status without reading finding bodies, then create
or update the independent user-owned decision file only with explicit consent:

```bash
canisend package-review status --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend package-review init --workspace <private-workspace> --job jobs/<job-slug> \
  --confirm-user-owned-write --format json
canisend package-review update --workspace <private-workspace> --job jobs/<job-slug> \
  --patch-file <patch.yaml> --expected-revision <revision> --expected-sha256 <sha256> \
  --confirm-user-owned-write --format json
```

One update uses `set_package_finding_disposition` or `clear_package_finding_disposition` for one current finding.
Use `accepted` or `revision_required`; blockers cannot be accepted. When aggregate Review changes, the old YAML is
preserved and must be explicitly rebound with `reset_for_current_package_review` before new decisions are made.
Every required document must already be individually reviewed, and every current non-blocker aggregate finding
must have a decision, before the derived application-package state becomes `reviewed`.

If an accepted package decision reports receipt or recovery pending, use the generic body-free
`user-mutation recover` operation with its mutation ID and explicit recovery consent; do not replay the patch.

`canisend check-package` independently rederives this boundary as APP-Q5. Missing, invalid, stale, incomplete, or
changed aggregate receipts fail closed, including for backward-readable legacy packages. Application-package
`reviewed` is not rendering approval, manual submission, or proof of submission.

## 11. Generate The Compatible Draft Package

Deterministic baseline:

```bash
canisend run --workspace <private-workspace> --job jobs/<job-slug>
```

Legacy monolithic LLM-backed parse and draft generation require explicit opt-in and provider configuration:

```bash
canisend run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

Use only `--llm-parser` when the user wants structured parsing but not drafted prose. Use only `--llm-drafts` when deterministic parsing is sufficient.

Always ask before enabling LLM-backed flags or a command provider for a real workspace, because those modes can send selected private advert, profile, evidence, and draft context to the configured provider. If the user has not opted in, run the deterministic baseline and report any gaps for manual review.

### Structured Match And Draft Views In The Compatible Pipeline

For `canisend run --workspace ...` without `--llm-drafts`, the pipeline uses the configured workspace profile and
will consume structured Match only when Match is current, its Criteria/Evidence/Match hashes and graph validate, and
the authoritative `parsed_job.json` equals the parse result for this run. The classifications remain visibly
`proposed`; the generated views are not Decision, confirmation, Draft readiness, or package readiness.

When those guards pass, the same deterministic view drives `02_fit_report.md`, `05_criteria_checklist.md`, the
structured essential-criteria checks in `07_material_review_checklist.md`,
`typst/application_package_content.json`, and `typst/application_package.typ`. The compatible
`06_final_application_package.md` also receives the same fit/checklist text.

Under the same guards, a current validated `cover_letter_draft.json` is projected only when deterministic Review is
current and has zero blocker findings. Each Claim is emitted once, in structured order, to
`03_cover_letter_draft.md`, `typst/cover_letter_content.json`, the Cover Letter portion of
`typst/application_package_content.json`, and the two Typst sources. Projection metadata binds the exact Draft and
Review hashes. `review_dispositions.yaml` binds the same receipts: every non-blocker finding must be explicitly
`accepted`, no finding may be `revision_required`, and blockers remain non-waivable before derived Cover Letter
readiness becomes `reviewed` and `requires_human_review=false`. Missing, stale, incomplete, or revision-required
dispositions remain review work. `check-package` re-derives this document gate; it does not infer whole-package
readiness from one Cover Letter.

An exact current Research Statement Draft, deterministic blocker-free Review, current audited dispositions, and
derived `reviewed` state may additionally produce `08_research_statement.md` and
`typst/research_statement.typ`. Each Claim is rendered once with exact Draft/Review/disposition/readiness and
Markdown hash provenance. These standalone files are not embedded in application-package content and do not enter
`check-package` requirements, issues, or input hashes. If a prior Research projection becomes ineligible, generated
views become body-free unavailable; an edited Typst primary is preserved and receives a candidate for reconciliation.

If Match or an upstream stage is stale, any protected structured output is drifted or tampered, the structured graph
is invalid, the current run parses a different job view, or `--profile-dir` selects a profile other than the
workspace-configured profile, the command safely generates the legacy deterministic views instead of mixing
provenance. `--llm-drafts` always keeps provider-generated draft views and does not replace them with deterministic
Match views. Legacy fallback is compatibility behavior, not evidence that the structured proposal or package is
current; report the reason and refresh Stage 2 before relying on Match.

A missing/blocked Review, Draft or Review drift, invalid structured content, a direct library call without workspace
provenance, or any guard above also keeps the legacy/provider Cover Letter path. If an editable Typst source has
changed, the structured regeneration is written to `*.generated.typ` for reconciliation rather than overwriting it.

## 12. Review Before Rendering

Review, in order:

1. `parsed_job.json`
2. `criteria.json`
3. Evidence state and receipts in `evidence_catalog.json` (read its private bodies only when needed and approved)
4. Proposed classifications and gaps in `criterion_matches.json`
5. Brief status and `application_brief.yaml` only when its Tier 2 body is needed and approved
6. Brief-stage status and `required_document_plan.json` only when its Tier 2 body is needed and approved
7. Draft-stage status and the selected document's Draft JSON only when its Tier 2 Claim body is needed and approved
8. Review-stage status and the selected document's Review JSON only when its Tier 2 body is needed and approved
9. Body-free `review-dispositions status --document-id <same-id>`, then the selected document's disposition YAML only
   when its Tier 2 body is needed
10. `00_preparation_questions.md`
11. `05_criteria_checklist.md`
12. `02_fit_report.md`
13. `03_cover_letter_draft.md`
14. `04_cv_tailoring_notes.md`
15. `07_material_review_checklist.md`
16. `08_research_statement.md` when generated
17. `typst/cover_letter.typ`
18. `typst/research_statement.typ` when present
19. `typst/application_package.typ`
20. `06_final_application_package.md`

Apply `quality-gates.md` before treating any output as usable.
In particular, check language/style confirmation, item-level citations, unsupported claims, required-document coverage, and private-file safety before presenting a package as ready.

Repeated generation protects edited Typst sources. If `run` reports a `*.generated.typ` candidate, compare it with the
preserved user-edited `.typ` file and merge intentionally before rendering. The optional Research Statement candidate
does not affect `check-package`, but it still blocks `render-typst` until reconciled.

Use `canisend check-package --write-report` when a machine-readable `application_gate_report.json` is useful. Without
that flag, the check remains read-only.

In agent-assisted mode, also report which private sources were read directly, which LLM-backed CLI flags were used, and which remaining claims need manual confirmation.

## 13. Optional Typst Rendering

Render only when the user asks for PDFs or needs local PDF review:

```bash
canisend render-typst --workspace <private-workspace> --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not.

## 13. Manual Submission

The tool stops at preparation. The user manually handles portal upload, eligibility declarations, equality monitoring, right-to-work, disability, visa, conflict, criminal record, and other sensitive form answers.
