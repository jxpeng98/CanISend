# ADR-020: Project Reviewed Research Statements As Standalone Compatibility Views

**Status:** Accepted for Task 12 of Stage 3

**Date:** 2026-07-14

## Context

ADR-018 and ADR-019 give Research Statement its own guarded Draft, deterministic Review, user-owned finding
dispositions, and derived document readiness. The current compatibility pipeline still projects only Cover Letter to
Markdown and Typst. A reviewed Research Statement therefore has no portable, editable document view for CLI and
agent hosts, even though its authoritative Claim graph is complete.

The projection must not silently broaden application-package readiness. It must also avoid leaving an old generated
Research Statement visible after its Draft, Review, dispositions, Parsed Job, or stage receipts stop being current.

## Decision

`canisend run` may create a standalone Research Statement compatibility projection only when all of the following
remain current for the configured workspace profile and Parsed Job:

- the document-scoped Research Statement Draft stage and authoritative Draft;
- the deterministic, blocker-free Research Statement Review and exact Draft receipt;
- the user-owned Research Statement disposition artifact and mutation audit; and
- derived Research Statement readiness equal to `reviewed`.

The projection consists of `08_research_statement.md`, `typst/research_statement_content.json`, and
`typst/research_statement.typ`. It renders each validated Claim once, in Draft order. Markdown neutralizes
agent-controlled structure, and Typst places Claim text inside escaped string expressions. The JSON projection binds
the exact Draft, Review, disposition, readiness, and Markdown hashes and declares
`integration_scope=standalone_document`.

Research Statement content is not copied into `application_package_content.json` or
`application_package.typ`. Its files are not required by `check-package`, are not included in that gate's input hash
set, and cannot make APP-Q pass or fail. `render-typst` may compile the standalone source, and explicit
`--git-add-materials` may stage the Markdown and primary Typst source when they exist. Its Typst edit baseline uses a
separate `.canisend-research-generated.json` manifest so optional document changes cannot alter the package manifest
hash indirectly.

If a projection was previously generated but is no longer eligible, a later pipeline run replaces generated
Markdown, content JSON, and an unedited generated Typst primary with controlled body-free unavailable views. A
user-edited Typst primary is preserved and receives `research_statement.generated.typ` for reconciliation. That
optional candidate blocks Typst rendering but is deliberately outside the existing package gate.

## Consequences

- reviewed Research Statement prose becomes portable across CLI, Codex, Claude, and other shell-capable hosts;
- the authoritative Draft/Review/disposition files remain unchanged and continue to own document truth;
- stale generated Research prose is not silently retained when currentness or readiness is lost;
- edited Typst remains user-owned and requires an explicit reconciliation before rendering;
- Cover Letter and application-package behavior, required files, and readiness semantics remain compatible.

## Rejected Alternatives

- Always render after blocker-free Review: rejected because unresolved finding decisions are not document approval.
- Embed Research Statement in the application package: rejected because per-document readiness is not aggregate
  package readiness or a cross-document consistency result.
- Delete stale projection files: rejected because removal is destructive and cannot preserve an edited Typst source.
- Leave the previous generated projection untouched: rejected because stale private prose would continue to look
  current.
- Make an optional Research Typst candidate fail APP-Q: rejected because it would change the established Cover
  Letter package boundary without an aggregate package policy.

## Revisit When

Revisit before adding aggregate cross-document review, making Research Statement package-required, deriving package
readiness from multiple documents, or supporting more than one Research Statement requirement.
