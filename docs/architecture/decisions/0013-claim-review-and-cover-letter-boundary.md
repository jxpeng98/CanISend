# ADR-013: Keep Draft Claims Structured And Review Findings Separate

**Status:** Accepted as the Stage 3 foundation boundary

**Date:** 2026-07-13

## Context

Stage 2 can establish current Criteria, Evidence, Match, a confirmed apply Decision, an Application Brief, and a
required-document plan. The legacy pipeline can generate prose, but its Markdown does not provide a durable answer to
four questions needed before Draft or package readiness can become executable:

1. which exact applicant-facing statements are claims;
2. which current evidence and criteria support each claim;
3. which claims are unsupported, contradictory, or still require human review; and
4. whether a generated document is merely a candidate or a reviewed artifact that may participate in readiness.

Cover-letter prose and claim text are Tier 2 private application content. An agent or configured provider may help
create a candidate only after explicit approval to read the declared Tier 2 inputs. Allowing that worker to write
`03_cover_letter_draft.md`, an editable Typst source, or another authoritative application artifact directly would
bypass the guarded candidate-validation-promotion boundary established by the resumable runtime.

## Decision

Stage 3 starts with one Cover Letter vertical slice. The core owns a versioned structured Cover Letter Draft
artifact. Its complete applicant-facing body is represented as ordered sections whose every prose block is one
explicit `ClaimV1`; there is no untracked free-text field alongside the claim graph. Each claim carries a stable
content-derived identifier, a claim kind, proposed support strength, current Criterion/Evidence references where
applicable, and executable blocker codes.

`ReviewFindingV1` is a separate contract. A Draft worker may propose claim metadata, but it may not declare its own
candidate reviewed, ready, final, or submission-ready. The later deterministic Review stage owns the review-finding
collection and derives blockers from the current promoted Draft plus current upstream receipts.

### Draft ownership and promotion

- `cover_letter_draft.json` is a core-owned Tier 2 structured Draft artifact.
- A host agent writes candidate JSON only to fresh scratch, then submits it through `canisend stage submit`.
- The guarded runtime writes the declared candidate and TaskResult, validates the candidate against current inputs,
  and atomically promotes exactly one core-owned target.
- Draft never rewrites `application_brief.yaml`, `application_decision.yaml`, Criteria, Evidence, Match, the required
  document plan, compatibility Markdown, editable Typst, or profile sources.
- Markdown and Typst projection from a promoted structured Draft is a later compatibility task, not an alternate
  promotion path.

### Compatibility implementation note (2026-07-13)

The later compatibility task now projects a promoted Draft only after the structured Match, Draft, and deterministic
Review views are independently current and validated for the same parsed job and configured profile, with zero Review
blocker findings. The projection renders each Claim once, records exact Draft/Review hashes, protects edited Typst by
writing a generated candidate, and fails closed to the legacy/provider path when provenance cannot be established.
It does not promote Markdown or Typst, close open findings, change `review_state=proposed`, or establish readiness.

The first executable Draft mode is `host_agent`. A configured-provider mode may be added only after it reuses the
same TaskSpec, candidate, validator, privacy, and promotion contracts. Deterministic Draft is not claimed merely by
templating generic prose.

### Claim semantics

Every claim is one complete prose block and has exactly one kind:

- `factual`: a statement about the applicant's existing record or capability;
- `motivation`: a user-owned reason for applying, bound to the confirmed Brief motivation;
- `future_intent`: a clearly forward-looking proposed contribution, not a disguised statement of past fact;
- `role_context`: a statement about the advertised role or institution, bound to current job/criterion context;
- `administrative`: salutation, closing, or another non-substantive document element.

Factual claims may be `strong`, `partial`, or `unsupported`. Strong and partial factual claims must resolve to at
least one current Evidence item. Unsupported factual claims must have no evidence reference and must carry the
`claim.unsupported` blocker. A partial factual claim carries `claim.partial_support` until Review determines that the
wording is proportional. Non-factual claims use `not_applicable` support, do not cite applicant Evidence as proof,
and must carry the appropriate Brief/criterion basis reference required by their kind.

Criterion and Evidence IDs are semantic references, not citations copied from private profile headings or item
labels. The Draft basis binds the exact raw hashes of Parsed Job, Criteria, Evidence, Match, Decision, Brief, and the
required-document plan. Candidate validation resolves every reference against those current artifacts and rejects a
strong claim whose referenced evidence is absent or whose upstream basis changed.

Claim IDs are derived from the job ID, document ID, claim kind, and normalized claim text. They do not depend on list
position, section order, line number, provider identity, or evidence ordering. Changing substantive wording creates
a new claim ID; moving unchanged wording between sections does not.

### Review findings and readiness

Review findings are core-owned, regenerable Tier 2 records. Each finding has a stable ID, severity, category, reason
code, bounded message/next action, and optional Claim/Criterion/Evidence references. Unsupported or contradictory
facts are blockers. Partial/semantic support, every non-factual Claim-kind classification, cross-document
inconsistency, compliance, completeness, and style findings stay explicit rather than being hidden in prose comments.

A promoted Draft remains `review_state=proposed` and cannot report package readiness. Review, Package, Verify, and
Render remain distinct downstream stages. Only a later current Review artifact with no blocker findings may allow a
document to participate in package readiness, and a missing required document still blocks the package.

### Privacy and control plane

Draft candidates, promoted Drafts, claim text, Review candidates/findings, Brief bodies, Evidence catalogs, and
required-document plans are Tier 2. Agents ask before reading them. A provider-backed mode, if added, is Tier 3 and
requires separate provider approval.

TaskSpecs, workflow state, receipts, manifests, claims for promotion, errors, ordinary stdout, and AgentResponse are
the body-free control plane. They may contain only safe paths, hashes, IDs, modes, states, counts, blocker/reason
codes, and other bounded scalars. They never copy claim text, evidence bodies, Brief motivation, review messages, or
provider prompts/responses.

## Consequences

- Every applicant-facing Draft block becomes inspectable and traceable instead of hiding claims in Markdown prose.
- A Draft worker can propose wording but cannot self-certify support or readiness.
- A Draft worker cannot bypass review by relabeling a factual statement as a non-factual Claim kind.
- Rejected or stale candidates cannot alter promoted Drafts, user-owned YAML, Markdown, Typst, or profile inputs.
- Strong factual claims have a machine-enforceable current-evidence invariant; semantic proportionality remains a
  Review responsibility.
- The first slice can supply guarded compatibility views without making those views authoritative workflow state.
- Claim and Review bodies remain private even though their status and blocker counts can be exposed safely.

## Rejected Alternatives

- Treat every Markdown sentence as an implicit claim: rejected because parsing prose after generation is ambiguous
  and allows untracked statements.
- Let the Draft worker emit its own final review findings: rejected because generation and verification must be
  independent stages.
- Promote `03_cover_letter_draft.md` directly: rejected because it is already a compatibility/editing surface and
  cannot carry the strict claim graph safely.
- Require every prose statement to cite Evidence: rejected because motivation, future intent, role context, and
  administrative text are not claims about the applicant's existing record.
- Accept a strong claim merely because one Evidence ID exists: rejected as a readiness rule; the structural link is
  necessary for Draft validation but proportional semantic support must still pass Review.
- Add all document kinds in the first slice: rejected because Cover Letter provides the smallest complete vertical
  path for validating the contract and promotion boundary.

## Revisit When

Revisit after the Cover Letter Draft and Review slices are locally accepted, before adding provider-backed Draft,
cross-document waiver semantics, user-owned finding dispositions, multi-file promotion, or readiness across research,
teaching, diversity, supporting, publication, email, and interview artifacts.
