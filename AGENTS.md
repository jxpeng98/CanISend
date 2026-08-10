<!-- TRELLIS:START -->
# Trellis Instructions

This project uses Trellis for current-task execution and cross-session memory. Product and release
truth still belongs to the authorities named in `.trellis/spec/guides/project-control.md`.

- Read `.trellis/workflow.md` before creating or advancing a Trellis task.
- Read the relevant `.trellis/spec/` files before changing their layer.
- Keep active PRDs and research under `.trellis/tasks/`; archive completed work through Trellis.
- Use `.agents/skills/` and `.codex/agents/` only as generated Trellis platform adapters.

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a
future `trellis update`.
<!-- TRELLIS:END -->

# CanISend Repository Instructions

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
2. Source gate: `cargo run -p xtask --locked -- release check` plus the fast workspace CI.
3. Native release: exact packaged-binary matrices on the five supported targets.
4. Extended assurance: scheduled fuzzing, dependency advisory/license checks, signing, notarization, Authenticode,
   provenance, package-manager lifecycle, and clean-tag release qualification.

Do not run a higher tier merely to repeat a passing lower-tier assertion. Run Tier 2 once on the final PR head for
shared contracts, release metadata, CI, or multi-crate behavior; Fast CI owns the complete workspace suite. Run Tier 3
only for packaging/runtime changes or an exact release candidate. Trust-boundary, consent, data-loss, recovery, and
release-integrity changes still require their smallest positive and negative regression at the owning layer.
