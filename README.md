<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/ci.yml?branch=main&label=ci" alt="CI status"></a>
  <a href="https://test.pypi.org/project/canisend/"><img src="https://img.shields.io/badge/TestPyPI-0.6.0b1-blue" alt="TestPyPI"></a>
  <img src="https://img.shields.io/badge/python-3.11%2B-blue" alt="Python 3.11+">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license">
</p>

# 这也能投/CanISend

Evidence-backed application prep for academic and professional jobs. CLI/package name: `canisend`.

这也能投是一款 local-first CLI：从职位广告和本地私人履历证据出发，生成可检查的申请材料包，包括 parsed criteria、fit report、cover letter draft、CV tailoring notes、material checklist 和 Typst-ready source。强主张应当能追溯到本地证据。

It prepares materials only. It does not submit applications, create accounts, fill portals, crawl job sites, upload packages, or answer sensitive declarations. A user may explicitly import one supplied advert URL; CanISend does not run background page discovery or broad scraping.

## What It Does

- Creates a private workspace for profile evidence, job folders, prompts, Typst templates, schemas, and agent instructions.
- Builds one private discovery catalog from jobs.ac.uk and generic RSS/Atom feeds, read-only Greenhouse/Lever public
  boards, local CSV/JSON/email-alert exports, and normalized Codex/Claude/other host-agent search results.
- Creates one local folder per application, with the full advert kept as a manual input.
- Extracts normalized evidence from Typst-first profile sources into `profile/generated/`.
- Materializes a private, run-scoped Evidence snapshot and durable `evidence_catalog.json` without exposing evidence
  bodies in workflow control records.
- Produces deterministic, reviewable `criterion_matches.json` proposals from current Criteria and Evidence catalogs.
- Preserves user-owned criteria confirmations/corrections and apply/hold/skip decisions through explicit-consent,
  revision/hash compare-and-swap operations with privacy-safe receipts.
- Provides the locally accepted Stage 2 Decision Spine: guarded user-owned corrections, Decision and Brief YAML,
  deterministic Criteria/Evidence/Match/required-document projections, and executable unresolved/omit/orphan blockers.
- Provides two Stage 3 guarded executors: host-agent or configured-provider Cover Letter plus host-agent Research
  Statement, each with an evidence-bound structured Draft, independent deterministic Review, user-owned dispositions,
  derived per-document readiness, and unique outputs.
- Adds deterministic aggregate `package_review` over the exact current document plan and document receipts. Missing
  required documents and provable Evidence-receipt conflicts are blockers; tone and narrative alignment remain
  explicit human-review work, and the stage never rewrites a Draft.
- Preserves package finding decisions in an independent user-owned CAS file and derives body-free application-package
  readiness from exact required-document, aggregate Review, and decision receipts. `check-package` rederives APP-Q5;
  this review boundary is not rendering approval or submission evidence.
- Derives a body-free required-document execution fan-out, so guarded Cover Letter/Research Statement work,
  confirmed omissions, unresolved tasks, and document kinds without implemented executors remain distinguishable
  across agent hosts.
- Renders current deterministic Match proposals and a current blocker-free structured Cover Letter into compatible
  views, plus an exact reviewed Research Statement into standalone views, while safely closing stale, drifted,
  blocked, mixed-profile, direct-library, or explicit LLM-draft paths. Document readiness never implies
  package/submission readiness.
- Generates `parsed_job.json`, preparation questions, fit reports, cover letter drafts, CV tailoring notes, criteria checklists, material review checklists, and structured Typst content.
- Runs deterministic local generation by default, with explicit opt-in for LLM-backed evidence augmentation, parsing, or drafting.
- Ships bridge files plus a self-contained workspace skill pack for Codex, Claude Code, and IDE agents.

## Quick Start

Choose one installation method. CanISend requires Python 3.11 or newer.

### Install as an isolated CLI with `uv`

Use this after the production release is available on PyPI:

```bash
uv tool install canisend
```

### Install as an isolated CLI with `pipx`

If you prefer the `pip` ecosystem but still want an isolated command-line tool:

```bash
pipx install canisend
```

### Install with `pip` in a virtual environment

Use this when you want CanISend available inside a project-specific Python environment:

```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install canisend
```

### Install the current TestPyPI build

For the current TestPyPI build, install `canisend==0.6.0b1` while still resolving dependencies from PyPI:

```bash
uv tool install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.6.0b1
```

Or with `pip` inside an active virtual environment:

```bash
python -m pip install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.6.0b1
```

Run the packaged fake-data workflow before using private profile or job data:

```bash
canisend run-example --workspace /tmp/canisend-example --overwrite
```

Inspect the generated dossier:

```text
/tmp/canisend-example/jobs/2026-06-15_example-university_lecturer-in-applied-economics/
  parsed_job.json
  00_preparation_questions.md
  02_fit_report.md
  03_cover_letter_draft.md
  05_criteria_checklist.md
  07_material_review_checklist.md
  typst/
```

From a development checkout, prefix CLI commands with `uv run`:

```bash
uv run canisend --help
uv run canisend run-example --workspace /tmp/canisend-example --overwrite
```

## Core Workflow

```text
private profile -> generated evidence -> private run snapshot -> Evidence catalog
                                                            |            |
full advert -> Parsed Job -> stable Criteria ----------------+            v
                                                        proposed criterion matches
                                                                  |
                                                                  v
                                                   user-owned application Decision
                                                                  |
                                                                  v
                                              user-owned Brief -> required-document plan
                                                                  |
                                                                  v
                                      guarded document-scoped structured Draft
                                            (Cover Letter or Research Statement)
                                                                  |
                                                                  v
                                                    deterministic independent Review
```

### 1. Initialize a private workspace

Normal users should install the CLI and keep private application data in a separate workspace. They do not need to fork this repository.

```bash
canisend init-workspace --workspace ~/CanISendWorkspace
canisend doctor --workspace ~/CanISendWorkspace
```

The workspace contains private profile data, job leads, job folders, editable prompt copies, Typst templates, schemas, examples, and agent-readable skills:

```text
~/CanISendWorkspace/
  canisend.yaml
  .env.example
  .gitignore
  AGENTS.md
  CLAUDE.md
  profile/
  jobs/
  job_leads/
  prompts/
  templates/
  schemas/
  examples/
    discovery/
  docs/
    stage4-migration.md
  agent-skills/
    canisend/
    canisend-job-intake/
    canisend-application-package/
    canisend-submission-readiness/
```

After upgrades:

```bash
uv tool upgrade canisend
canisend update-workspace --workspace ~/CanISendWorkspace
canisend doctor --workspace ~/CanISendWorkspace
```

`update-workspace` preserves local prompt, template, and skill edits by default. Use `--overwrite` only when packaged defaults should replace local copies.
If `doctor` reports deprecated packaged files from an older release, remove only those retired defaults with:

```bash
canisend update-workspace --workspace ~/CanISendWorkspace --prune-deprecated
```

`doctor` also reports local default resources that differ from the packaged version. Use `update-workspace --overwrite` only after deciding those local prompt, template, schema, or bridge edits should be replaced.

### 2. Prepare profile evidence

Put your real modernpro CV and statements under `~/CanISendWorkspace/profile/typst/`. These files stay local, and `profile/` is ignored by git except for `.gitkeep`.

Create starter profile files if needed:

```bash
canisend init-profile --workspace ~/CanISendWorkspace --mode typst
```

Generate normalized evidence:

```bash
canisend extract-profile-evidence --workspace ~/CanISendWorkspace
```

When local Typst extraction misses evidence, you can explicitly opt into provider-backed augmentation:

```bash
canisend extract-profile-evidence \
  --workspace ~/CanISendWorkspace \
  --llm-augment
```

The profile manifest lives at `profile/profile.yaml`. Generated evidence is written to `profile/generated/` and cited with item-level references such as:

```text
profile/generated/cv.evidence.md#Teaching/cv-001
```

Review item-level evidence citations before trusting any generated claim.

Current Typst extraction also writes a source-hash receipt into each generated evidence file. If a file was produced
by an older CanISend version and the resumable Evidence stage reports `evidence.source_receipt_missing`, rerun
`extract-profile-evidence`; if the underlying Typst source has changed, re-extraction also resolves
`evidence.source_receipt_stale`. Do not hand-edit the receipt to make stale evidence appear current.

### 3. Import leads and create one job folder

Discovery inputs create candidate leads only. They do not become complete job adverts, applicant evidence, or
application materials. Stage 4 keeps a deterministic catalog at `job_leads/catalog.json`, with stable lead IDs,
source provenance, aliases, ranking reasons, and inspectable exclusions.

Start with the packaged
[`examples/discovery/discovery-sources.example.yaml`](examples/discovery/discovery-sources.example.yaml), replace its
placeholder identifiers, and save it as `discovery-sources.yaml` in the private workspace. It can combine RSS/Atom
feeds with identifier-only, read-only Greenhouse and Lever public-job adapters:

```bash
canisend discovery refresh \
  --workspace ~/CanISendWorkspace \
  --sources discovery-sources.yaml
```

The public adapter endpoints and exclusions are documented in
[`docs/discovery-adapters.md`](docs/discovery-adapters.md). Source configuration does not accept credentials,
arbitrary public-API URLs, application endpoints, or upload behavior.

Local saved searches and email alerts can enter the same catalog without calling an API:

```bash
canisend discovery import \
  --workspace ~/CanISendWorkspace \
  --input ~/Downloads/saved-jobs.csv \
  --source-name "Saved Academic Search" \
  --include economics \
  --exclude phd
```

Supported local discovery exports are CSV, JSON, EML, and MBOX. The importer retains only normalized job fields;
raw email bodies, unknown vendor columns, credentials, local absolute paths, and connector/session metadata do not
enter catalog artifacts.

Codex, Claude, or another host can perform a public search and save only the strict
`canisend.discovery-search/v1` envelope. Import it with:

```bash
canisend discovery import-search \
  --workspace ~/CanISendWorkspace \
  --input normalized-search.json
```

The packaged
[`normalized-search.example.json`](examples/discovery/normalized-search.example.json) shows the host-neutral shape.
The host must not put provider names, session IDs, credentials, email addresses, or private paths into that file.

Existing lists can also be merged explicitly:

```bash
canisend discovery merge \
  --workspace ~/CanISendWorkspace \
  --input job_leads/jobs_ac_uk.json \
  --input job_leads/example-university.json \
  --include economics \
  --exclude phd
```

Select from the catalog by stable ID so source reordering cannot change the selected role:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --leads-file job_leads/catalog.json \
  --lead-id <lead_id> \
  --institution "Example University" \
  --deadline "2026-08-31"
```

The original jobs.ac.uk, generic RSS/Atom, and zero-based index workflows remain available for compatibility.

Fetch jobs.ac.uk RSS leads locally:

```bash
canisend fetch-jobs-ac-uk \
  --workspace ~/CanISendWorkspace \
  --feed-url "<jobs.ac.uk RSS url>" \
  --include economics \
  --exclude phd
```

Any stable RSS or Atom source can use the generic entrypoint:

```bash
canisend fetch-job-feed \
  --workspace ~/CanISendWorkspace \
  --source-name "Example University" \
  --feed-url "https://example.edu/jobs.atom" \
  --include economics \
  --exclude phd
```

Generic feeds default to `job_leads/<source-name>.json`. They use the same normalized lead fields and local filters as
jobs.ac.uk. Feed records are discovery leads, not full adverts.

Create a job folder from a selected zero-based lead index when using a legacy list:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --lead-index 0 \
  --institution "University X" \
  --deadline "2026-06-15"
```

For a generic legacy feed output, pass its lead file explicitly:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --leads-file job_leads/example-university.json \
  --lead-index 0 \
  --institution "Example University"
```

You can also create a job manually:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --english-variant "UK English" \
  --writing-style "direct, warm, evidence-led" \
  --source-url "https://www.jobs.ac.uk/job/example"
```

If these preferences are unknown, leave them unset. CanISend will mark them as `needs_confirmation` and include language/style questions in `00_preparation_questions.md`.

Paste the full advert, or import it from a supported local file, into `jobs/<job-slug>/job_advert.md` before relying on parsed criteria or generated
drafts. A user-provided PDF is a first-class intake path:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --advert-file ~/Downloads/lecturer-job.pdf
```

`--source-url` records metadata only. When the user explicitly wants one supplied URL imported, add `--fetch-url`;
the response may be HTML or PDF. This bounded one-URL import is different from crawling or searching a site:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --source-url "https://example.edu/jobs/123" \
  --fetch-url
```

See [`docs/stage4-migration.md`](docs/stage4-migration.md) for legacy list/index migration, catalog and cache
recovery, workspace update behavior, and rollback.

### 4. Generate draft materials

Run the deterministic local pipeline:

```bash
canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

Generated output includes:

```text
jobs/<job-slug>/
  parsed_job.json
  00_preparation_questions.md
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  07_material_review_checklist.md
  08_research_statement.md       # conditional: current reviewed Research Statement
  typst/
    cover_letter.typ
    application_package.typ
    research_statement.typ       # conditional standalone document
```

Generated Typst files are the editable source of truth for final formatting. Content JSON files may still be emitted as compatibility/debug artifacts, but normal edits should happen in the `.typ` files.

The Research Statement files appear only after its exact current Draft, deterministic blocker-free Review, and
user dispositions derive document readiness `reviewed`. They remain standalone: they are not embedded in the
application-package source and do not change the existing package gate.

CanISend records hashes of its generated Typst baseline. On a later `run`, an unchanged source updates normally. If a
source has been edited, the user version is preserved and the new generation is written as `*.generated.typ` for
review instead of silently overwriting the edit. Pending Cover Letter or application-package candidates block
`check-package`; every pending candidate blocks `render-typst`, and `run --git-add-materials` skips staging so
Markdown and Typst cannot be recorded as a mismatched set. A standalone Research Statement candidate stays outside
the package gate but must still be reconciled before Typst rendering.

To track edits to generated application materials in a private git repository, opt in after generation:

```bash
canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --git-add-materials
```

Without the flag, interactive terminals are asked whether to add generated application materials to git;
non-interactive runs skip git staging unless the flag is set. CanISend stages only the generated preparation
questions, fit report, cover letter draft, CV tailoring notes, criteria checklist, final package, material review
checklist, an eligible standalone Research Statement, and editable Typst sources. It does not stage raw adverts,
source URLs, parsed job JSON, compatibility JSON, PDFs, or profile files, and it never commits automatically.

LLM-backed evidence augmentation, parser, and draft generation are explicit opt-in modes. Configure a provider before using them:

```bash
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=...

canisend extract-profile-evidence \
  --workspace ~/CanISendWorkspace \
  --llm-augment

canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

For local CLI model access, use the command provider:

```bash
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

The LLM-backed evidence augmenter only accepts items tied to a supporting `source_text` found in the local profile source. The LLM-backed parser must return JSON matching the `parsed_job.json` contract. Draft outputs must cite profile evidence; unknown citations fail validation. Missing evidence should be marked as a gap, not replaced with unsupported claims.

### 5. Review, render, and submit manually

Review files in this order:

1. `parsed_job.json`
2. `criteria.json`
3. Evidence state and receipts in `evidence_catalog.json` (its body is private profile data)
4. Proposed classifications and gaps in `criterion_matches.json`
5. `00_preparation_questions.md`
6. `05_criteria_checklist.md`
7. `02_fit_report.md`
8. `03_cover_letter_draft.md`
9. `04_cv_tailoring_notes.md`
10. `07_material_review_checklist.md`
11. `08_research_statement.md` when generated from a reviewed Research Statement
12. `typst/cover_letter.typ`
13. `typst/research_statement.typ` when present
14. `typst/application_package.typ`
15. `06_final_application_package.md`

Use `00_preparation_questions.md` to confirm US English vs UK English, writing style, and the grill-me details that make the application specific. Use `07_material_review_checklist.md` to track the cover letter draft, CV tailoring notes, placeholders, item-level evidence citation checks, and next manual actions.

After reviewing the Markdown drafts, directly edit `typst/cover_letter.typ` and the other applicable `.typ` sources
for final wording and layout. The files include stable `// CANISEND: section ...` or Claim markers so agents can make
bounded edits without rewriting the whole Typst source.

Run a read-only package check before treating generated materials as ready for user review:

```bash
canisend check-package \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

The check reports incomplete advert or parsed-job inputs, missing or structurally incomplete Typst sources, explicit
review blockers, unresolved bracketed placeholders, unknown profile evidence citations, and APP-Q5 aggregate receipt
failures. A legacy package remains readable but cannot pass without current aggregate Review and package decisions.
The command does not modify files unless an explicit machine-readable gate report is requested:

```bash
canisend check-package \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --write-report
```

The report records safe relative input labels and SHA-256 hashes. A later `canisend run` marks an existing report
`STALE`, so it cannot be mistaken for a check of regenerated materials.

Render Typst only when needed:

```bash
canisend render-typst \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not. Submit manually through the institution portal outside this tool.

## Agent Usage

Codex, Claude Code, and IDE agents can run the `canisend` workflow by opening the private workspace as the project root. This is an agent-assisted workflow, not a local-only workflow: any file, PDF, webpage, or generated material the agent reads or summarizes may be processed by the agent model provider.

- Codex and AGENTS.md-aware tools should read `AGENTS.md`.
- Claude Code should read `CLAUDE.md`, which imports `agent-skills/canisend/SKILL.md`.
- IDE agents can read `AGENTS.md` and then `agent-skills/canisend/SKILL.md`.

Agents should start with:

```bash
canisend agent context --workspace . --format json
```

Inspect the versioned runtime contract before choosing an operation:

```bash
canisend agent capabilities --format json
```

The contract foundation provides JSON output for `doctor`, `new-job`, `new-job-from-lead`, `list-jobs`, and
`check-package`. Each
successful invocation writes one `canisend.agent/v1` response to stdout. Responses use workspace-relative or opaque
artifact references and hashes rather than private document bodies. A failed package gate remains a successful
operation result (`ok: true`) but exits non-zero; operational failures return `ok: false` with a stable error code.

The locally accepted Stage 2 resumable workflow exposes Parse, Confirm, Evidence, Match, guarded user-owned mutation,
Brief, document planning, and compatible structured views through the same shell-capable hosts. Start
a deterministic fresh session with:

```bash
canisend agent context --workspace . --job jobs/<job-slug> --format json
canisend stage status --workspace . --job jobs/<job-slug> --format json
canisend extract-profile-evidence --workspace .
canisend stage run --workspace . --job jobs/<job-slug> --stage evidence --mode deterministic --format json
canisend stage run --workspace . --job jobs/<job-slug> --stage parse --mode deterministic --format json
canisend stage run --workspace . --job jobs/<job-slug> --stage confirm --mode deterministic --format json
canisend corrections status --workspace . --job jobs/<job-slug> --format json
canisend corrections init --workspace . --job jobs/<job-slug> --confirm-user-owned-write --format json
canisend corrections status --workspace . --job jobs/<job-slug> --format json
```

`status` is read-only. Record the current Tier 2 artifact `sha256` and the
`canisend.user_artifact_revision` extension. Put exactly one supported operation in a bounded strict YAML or JSON
patch file—do not ask an agent to replace `confirmed_corrections.yaml` directly:

```yaml
operation: confirm_criterion
criterion_id: criterion_<32-lowercase-hex>
```

Accept that one scoped update and immediately refresh Confirm:

```bash
canisend corrections update \
  --workspace . \
  --job jobs/<job-slug> \
  --patch-file /safe/scratch/correction-patch.yaml \
  --expected-revision <revision-from-status> \
  --expected-sha256 <sha256-from-status> \
  --confirm-user-owned-write \
  --format json
canisend stage run --workspace . --job jobs/<job-slug> --stage confirm --mode deterministic --format json
```

Every semantic corrections patch requires current Parse and Confirm. Empty initialization has no active correction
and is fingerprint-neutral; each accepted semantic update makes Confirm stale, so rerun Confirm before
`corrections status` and the next patch. Do not batch corrections against one catalog baseline.
Supported operations are `confirm_criterion`, `correct_criterion`, `withdraw_criterion`, and `confirm_empty`. An
extraction with no criteria remains unknown until the user explicitly chooses `confirm_empty`; initializing an empty
corrections record does not mean `confirmed_empty`.

When Criteria and Evidence are current, propose matches and record the user's separate Decision:

```bash
canisend stage run --workspace . --job jobs/<job-slug> --stage match --mode deterministic --format json
canisend decision status --workspace . --job jobs/<job-slug> --format json
canisend decision init --workspace . --job jobs/<job-slug> --confirm-user-owned-write --format json
canisend decision status --workspace . --job jobs/<job-slug> --format json
```

Create one strict patch, for example:

```yaml
operation: set_decision
decision: apply
rationale_mode: keep
```

Apply it with `decision update`, passing `--patch-file`, the revision/hash returned by the latest `decision status`,
and `--confirm-user-owned-write`. `undecided` is not an implicit apply, hold, or skip. Match remains advisory and
cannot write this Decision.

If a later Parse, Confirm, Evidence, or Match refresh changes the Decision basis, the accepted value and raw
`application_decision.yaml` bytes remain present. `decision status` derives
`canisend.decision_basis_status=review_required`; inspect the new basis, then use a new `set_decision` patch (including
the same value when reconfirming it) with the latest revision/hash. Staleness is never written into the user YAML.
If an accepted mutation reports recovery required or a receipt-pending state, recover only its opaque ID:

```bash
canisend user-mutation recover \
  --workspace . \
  --job jobs/<job-slug> \
  --mutation-id mutation_<32-lowercase-hex> \
  --confirm-user-owned-write \
  --format json
```

Fresh status also detects a process interruption between publishing a complete private target link and removing
CanISend's temporary link. It remains read-only and routes the same opaque mutation ID to explicit recovery; ordinary
hard links are still rejected.

Brief work starts only from a current confirmed `decision=apply`. Brief status is body-free; initialization and every
scoped change use the same explicit-consent revision/hash CAS boundary as corrections and Decision:

```bash
canisend brief status --workspace . --job jobs/<job-slug> --format json
canisend brief init --workspace . --job jobs/<job-slug> --confirm-user-owned-write --format json
canisend brief status --workspace . --job jobs/<job-slug> --format json
```

Use `brief update` with one bounded strict patch file, the latest revision/hash, and
`--confirm-user-owned-write`; never replace `application_brief.yaml` directly. Language, writing style, motivation,
emphasis, exclusions, the advert-document requirement-set confirmation, and document choices remain field-level user
decisions. An empty Parsed Job document list is `unconfirmed`, not `confirmed_empty`; only an explicit
`confirm_document_requirements` patch may record `confirmed_empty` against the current requirements-basis hash.
For a non-empty set, every extracted item must resolve to one complete, positive advert document member. Missing,
ambiguous, conditional, alternative, qualified, truncated, or unreconciled source context remains `unconfirmed`; its
section/item anchor is part of the basis, so a source move or change requires a new confirmation.

Generate the current core-owned plan deterministically after the Brief exists:

```bash
canisend stage run --workspace . --job jobs/<job-slug> --stage brief --mode deterministic --format json
```

The plan records body-free status counts and blocker codes in AgentResponse. Its Tier 2 body remains ask-first for an
agent because it can contain advert source text and application strategy. An unconfirmed requirement set, unresolved
choice, `required + omit`, required document without a preparation action, or orphaned old choice blocks later
Draft/Verify work. Stage 2 is locally accepted, but these artifacts alone do not establish Draft,
application-package, or submission readiness.

Derive the current multi-document fan-out without reading document bodies:

```bash
canisend documents status --workspace . --job jobs/<job-slug> --format json
```

The response binds the exact plan hash and reports aggregate `ready`, `partially_dispatchable`, `blocked`, or
`no_work` state. Cover Letter and Research Statement are currently available guarded executors. Confirmed teaching,
supporting, diversity, publication, CV, and other routes remain explicit `executor_unavailable` work until their own
structured schemas, validators, and promotion paths are implemented; they are never silently treated as complete.

Draft and Review execution state is owned by the composite `(stage, document_id)` identity from the current Required
Document Plan. One current dispatchable target is resolved automatically for compatibility. When both Cover Letter
and Research Statement are ready, the host must obtain their stable IDs from the Tier 2 plan after approval and pass
the selected `--document-id <document_...>` on `stage status`, `prepare`, `run`, or `cancel`; an omitted ambiguous,
mismatched, malformed, or unsupported target fails before a task is created.

When the plan contains a blocker-free confirmed `prepare` Cover Letter or Research Statement, a host agent can enter
the guarded structured Draft path without starting a second model call (include `--document-id` when required):

```bash
canisend stage prepare \
  --workspace . \
  --job jobs/<job-slug> \
  --stage draft \
  --mode host-agent \
  --format json
```

After the user approves the returned `read-private-draft-inputs` consent, the agent reads only the TaskSpec-declared
Tier 2 inputs and writes schema-valid candidate JSON to fresh private scratch. It passes that file to `stage submit`
with the returned `--task` path, then passes the immutable TaskResult from the response to `stage apply`; neither the
agent nor a provider writes the run directory, `cover_letter_draft.json`, or `research_statement_draft.json`
directly. Research Statement candidates validate against `research-statement-draft.schema.json` and require
`research_overview`, `research_contributions`, and `future_agenda` sections before Review is blocker-free.

For Cover Letter only, after explicit Tier 3 approval to transmit the seven declared Draft inputs, the same
structured path can call the configured provider and complete prepare, candidate submission, validation, and
promotion in one command. Research Statement configured-provider execution fails closed as unsupported:

```bash
canisend stage run \
  --workspace . \
  --job jobs/<job-slug> \
  --stage draft \
  --mode configured-provider \
  --allow-provider-backed \
  --format json
```

The consent flag is required on each non-cached invocation. The provider receives only `parsed_job.json`,
`criteria.json`, `evidence_catalog.json`, `criterion_matches.json`, `application_decision.yaml`,
`application_brief.yaml`, and `required_document_plan.json`. It proposes sections and Claim semantics only; the core
derives identity, current-basis receipts, stable Claim IDs, generation metadata, and review state. Raw provider output
is bounded and is not stored. Failure, invalid output, or input drift promotes nothing, while an already submitted
candidate resumes without another provider call. Legacy `canisend run --llm-drafts` remains a separate compatibility
path.

Review the promoted Draft independently and deterministically:

```bash
canisend stage run \
  --workspace . \
  --job jobs/<job-slug> \
  --stage review \
  --mode deterministic \
  --format json
```

When `--document-id` was supplied for Draft, pass the same ID to Review. Review resolves only the Draft instance for
that document; cache, retry, failure, and recovery records for another document cannot satisfy it.

Unsupported factual claims, document-specific missing sections, and detectable Brief-exclusion conflicts remain
blockers. Partial support, semantic support, and non-factual Claim-kind classification remain review work. For
either Cover Letter or Research Statement, use body-free status with the same stable document ID, then initialize
and disposition one finding at a time with the current revision/hash:

```bash
canisend review-dispositions status --workspace . --job jobs/<job-slug> --document-id <document_...> --format json
canisend review-dispositions init --workspace . --job jobs/<job-slug> --document-id <document_...> \
  --confirm-user-owned-write --format json
canisend review-dispositions update --workspace . --job jobs/<job-slug> --patch-file <patch.yaml> \
  --document-id <document_...> --expected-revision <revision> --expected-sha256 <sha256> \
  --confirm-user-owned-write --format json
```

The option may be omitted only when the current plan has exactly one supported prepared document. Cover Letter keeps
`review_dispositions.yaml`; Research Statement uses the independent
`research_statement_review_dispositions.yaml`. `accepted` is valid only for non-blockers;
`revision_required` keeps that document in review. A changed Draft/Review preserves the old decisions and requires
`reset_for_current_review`. When every current non-blocker finding is accepted, that document derives `reviewed`
while its Draft and Review stay `proposed`. Cover Letter readiness feeds its compatibility views and existing package
checks. Research Statement readiness may feed only a standalone Markdown/Typst view; it is not embedded in the
application package and does not affect package readiness.

After dispositioning the selected documents, run aggregate consistency Review without selecting a document ID:

```bash
canisend stage run \
  --workspace . \
  --job jobs/<job-slug> \
  --stage package_review \
  --mode deterministic \
  --format json
```

`package_review_findings.json` binds the current Parsed Job, Brief, Required Document Plan, derived execution plan,
and every observed Draft/Review/disposition/readiness receipt. The body-free response reports counts and reason
codes. Required omitted, unavailable, missing, stale, blocked, unreviewed, or revision-required documents block the
aggregate result. An exact repeated factual assertion with different Evidence receipts also blocks; semantic
support, proportionality, tone, and narrative alignment are never guessed. Correction proposals target one document
and Claim set and may be applied only through a new guarded Draft candidate. This Review is not package readiness or
submission approval.

Inspect aggregate readiness without exposing finding bodies, then initialize or change the independent user-owned
package decisions with explicit consent and the latest revision/hash:

```bash
canisend package-review status --workspace . --job jobs/<job-slug> --format json
canisend package-review init --workspace . --job jobs/<job-slug> \
  --confirm-user-owned-write --format json
canisend package-review update --workspace . --job jobs/<job-slug> --patch-file <patch.yaml> \
  --expected-revision <revision> --expected-sha256 <sha256> \
  --confirm-user-owned-write --format json
```

Use `set_package_finding_disposition` or `clear_package_finding_disposition` for one current finding; use
`reset_for_current_package_review` after the aggregate Review changes. Blockers cannot be accepted. Only complete
current decisions over individually reviewed required documents derive application-package `reviewed`; that state
still does not mean rendered, submitted, or received.

Evidence and Parse are independent after intake, so their deterministic runs may be ordered either way; Match waits
for current Confirm and Evidence outputs. Host-agent execution applies to Parse and Draft, while configured-provider
execution currently applies only to Draft; Evidence, Confirm, Match, Brief, Review, and Package Review are
deterministic-only. For
current-host Parse or Draft reasoning,
`stage prepare --mode host-agent` writes a TaskSpec plus an immutable preparation receipt under the job's
`workflow/runs/` directory. Non-document stages retain the frozen 1.0 wire shape; document-scoped Draft/Review
records use backward-readable control-contract 1.1 and carry the stable `document_id` through result, validation,
manifest, terminal claim, promotion, state, CLI, and AgentResponse. After explicit approval to read the full advert,
the host creates candidate JSON only in a fresh scratch file and passes it to `stage submit --candidate-file`; the
guarded service writes the declared
candidate, TaskResult, and submission receipt without following symlink or hard-link aliases. `stage apply` then
rechecks task integrity, active status, input freshness, scope, hashes, schema, source receipts, and output drift
before atomically promoting `parsed_job.json`. Agents never write a run path or authoritative stage artifact directly.
If inputs or dependencies change while a task is active, run
`canisend stage cancel --workspace . --job jobs/<job-slug> --stage <stage> [--document-id <document_...>]` before
preparing a replacement; audit
records and any candidate remain available.

Confirm projects Parsed Job v1 into `criteria.json` with stable criterion IDs, confidence, and separate
source/confirmation states. A resolved source has one span; a missing source stays unknown, while an ambiguous source
exposes candidate spans without choosing one. It reports `review_required` for unconfirmed or source-unknown
criteria, orphaned corrections, and an empty extraction that has not been explicitly confirmed. An optional
`confirmed_corrections.yaml` is user-owned: Confirm reads it, while neither Parse nor Confirm creates, rewrites, or
deletes it. Prefer `corrections status|init|update` for agent-assisted writes; these accept one discriminated patch,
explicit consent, and the current revision/hash baseline. Direct manual YAML edits are still valid and must follow
`schemas/confirmed-corrections.schema.json`; status and stage reruns validate without normalizing or rewriting the
file. An explicitly consented scoped update creates the canonical next revision and may not preserve YAML comments.
A changed source receipt or parsed
interpretation receives a new criterion ID and leaves the old correction visible for reconciliation instead of
attaching it to a different requirement.

Evidence prepares one immutable private input at
`workflow/runs/<run-id>/inputs/evidence-snapshot.json`, then promotes `evidence_catalog.json`. The snapshot, Evidence
candidate, and promoted catalog may contain normalized profile text and deliberately duplicate it inside the ignored
private job data plane. They remain until the user removes the private run or job folder; CanISend does not claim to
erase them automatically. TaskSpec, state, receipts, manifests, errors, ordinary command output, and AgentResponse
extensions contain only privacy-safe paths, hashes, IDs, reason codes, and counts. Resumable Evidence rejects
workspace-external profile roots, path escapes, symlinks, hard-link aliases, non-regular files, and bounded-input
violations. Legacy commands may retain their existing external-profile behavior, but that does not expand the
Evidence TaskSpec v1 boundary.

Match reads only the current job-local `criteria.json` and `evidence_catalog.json`. It writes
`criterion_matches.json` with one deterministic `strong`, `partial`, `weak`, `missing`, or `unknown` proposal per
criterion, explicit privacy-safe gaps, matcher provenance, and opaque catalog references. It does not copy evidence
text, generated-evidence headings, or legacy locators. Every record has `review_state=proposed`: Match is not an
application decision, does not confirm applicant claims, and does not make a package ready. The user-owned Decision
is managed separately through `decision status|init|update`; Brief uses `brief status|init|update`, Review decisions
use `review-dispositions status|init|update`, and deterministic
Brief-stage planning writes `required_document_plan.json`. For a current deterministic Match using the configured
workspace profile, `canisend run` now projects the same proposal graph into `02_fit_report.md`,
`05_criteria_checklist.md`, structured HR checks, and Typst package content. If the current structured Draft and
deterministic Review also validate and Review has no blocker findings, the same run projects each Cover Letter Claim
once into `03_cover_letter_draft.md`, Cover Letter/package content JSON, and Typst. Complete exact dispositions make
that document reviewed; stale/drifted state, a different
parsed view, a profile override, blocked/missing Draft or Review, a direct library call without workspace provenance,
or `--llm-drafts` keeps the compatible fallback path. Match, Draft, and Review remain proposed; document readiness is
derived and never becomes a Decision or whole-package result.

When the Research Statement Draft and Review pass the same currentness checks and every non-blocker finding is
accepted, the run also projects each Research Claim once into conditional standalone Markdown and Typst files. The
projection binds exact Draft, Review, disposition, readiness, and Markdown hashes. It is deliberately absent from
the application-package source and `check-package` inputs. If a prior Research projection becomes ineligible, a
rerun replaces generated views with a body-free unavailable state; an edited Typst primary is preserved and receives
a candidate for explicit reconciliation.

Use `canisend doctor --workspace .` when a human-readable environment diagnostic is also useful.

A fresh Codex, Claude Code, or IDE shell session resumes from the same durable workspace state by running the same
`agent context` command; it does not need the previous chat transcript. The fake-data conformance fixture under
`examples/agent_handoff/` demonstrates this host-neutral handoff. Parse and Draft use durable task/result exchange
through the existing CLI; Confirm, Evidence, Match, required-document planning, and Review reuse deterministic
runtime paths, while corrections, Decision, and Brief use separate user-owned mutation operations. None requires a
platform-specific API, network, or MCP transport; deterministic stages need no configured provider.

They may run local deterministic CLI commands, inspect generated evidence, and review current job artifacts. They must
ask first before reading full private CVs, statements, full job adverts, references, PDFs, source URLs, Evidence
snapshots/candidates/catalogs, `criteria.json`, `criterion_matches.json`, `application_brief.yaml`,
`required_document_plan.json`, either structured Draft/Review, `package_review_findings.json`, `review_dispositions.yaml`,
`research_statement_review_dispositions.yaml`, generated packages, or enabling
LLM-backed CLI flags/providers. Criteria may contain corrected wording; Match, Brief, the document plan, Draft, and
Review remain Tier 2. They must not
scrape pages, submit applications, upload packages, fabricate evidence, or commit private profile/job data.

Original profile inputs under `profile/` are protected. Agents should normally produce profile-improvement suggestions inside the job folder, not rewrite the source CV or statements. A write back to `profile/typst/*.typ`, `profile/*.md`, or other non-generated profile input is allowed only through an orchestrator task that declares `edits_profile_input: true`, depends on a prior review task, uses privacy tier 2 or higher, and is launched with `--allow-profile-input-edits --confirm-profile-input-edit --confirm-profile-input-edit-again`.

For coordinated multi-CLI review, use `canisend orchestrate` with an explicit local YAML plan:

```bash
canisend orchestrate \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --plan orchestration.yaml \
  --dry-run
```

Worker entries declare either `kind` or `command`. Built-in `kind` values are `codex`, `claude`, `antigravity`, `agy`, and `custom`; `antigravity` and `agy` share the same default command. Use `command` to override the preset for the CLI installed on your machine, and use `prompt_mode` to choose how the generated task prompt is delivered: `stdin`, `arg`, or `none`.

```yaml
workers:
  codex-reviewer:
    kind: codex
  claude-reviewer:
    kind: claude
    prompt_mode: arg
  agy-reviewer:
    kind: agy
    command: "agy --print"
```

Worker entries also support `max_parallel_tasks`, `supports_native_subagents`, and `privacy_tier_limit`. Task entries declare `role`, `inputs`, `outputs`, `writes`, `depends_on`, `privacy_tier`, optional `agent_count`, and optional `edits_profile_input` for tightly controlled original profile source edits. The orchestrator runs dependency-ready tasks in parallel, enforces worker concurrency limits, writes run artifacts under `jobs/<job-slug>/orchestration/runs/`, and requires `--allow-private-sources`, `--allow-provider-backed`, or the profile-input edit confirmation flags for higher-risk tasks.

Privacy modes:

- Direct CLI deterministic mode can be local-only when no agent reads private content and no LLM flags/providers are used.
- Agent-assisted mode means agent-read content may enter the agent model context.
- LLM-backed CLI mode means selected context may be sent to the configured provider or local command through
  `stage run --stage draft --mode configured-provider --allow-provider-backed` or flags such as
  `extract-profile-evidence --llm-augment`, `--llm-parser`, or `--llm-drafts`.

Detailed agent guidance lives in:

```text
agent-skills/canisend/
  SKILL.md
  references/
    workflow.md
    job-lifecycle.md
    file-contracts.md
    typst-profile.md
    provider-config.md
    quality-gates.md
    agent-orchestration.md
    platforms.md
    privacy.md
```

`prompts/` contains LLM prompt files used by the application pipeline. Workspace-local `agent-skills/` contains the
complete main and focused skill pack, so routes from the main skill do not depend on a global installation.

## Skill Distribution

The project also ships a reusable skill pack for cases where you want a narrower agent behavior without opening a
CanISend workspace as the active project. `skills/` is the canonical pack used by exports and workspace updates. The
root Codex plugin manifest at `.codex-plugin/plugin.json` exposes that pack, while `agent-skills/canisend/` remains a
one-release compatibility mirror of the canonical main skill.

The repository-native substitute for an external `sync_hermes_tap` contract is
`python scripts/sync_workspace_skill_mirror.py --check`. CI and local release checks fail on missing, extra, or
content-drifted mirror entries; run the script without `--check` to safely rebuild the compatibility mirror.

Use the main `canisend` skill for full workspace workflows. Use `canisend-job-intake` for source-to-advert intake,
`canisend-application-package` for coordinated package construction, and `canisend-submission-readiness` for the final
manual-submission gate. Material-focused skills cover job fit, research and teaching statements, cover letters, CV
tailoring, humanization, application email, interview preparation, criteria checks, and material review.

The material-focused skill IDs are `canisend-job-fit`, `canisend-research-statement`,
`canisend-teaching-statement`, `canisend-cover-letter`, `canisend-cv-tailoring`, `canisend-humanizer`,
`canisend-application-email`, `canisend-interview-prep`, `canisend-criteria-check`, and `canisend-material-review`.

For a Codex marketplace repository, mount this repository as the plugin source:

```bash
git submodule add https://github.com/jxpeng98/CanISend plugins/canisend
```

Then add a marketplace entry that points to `./plugins/canisend`.

To export skills from an installed package instead of a checkout:

```bash
canisend export-skills --target ~/plugins/canisend --kind codex-plugin
canisend export-skills --target ~/.claude/skills --kind skills-only
```

`codex-plugin` writes `.codex-plugin/` plus `skills/`. `skills-only` writes only the skill folders for agents that install skills directly.

## Privacy Boundaries

This repository is intended to be open source. Personal application data should stay local:

- `profile/` is ignored by git except for `.gitkeep`.
- `jobs/` generated job folders are ignored by git unless selected generated materials are explicitly staged with `canisend run --git-add-materials`.
- `job_leads/` feed outputs are ignored by git.
- `.env`, API keys, rendered PDFs, raw job adverts, real source URLs, parsed job JSON, and profile files should not be committed.
- Evidence snapshots, Evidence candidates, `evidence_catalog.json`, and `criterion_matches.json` stay inside ignored
  private job folders. The first three may contain duplicated normalized profile text; removing a run or job remains
  an explicit user retention decision.
- `criteria.json`, `criterion_matches.json`, `confirmed_corrections.yaml`, `application_decision.yaml`,
  `application_brief.yaml`, `required_document_plan.json`, both structured Draft/Review pairs,
  `review_dispositions.yaml`, `research_statement_review_dispositions.yaml`, `package_review_findings.json`,
  `package_review_dispositions.yaml`, and
  private mutation/stage candidates are Tier 2. Criteria may contain corrected wording; Match is body-minimized,
  while Brief, plan, Draft, Review, and dispositions may reveal private motivation, exclusions, source text, application strategy,
  or prose. Mutation receipts are Tier 1 and contain none of those bodies; neither do workflow control records,
  errors, ordinary command output, or AgentResponse.
- User YAML may be edited manually. Status and stage reruns do not normalize it; an explicitly consented scoped
  update creates a canonical next revision and may not preserve comments. Revision/hash CAS coordinates cooperative
  CanISend writers in a stable job directory; it does not linearize a normal editor save in the final replace window
  or a hostile same-user rename. Run status immediately before changing a file and avoid concurrent manual saves.
- Resetting/clearing the current Decision or Brief field, removing a document choice, or withdrawing a correction is
  a semantic update, not disk erasure.
  Historical corrections and private-mode Tier 2 mutation candidates (0600 on POSIX) deliberately retain old bodies for audit
  and recovery. Keep job folders private and git-ignored, include them in backups only intentionally, and delete the
  relevant private mutation events or whole job when retention is no longer wanted. CanISend does not currently
  promise automatic secure erasure, including from backups or filesystem snapshots.
- Sensitive declarations such as right-to-work, visa, disability, equality monitoring, health, criminal record, and conflicts remain user-only.

这也能投只是材料准备工具，不是提交凭证。

## Maintainer Release

Release automation lives in GitHub Actions for `jxpeng98/CanISend`.

Local checks:

```bash
uv run python -m pytest -v
uv build
uvx twine check dist/*
uv run python -m canisend.package_check dist/*.whl
```

CI runs the same test/build/resource-check sequence on pushes and pull requests. The release workflow uses PyPI Trusted Publishing with OIDC:

- Pushing `test/v<version>` publishes to TestPyPI only.
- Pushing `v<version>bN` or `v<version>rcN` publishes to TestPyPI, smoke-tests the TestPyPI package, then publishes a PyPI prerelease and creates a GitHub prerelease.
- Pushing `v<version>` publishes to TestPyPI, smoke-tests the TestPyPI package, then publishes a stable PyPI release and creates a GitHub Release.
- TestPyPI and PyPI need a Trusted Publisher for `.github/workflows/release.yml` with environments named `testpypi` and `pypi`.

Preferred tag-driven release orchestration:

```bash
scripts/release.sh test --version 0.3.0.dev2
scripts/release.sh beta --version 0.6.0b1
scripts/release.sh stable --version 0.6.0
```

The script updates `pyproject.toml` and `src/canisend/__init__.py`, runs local checks, commits the version bump, pushes the current branch, then creates and pushes the matching git tag:

```bash
git tag -a test/v0.3.0.dev2 HEAD -m "canisend 0.3.0.dev2 TestPyPI"
git tag -a v0.6.0b1 HEAD -m "canisend 0.6.0b1 prerelease"
git tag -a v0.6.0 HEAD -m "canisend 0.6.0 stable"
```

`release.yml` is the only remote publisher. It always publishes to TestPyPI first, and only promotes `v*` tags to
prerelease or stable PyPI after the TestPyPI publish and smoke test succeed. The release script verifies the candidate
commit on its remote branch before tagging; stable releases must be reachable from `origin/main`. Published tags,
including `v0.2.0`, are never reused.
Use `test/v*` with a disposable version because TestPyPI package versions cannot be overwritten.

Use `RELEASE.md` for the full TestPyPI and PyPI release playbook. Version updates must change both `pyproject.toml` and `src/canisend/__init__.py`.

## Repository Layout

```text
src/canisend/             CLI and application pipeline
prompts/                  LLM prompt templates
templates/typst/          modernpro Typst templates
schemas/                  JSON schema contracts
agent-skills/             one-release compatibility mirror of the main skill
skills/                   canonical reusable and workspace-installed skill pack
.codex-plugin/            Codex plugin manifest for the skill pack
platform-bridges/         AGENTS.md and CLAUDE.md workspace bridges
examples/end_to_end/      fully local fake-data workflow
examples/agent_handoff/   host-neutral agent contract fixtures
examples/discovery/       public synthetic Stage 4 discovery fixtures
tests/                    CLI, pipeline, packaging, release, and contract tests
assets/                   project logo and README media
RELEASE.md                maintainer release playbook
docs/stage3-migration.md  Stage 3 workspace upgrade and recovery guide
docs/stage4-migration.md  Stage 4 discovery upgrade and rollback guide
```

See `canisend_v1_proposal.md` for the original V1 engineering proposal,
`docs/superpowers/specs/2026-07-09-discovery-and-workflow-v2-design.md` for detailed multi-source and stage-hardening
constraints, and `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md` for the current
delivery roadmap. The current Stage 4 execution plan is
`docs/superpowers/plans/2026-07-15-stage4-discovery-ecosystem.md`; the Stage 3 Draft, Stage 2 Decision Spine, and Stage
1 Agent Runtime plans remain accepted records. Existing private workspaces should follow
`docs/stage4-migration.md` before first use with the Stage 4 beta.
