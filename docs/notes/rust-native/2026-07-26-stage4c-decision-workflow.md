# Stage 4C decision-workflow evidence

**Date:** 2026-07-26

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4C profile, evidence, criteria, match-inspection, and application-plan GUI parity

## Outcome

Stage 4C adds four GUI operation families through the same Rust application and store boundaries
used by the CLI:

- `profile.*`;
- `criteria.*`;
- `match.*`; and
- `plan.*`.

The Profile surface imports explicitly classified local sources and provides structured evidence
review. The selected-job surface provides structured criteria confirmation, read-only
revision-bound match inspection, and explicit application-plan decisions. Source spans, artifact
identity, match results, and derived blockers stay read-only where the store owns them.

Parity is now `26 implemented` and `9 deferred-beta`. Stage 4D next covers discovery, Agent v2 task
lifecycle, and agent asset-pack operations.

## Atomic commits

| Work package | Commit | Result |
|---|---|---|
| C1 application profile | `6f8de54` | typed source and evidence actions |
| C2 CLI profile alignment | `ec3ba20` | profile commands use the shared application facade |
| C3 GUI profile catalog | `7294135` | metadata-only import and sensitivity controls |
| C4 GUI evidence review | `04db8e6` | correction, exclusion, confirmation, and invalidation preview |
| C5 application criteria/match | `8e343de` | typed decision read/write models |
| C5 CLI criteria/match | `2732f09` | criteria and match commands use the application facade |
| C5–C6 GUI criteria/match | `87f0577` | structured criteria and read-only match surfaces |
| C7 application plan | `5dc66a8` | typed plan template/current/confirm actions |
| C7 CLI plan alignment | `325ec1a` | plan commands use the application facade |
| C7 GUI plan | `179bd1f` | explicit plan decision and confirmation |
| C8 qualification | `03de1f3` | full persistence regression and corrected native focus contract |

## Persistence qualification

The new GUI worker regression creates an isolated workspace and job, imports job and private
profile sources, completes bounded Agent v2 evidence and match task fixtures, confirms evidence and
criteria through GUI worker requests, inspects the current match, and confirms an Apply plan. It
then reopens the workspace and proves that the evidence, criteria, match, and plan identities and
revisions are unchanged. The final plan is also read through the shared application facade and
must equal the GUI result.

This combines with existing CLI binary-contract and Agent v2 task regressions rather than adding a
second GUI-only persistence representation.

## Local source verification

| Check | Result |
|---|---|
| affected application, store, CLI, and GUI tests | 89 passed; bounded network, package-fixture, and release-performance tests ignored by policy |
| decision-workflow reopen regression | 1 passed |
| affected all-target Clippy | passed with warnings denied |
| formatter, shell syntax, and diff check | passed |
| release check | passed; `26 implemented`, `9 deferred-beta` |
| release build | passed for the Apple Silicon CLI and GUI |
| staged bundle verification | final-byte manifest, layout, version, and nested/outer ad-hoc signatures passed |
| macOS accessibility smoke | English and Simplified Chinese semantics, exact Tab order, 200% focus visibility, reduced motion, and reset passed |

The first native smoke exposed a stale test expectation after Profile entered the navigation order.
The bounded contract now asserts `Overview → Jobs → Profile → Workspaces`; the complete smoke
passed after that correction.

The staged app under `/private/tmp` is disposable local qualification output, not a release asset.
No tag, package-manager update, five-target native matrix, or public release was created.
