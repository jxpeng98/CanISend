# Stage 4E documents and delivery evidence

**Date:** 2026-07-27

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4E document, review, package, projection-recovery, and render GUI parity

**Qualification status:** Complete

## Outcome

Stage 4E implements four GUI operation families through the same Rust application and store
boundaries used by the CLI:

- `document.*`;
- `review.*`;
- `package.*`; and
- `render.*`.

The selected-job Documents view loads exact structured drafts only after explicit private-read
consent. Review & export keeps deterministic findings read-only, records explicit human
dispositions, computes body-free package readiness, exports only to safe job-scoped paths, and
never submits an application. Managed projection edits require a user-invoked reconcile followed
by an explicit replace or preserve-copy-then-replace action. The embedded renderer reads
authoritative structured artifacts, validates bounded PDFs, and requires a separate private export
consent.

The parity authority is now `33 implemented` and `2 deferred-beta`. Only these four fully
qualified families moved to `implemented`; schema and resource operations remain explicitly
deferred to Stage 4F.

## Atomic commits

| Work package | Commit | Result |
|---|---|---|
| E0 tracking authority | `3a1f703` | dependency order and delivery invariants |
| E1 document application | `0692ce6` | typed current-list, current-kind, set, and workspace reads |
| E2 document CLI alignment | `16e36b0` | document commands use the application facade |
| E3 Documents GUI | `98f4d50` | private-read accepted-set and structured document inspection |
| E4 review application | `7da311b` | typed current, template, workspace, and confirmation actions |
| E5 review CLI alignment | `4bfda70` | review commands use the application facade |
| E6 Review GUI | `5d0213e` | authority-aware dispositions with explicit rationale |
| E7 package application | `532b852` | readiness, export, reconciliation, and recovery actions |
| E7 package CLI alignment | `1cc5d23` | package commands use the application facade |
| E8 package GUI | `1c74d10` | consented export and explicit projection recovery |
| E9 render application | `a0b476a` | embedded build, current, and private export actions |
| E9 render CLI alignment | `6115187` | render commands use the application facade |
| E10 render GUI | `11c9d89` | validated PDF manifest and export workflow |
| E11 persistence qualification | `01397e7` | delivery reopen and export-recovery regression |

## Persistence and recovery qualification

The E11 regression builds one isolated local workspace from source and profile fixtures, confirms
evidence, criteria, match, and an Apply plan, and then completes every planned structured document
through the GUI worker and shared application facade. It creates one human-review finding,
explicitly accepts the risk with a rationale, and proves the resulting package is ready to export.

The same regression then proves:

- package and render exports fail before consent and create no destination;
- package projections are written only below `jobs/JOB_ID/`;
- reconciliation detects a bounded user edit without changing authoritative artifacts;
- copy-as-new preserves the edited bytes at a distinct unmanaged path and restores the managed
  projection;
- replace discards a second edit only after the explicit recovery request;
- the embedded renderer produces a current validated manifest and private PDF export;
- `submission_performed` remains false; and
- documents, accepted set, confirmed review, export receipt, current projections, and render
  manifest remain identical after reopening the workspace.

## Verification evidence

| Check | Result |
|---|---|
| focused worker/application delivery regression | passed |
| complete workspace tests | passed; GUI 42, CLI binary contract 18, all other suites green |
| workspace all-target Clippy | passed with warnings denied |
| formatter, parity JSON, UX delivery review, and diff check | passed |
| source release check | passed at `33 implemented`, `2 deferred-beta` |
| macOS release build | passed for Apple Silicon CLI and GUI |
| staged bundle verification | final-byte manifest, layout, version, and nested/outer ad-hoc signatures passed |
| hosted Fast CI | passed for the Stage 4E closure checkpoint |
| packaged macOS bilingual/accessibility smoke | English and Simplified Chinese semantics, exact Tab order, 200% text/focus visibility, reduced motion, and reset passed |

The local fixtures are bounded and synthetic. The staged app under `/private/tmp` is disposable
qualification output, not a release asset. Stage 4E does not create a tag, release,
package-manager update, or five-target native matrix.
