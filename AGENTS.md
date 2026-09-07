# CanISend Repository Instructions

## Project control

- Read the [project control guide](.trellis/spec/guides/project-control.md) and the relevant
  [engineering guidelines](.trellis/spec/backend/index.md) before changing their layer.
- Accepted ADRs own decisions; the [1.0 roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md)
  owns work ordering; machine records and exact artifacts own release facts.
- Work directly on the authorized outcome. Reuse the existing plan and record scope, acceptance,
  evidence, and the next step there; no Trellis task creation, phase activation, agent dispatch,
  journal, or bookkeeping commit is required.
- Preserve existing `.trellis/tasks/`, `.trellis/spec/`, and `.trellis/workspace/` material as
  ordinary project documents. Their directory names and task status fields do not control execution.
- Project-local Trellis skills and hooks were removed by owner request. Do not run `trellis init`
  or `trellis update`, regenerate those adapters, or require a Trellis skill unless the owner
  explicitly requests reinstalling that workflow.
- Ask only for missing decisions that change the outcome, or actions outside existing authority.
  Do not repeatedly ask to plan, implement, check, or continue already authorized work.
- Default to local development and local integration. Reuse the current branch; do not create a
  branch, worktree or PR for each task. Create isolation only for conflicting parallel work or
  a risky experiment. Merge completed branches locally, preferring fast-forward when possible.
- Use meaningful local commits and milestone checks. Create a PR only when the owner requests
  external review or the remote integration policy requires one; batch a coherent milestone.
- Keep each change independently reviewable. Record actual checks and remaining gates; never
  equate local implementation with protected CI, qualified artifacts, or user validation.

## Product scope

CanISend is a local-first Rust framework for preparing evidence-bound applications and
submissions. Its domain-neutral kernel enforces evidence, consent, review, export, recovery, and
audit invariants; declarative workflow packs provide domain vocabulary, stages, Deliverables,
templates, and validators. The academic-job journey is the first built-in reference pack, not the
kernel's ontology. Work in this repository is ordinary product engineering, release engineering,
data-integrity testing, and defensive software assurance over code and infrastructure owned by
this project.

## Defensive assurance boundary

Security-adjacent work is limited to protecting CanISend and its users:

- dependency, license, artifact, signature, and provenance verification;
- bounded parsing of user-supplied URL, HTML, PDF, JSON, CSV, and text inputs;
- privacy/consent, path, workspace integrity, backup, recovery, and concurrency controls;
- regression, property, fault-injection, and fuzz testing of repository-owned code;
- private vulnerability reporting and release-blocker triage.

Do not turn these tasks into instructions for accessing third-party systems, acquiring credentials, evading platform
safeguards, deploying payloads, exploiting public targets, persistence, exfiltration, or destructive testing. Never
weaken a product control merely to avoid an agent or platform safety warning.

## Task routing

- Describe normal Rust build, parser, database, CLI, documentation, and release work as software engineering.
- For assurance work, state the owned component, defensive invariant, bounded local fixture, and expected test.
- Prefer precise phrases such as `malformed-input regression`, `URL destination policy`, `artifact verification`,
  `dependency advisory check`, and `release integrity` over the broad label `cybersecurity`.
- Keep extended fuzzing and dependency assurance separate from the fast edit/test loop. Use their scheduled workflows
  unless a focused local reproduction is required.
- If Codex or another host raises a safety classification, narrow the task to the repository-owned defensive outcome;
  do not ask the host to disable, downgrade, or bypass its safety policy.

## Verification tiers

Use the smallest tier that proves the change, then rely on the scheduled/native gates for their owned scope:

1. Focused: one test or smoke at the lowest layer that owns the changed invariant; add formatting and relevant Clippy
   only when Rust source changed. Documentation-only changes do not run Rust tests.
2. Source gate: `cargo run -p xtask --locked -- source check` plus Fast CI; no human Host or release-evidence refresh.
3. Native release: `cargo run -p xtask --locked -- release check` and exact packaged-binary qualification.
4. Extended assurance: scheduled fuzzing, dependency advisory/license checks, signing, notarization, Authenticode,
   provenance, package-manager lifecycle, and clean-tag release qualification.

Do not run a higher tier merely to repeat a passing lower-tier assertion. Batch Tier 2 at meaningful local integration
milestones for shared contracts, release metadata, CI, or multi-crate behavior; do not require a PR to run it. Fast CI
owns the complete remote workspace suite. Run Tier 3
only for packaging/runtime changes or an exact release candidate. Trust-boundary, consent, data-loss, recovery, and
release-integrity changes still require their smallest positive and negative regression at the owning layer.

Human Host acceptance and exact release records gate the affected candidate, not ordinary implementation or
the next independent development slice. Use synthetic confirmation responses only in isolated automated tests;
never answer a real user's product consent form. Do not request another approval to continue authorized development.
