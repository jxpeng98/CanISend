# CanISend Repository Instructions

## Product scope

CanISend is a local-first Rust application for preparing academic job applications. Work in this repository is
ordinary product engineering, release engineering, data-integrity testing, and defensive software assurance over
code and infrastructure owned by this project.

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

1. Focused: affected crate tests, formatter, and relevant Clippy target.
2. Source gate: `cargo run -p xtask --locked -- release check` plus the fast workspace CI.
3. Native release: exact packaged-binary matrices on the five supported targets.
4. Extended assurance: scheduled fuzzing, dependency advisory/license checks, signing, notarization, Authenticode,
   provenance, package-manager lifecycle, and clean-tag release qualification.
