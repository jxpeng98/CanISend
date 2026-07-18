# R8.2 structured draft task note

## Outcome

R8.2 makes Draft an available workflow stage. The current application plan is projected into ordered planned
documents, and each non-omitted entry is completed through one leased `canisend.document-candidate/v2` task. The
supported first-alpha order comes from the user-confirmed plan; CanISend permits only the next missing entry, which
prevents parallel tasks from racing to assemble different document sets.

The CLI operations are:

- `cover-letter-draft`;
- `cv-draft`;
- `research-statement-draft`;
- `teaching-statement-draft`.

`workflow status` selects the next operation and its planned `host-agent` or `configured-provider` mode. Older R7
workspaces whose plan predates the ordered projection are asked to re-export and reconfirm the plan once.

## Validation boundary

Every task declares exactly five current private artifacts: ApplicationPlan, EvidenceMatches, Criteria,
EvidenceCatalog, and ParsedJob. Completion rechecks every revision/hash inside the commit transaction and verifies
the dependency chain between them. The candidate must repeat the exact job, plan, planned-document, and document
kind.

Applicant facts may cite only exact confirmed, non-excluded evidence revisions that occur in the current match set.
Job requirements may cite only exact current criterion revisions. User-intent and non-factual claims cannot carry
citations. The core rejects invented nested IDs and assigns document, section, claim, placeholder, generation, and
revision metadata only after structural and semantic validation.

Both executors use this same boundary. Configured-provider mode adds `send-to-configured-provider` consent on top of
the private-read consent. Existing input, blob, task-completion, section-body, claim, citation, and placeholder limits
bound the provider request and response.

## Persistence and invalidation

SQLite migration 9 adds the ordered `application_plan_documents` projection and current `document_heads`. Accepted
documents remain immutable artifacts. After each document, Draft returns to ready if another non-omitted entry is
missing. The final document atomically creates a `document-set` artifact with dependencies on the exact plan and all
current document revisions, completes Draft, and makes Review ready.

Plan revisions, evidence revisions, profile imports, job revisions, and reruns that reach Draft mark individual
document artifacts stale and clear their current heads. The set is also stale through its stage output/dependencies.
`document list`, `document show`, and `document set` therefore expose only coherent current state.

## Agent assets

Draft is now reported as available and `document.lifecycle` is an available capability. The Codex, Claude, and
generic packs include `prompt.document-draft`, the candidate/document/set schemas, and host instructions for the
sequential task loop. Advert, evidence, criteria, match, and plan bodies remain untrusted inputs.

## Verification

The R8.2 quality gate passed:

- 64 Rust tests across the workspace;
- Clippy for all targets/features with warnings denied;
- 31 generated public-schema checks;
- 40 embedded-resource checks and 21-file host-pack verification;
- release compilation;
- packaged host-agent smoke.

The integration path covers both executor modes, consent counts, rejection of invented IDs and unknown evidence,
exact generation metadata, all four document shapes, final set assembly, Review unlock, and profile-change
invalidation.

## Next boundary

R8.3 must consume the immutable document set and produce revision-bound ReviewFindings. It should independently
detect unsupported prose, prohibited-claim conflicts, unresolved placeholders, and cross-document inconsistencies;
then separate deterministic blockers from human review dispositions without mutating accepted drafts in place.
