# ADR-011: Materialize Evidence Through Run-Scoped Job-Local Snapshots

**Status:** Accepted and implemented for the Task 4 Evidence/Match slice

**Date:** 2026-07-11

## Context

Stage 2 needs a durable Evidence catalog and Match stage. The normalized evidence currently lives under the workspace
profile directory, while `TaskSpecV1` deliberately treats every input and allowed read as relative to one selected
job directory. Pretending that `profile/generated/...` is job-relative, using parent traversal, or passing absolute
profile paths would make the task contract untruthful and weaken the path boundary already enforced by the runtime.

Adding a `read_scope=job|workspace` field to TaskSpec v1 would not be a harmless extension. TaskSpec is a strict,
versioned contract; existing validators, persisted run reconstruction, and path resolvers all interpret its paths as
job-relative. Old strict readers would reject the new field, and a workspace scope would substantially enlarge the
path and symlink attack surface.

Evidence matching also creates a privacy distinction that path safety alone cannot express. A matcher needs the
normalized evidence body, but workflow state, task receipts, command responses, and logs do not. The system therefore
needs a private data plane that may contain evidence text and a separate control plane that never does.

## Decision

The Stage 2 Evidence slice keeps TaskSpec v1 unchanged and materializes its profile inputs into a
run-scoped, immutable, job-local snapshot before an Evidence task is executed.

The flow is:

```text
workspace profile/generated evidence
  -> deterministic core materialization
  -> workflow/runs/<run-id>/inputs/evidence-snapshot.json
  -> Evidence candidate validation and promotion
  -> evidence_catalog.json
  -> Match reads criteria.json + evidence_catalog.json
  -> criterion_matches.json
```

The runtime must follow these rules:

1. Stage status computes the Evidence semantic fingerprint without writing a snapshot.
2. During prepare, after a run identity exists and before TaskSpec is finalized, the core service validates the
   configured generated-evidence inputs and writes one immutable snapshot under that run.
3. The Evidence TaskSpec names only the job-relative snapshot in `inputs` and `allowed_reads`. Match names only the
   current job-local `criteria.json` and `evidence_catalog.json`.
4. Submit, apply, terminal recovery, and the final pre-promotion claim recheck the immutable prepared input and the
   live Evidence semantic fingerprint. A profile or generated-evidence change after prepare makes the task stale and
   cannot promote its candidate.
5. Snapshot filenames and control records use generic paths, hashes, identifiers, sizes, reason codes, and counts.
   They never derive filenames or messages from private evidence text.
6. Generated-evidence discovery rejects unsafe or duplicate manifest keys, absolute generated-file paths, parent
   traversal, path escapes, symlinks, dangling symlinks, hard-link aliases, non-regular files, workspace-external
   profile roots, files over 4 MiB, and combined Evidence inputs over 16 MiB before accepting their bodies. On POSIX
   systems that support it, reads use descriptor-relative traversal with no-follow checks; the portable fallback
   performs bounded pre/post identity checks and final receipt revalidation.
7. Evidence and Match are deterministic-only in this slice. They do not invoke a configured provider, require a
   platform SDK, or depend on MCP, a hosted service, or a platform-specific API.
8. Typst-backed generated evidence carries `canisend-source-sha256` for its bound raw profile source. A missing receipt
   produces `evidence.source_receipt_missing`; a changed source produces `evidence.source_receipt_stale`. Both make
   the catalog unavailable until `extract-profile-evidence` regenerates it. The Evidence fingerprint and catalog also
   record current manifest, raw source, and generated-evidence receipts.

The private data plane consists of the run-scoped Evidence input snapshot, the Evidence candidate, and the promoted
`evidence_catalog.json`. These files live inside the ignored private job workspace and may contain normalized
evidence bodies because they are application data artifacts.

The control plane consists of AgentResponse JSON or text output, workflow state, TaskSpec, preparation and submission
receipts, TaskResult, validation reports, terminal claims, promotion receipts, run manifests, error messages, and
AgentResponse extensions. `criterion_matches.json` is a body-minimized semantic projection, but remains a Tier 2
job-strategy artifact for agent-reading consent. Its references are
opaque `evidence_catalog.json#items/<evidence-id>` catalog locators rather than profile paths, headings, item labels,
or evidence kinds. These records contain only safe paths, hashes, semantic IDs, classifications, privacy-safe reason
codes, and counts; they never copy or quote evidence bodies.

EvidenceRef identity remains content-derived as required by ADR-009. Evidence v1 normalizes Unicode and whitespace,
canonicalizes the evidence kind, and hashes only normalized kind plus normalized body. Legacy item labels such as
`cv-001`, paths, sections, job identity, and input order remain human-readable locators rather than identity, and the
legacy pipeline keeps its existing display citations. Semantic duplicates collapse to one stable ID and retain the
deterministically first display locator.

Evidence and Match use the existing single-file candidate, validation, terminal-claim, and atomic-promotion path.
Their accepted Task 4 slice covers prepare, guarded submit, apply, cancel, cache, output drift, stale prepared inputs,
dependency invalidation, terminal-action competition, recovery, and fresh-session reconstruction. Match always emits
`review_state=proposed`; its deterministic classification is not a user-owned application decision or a readiness
claim.

## Consequences

- Existing TaskSpec v1 producers, validators, historical runs, and shell-capable hosts keep the same path semantics.
- Codex, Claude Code, and IDE shell agents use the same CLI and do not need a platform adapter or second provider.
- Match receives a truthful, job-local read scope and cannot reach arbitrary workspace profile files.
- Immutable snapshots improve reproducibility and stale-result detection.
- Evidence text is duplicated inside the private job/run data plane. It remains there until the user removes the
  private run or job directory; control records must not imply that the text was erased automatically.
- A failed prepare may leave an unreferenced private snapshot; cleanup is not silently inferred from failure.
- Workspace-external profiles remain supported by legacy commands where already supported, but are not inputs to the
  resumable Evidence slice until a safe versioned external-artifact contract exists.
- A valid empty Evidence catalog and an unavailable catalog remain distinct. Malformed or unsafe inputs fail the
  stage; missing or stale Typst source receipts produce an unavailable catalog with a stable reason. Match maps empty
  and unavailable catalogs to distinct `unknown` gaps and cannot turn either into a claim that the applicant has no
  evidence.

## Rejected Alternatives

- Add `read_scope=job|workspace` to TaskSpec v1: rejected because it changes a strict public contract, historical path
  interpretation, and the runtime security boundary without version negotiation.
- Use `../profile` or absolute paths in TaskSpec: rejected because the declared scope would be unsafe and dishonest.
- Let Match read profile files directly while omitting them from TaskSpec: rejected because receipts and allowed reads
  would not describe the work performed.
- Put evidence bodies in AgentResponse extensions, manifests, or `criterion_matches.json`: rejected because control
  records and ordinary command output must remain privacy-safe.
- Store only locator metadata in the Evidence catalog while claiming host-executable matching: rejected for this
  slice because the declared Match input would not contain the content required to perform the match.

## Revisit When

Revisit this decision when TaskSpec v2 has explicit scope-qualified artifact references, migration and capability
negotiation, and negative tests for workspace and external roots. Also revisit if a retention policy forbids any
job-local duplication of evidence bodies or if a future secure content-addressed store replaces run-scoped snapshots.
